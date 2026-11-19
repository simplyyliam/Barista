//! Reads the classic menu bar of another program's window (`GetMenu`) and runs one of its commands
//! (ADR 0036). Menu handles are shared system objects, so the text, check marks and states can be read
//! from another process without touching the app. Everything read here is untrusted text from another
//! program: the caller caps it again before drawing.

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    GetMenu, GetMenuItemCount, GetMenuItemInfoW, IsWindow, PostMessageW, SendMessageTimeoutW, HMENU, MENUITEMINFOW,
    MFS_CHECKED, MFS_DISABLED, MFS_GRAYED, MFT_OWNERDRAW, MFT_SEPARATOR, MIIM_FTYPE, MIIM_ID, MIIM_STATE, MIIM_STRING,
    MIIM_SUBMENU, SMTO_ABORTIFHUNG, SMTO_BLOCK, WM_COMMAND, WM_INITMENUPOPUP,
};

/// Most items read from one window (a menu is a list, not a database).
pub const MAX_ITEMS: usize = 300;
const MAX_DEPTH: usize = 3;
const MAX_TOP: i32 = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Normal,
    Separator,
    /// Has `children`.
    Submenu,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawItem {
    pub kind: Kind,
    /// Without the `&` mnemonic marks and without the accelerator text.
    pub label: String,
    /// The text after the tab (`Ctrl+S`).
    pub accelerator: Option<String>,
    /// The command id to post (`WM_COMMAND`). 0 for separators and submenus.
    pub command: u32,
    pub checked: bool,
    pub enabled: bool,
    pub children: Vec<RawItem>,
}

/// `&File` becomes `File`, `Save &As` becomes `Save As`, `A&&B` becomes `A&B`.
pub fn strip_mnemonic(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            if chars.peek() == Some(&'&') {
                out.push('&');
                chars.next();
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// Splits `Save\tCtrl+S` into the label and the accelerator.
pub fn split_accelerator(text: &str) -> (String, Option<String>) {
    match text.split_once('\t') {
        Some((label, accel)) => (strip_mnemonic(label), Some(accel.trim().to_string()).filter(|a| !a.is_empty())),
        None => (strip_mnemonic(text), None),
    }
}

/// One item's info. `None` if Windows won't tell (the menu changed under us).
fn item_info(menu: HMENU, position: u32) -> Option<(MENUITEMINFOW, String)> {
    let mut buffer = vec![0u16; 256];
    let mut info = MENUITEMINFOW {
        cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
        fMask: MIIM_FTYPE | MIIM_ID | MIIM_STATE | MIIM_STRING | MIIM_SUBMENU,
        dwTypeData: windows::core::PWSTR(buffer.as_mut_ptr()),
        cch: (buffer.len() - 1) as u32,
        ..Default::default()
    };
    // SAFETY: `info` points at `buffer`, which outlives the call; `cch` leaves room for the NUL.
    unsafe { GetMenuItemInfoW(menu, position, true, &mut info).ok()? };
    let len = (info.cch as usize).min(buffer.len() - 1);
    Some((info, String::from_utf16_lossy(&buffer[..len])))
}

/// Reads a menu (and its submenus) into plain data. `before_submenu` is called with each submenu
/// handle just before it is read: for a real window it sends `WM_INITMENUPOPUP`, which is what
/// the app does itself when its menu opens, so its enabled and checked states are current.
pub fn read_menu(
    menu: HMENU,
    depth: usize,
    budget: &mut usize,
    before_submenu: &mut dyn FnMut(HMENU, i32),
) -> Vec<RawItem> {
    // SAFETY: plain call on a handle; a bad handle gives -1.
    let count = unsafe { GetMenuItemCount(Some(menu)) };
    let mut out = Vec::new();
    for position in 0..count.max(0) {
        if *budget == 0 {
            break;
        }
        let Some((info, text)) = item_info(menu, position as u32) else { continue };
        *budget -= 1;
        if info.fType.0 & MFT_SEPARATOR.0 != 0 {
            out.push(RawItem { kind: Kind::Separator, label: String::new(), accelerator: None, command: 0, checked: false, enabled: true, children: vec![] });
            continue;
        }
        // Owner-drawn items carry no text we can read.
        if info.fType.0 & MFT_OWNERDRAW.0 != 0 && text.is_empty() {
            continue;
        }
        let (label, accelerator) = split_accelerator(&text);
        let enabled = info.fState.0 & (MFS_DISABLED.0 | MFS_GRAYED.0) == 0;
        let checked = info.fState.0 & MFS_CHECKED.0 != 0;
        if info.hSubMenu.0.is_null() {
            out.push(RawItem { kind: Kind::Normal, label, accelerator, command: info.wID, checked, enabled, children: vec![] });
        } else {
            before_submenu(info.hSubMenu, position);
            let children = if depth < MAX_DEPTH { read_menu(info.hSubMenu, depth + 1, budget, before_submenu) } else { vec![] };
            out.push(RawItem { kind: Kind::Submenu, label, accelerator: None, command: 0, checked: false, enabled, children });
        }
    }
    out
}

/// The window's menu bar, or `None` if it has none. Each top-level title comes with its items.
/// A window that does not answer `WM_INITMENUPOPUP` in time is not asked again.
pub fn read_menu_bar(hwnd: isize) -> Option<Vec<RawItem>> {
    let window = HWND(hwnd as *mut _);
    // SAFETY: plain calls; a dead window gives a null menu.
    let menu = unsafe { GetMenu(window) };
    if menu.0.is_null() {
        return None;
    }
    let top = unsafe { GetMenuItemCount(Some(menu)) };
    if top <= 0 || top > MAX_TOP {
        return None;
    }
    let mut asked_in_vain = false;
    let mut budget = MAX_ITEMS;
    let mut ask = |submenu: HMENU, position: i32| {
        if asked_in_vain {
            return;
        }
        let mut result = 0usize;
        // SAFETY: a short, abortable send: a hung app costs 150 ms once, not a freeze.
        let sent = unsafe {
            SendMessageTimeoutW(
                window,
                WM_INITMENUPOPUP,
                WPARAM(submenu.0 as usize),
                LPARAM((position & 0xffff) as isize),
                SMTO_ABORTIFHUNG | SMTO_BLOCK,
                150,
                Some(&mut result),
            )
        };
        if sent.0 == 0 {
            asked_in_vain = true;
        }
    };
    Some(read_menu(menu, 0, &mut budget, &mut ask))
}

/// Does this window still exist?
pub fn is_window(hwnd: isize) -> bool {
    // SAFETY: plain call.
    unsafe { IsWindow(Some(HWND(hwnd as *mut _))).as_bool() }
}

/// Runs a menu command of the window, as if the user picked it in the app's own menu. Posted, not
/// sent: the bar never waits for the app.
pub fn post_command(hwnd: isize, command: u32) -> bool {
    // SAFETY: plain call. `WM_COMMAND` with the id in the low word and 0 (menu) in the high word.
    unsafe { PostMessageW(Some(HWND(hwnd as *mut _)), WM_COMMAND, WPARAM((command & 0xffff) as usize), LPARAM(0)).is_ok() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::HSTRING;
    use windows::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CheckMenuItem, CreateMenu, CreatePopupMenu, DestroyMenu, MF_BYCOMMAND, MF_CHECKED, MF_GRAYED, MF_POPUP,
        MF_SEPARATOR, MF_STRING, MF_UNCHECKED,
    };

    fn read(menu: HMENU) -> Vec<RawItem> {
        let mut budget = MAX_ITEMS;
        read_menu(menu, 0, &mut budget, &mut |_, _| {})
    }

    /// File: New (Ctrl+N), Open, ---, Exit (disabled), Edit: Word wrap (checked), View: Zoom > In, Out
    fn sample() -> HMENU {
        // SAFETY: creates and fills menus owned by this test; destroyed by the caller.
        unsafe {
            let bar = CreateMenu().unwrap();
            let file = CreatePopupMenu().unwrap();
            AppendMenuW(file, MF_STRING, 101, &HSTRING::from("&New\tCtrl+N")).unwrap();
            AppendMenuW(file, MF_STRING, 102, &HSTRING::from("&Open...")).unwrap();
            AppendMenuW(file, MF_SEPARATOR, 0, None).unwrap();
            AppendMenuW(file, MF_STRING | MF_GRAYED, 103, &HSTRING::from("E&xit")).unwrap();
            AppendMenuW(bar, MF_POPUP, file.0 as usize, &HSTRING::from("&File")).unwrap();
            let edit = CreatePopupMenu().unwrap();
            AppendMenuW(edit, MF_STRING | MF_CHECKED, 201, &HSTRING::from("Word wrap")).unwrap();
            AppendMenuW(bar, MF_POPUP, edit.0 as usize, &HSTRING::from("&Edit")).unwrap();
            let view = CreatePopupMenu().unwrap();
            let zoom = CreatePopupMenu().unwrap();
            AppendMenuW(zoom, MF_STRING, 301, &HSTRING::from("Zoom &In")).unwrap();
            AppendMenuW(zoom, MF_STRING, 302, &HSTRING::from("Zoom &Out")).unwrap();
            AppendMenuW(view, MF_POPUP, zoom.0 as usize, &HSTRING::from("&Zoom")).unwrap();
            AppendMenuW(bar, MF_POPUP, view.0 as usize, &HSTRING::from("&View")).unwrap();
            bar
        }
    }

    #[test]
    fn mnemonics_and_accelerators_are_split_off() {
        assert_eq!(strip_mnemonic("&File"), "File");
        assert_eq!(strip_mnemonic("Save &As..."), "Save As...");
        assert_eq!(strip_mnemonic("Fish && Chips"), "Fish & Chips");
        assert_eq!(split_accelerator("&New\tCtrl+N"), ("New".into(), Some("Ctrl+N".into())));
        assert_eq!(split_accelerator("Plain"), ("Plain".into(), None));
        assert_eq!(split_accelerator("Odd\t"), ("Odd".into(), None));
    }

    #[test]
    fn a_menu_bar_is_read_with_titles_items_and_states() {
        let bar = sample();
        let items = read(bar);
        let titles: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(titles, vec!["File", "Edit", "View"]);
        let file = &items[0].children;
        assert_eq!(file.len(), 4);
        assert_eq!((file[0].label.as_str(), file[0].accelerator.as_deref(), file[0].command), ("New", Some("Ctrl+N"), 101));
        assert_eq!(file[2].kind, Kind::Separator);
        assert!(!file[3].enabled, "the grayed Exit is disabled");
        assert!(file[0].enabled);
        assert!(items[1].children[0].checked, "Word wrap is checked");
        // A submenu inside a submenu.
        let zoom = &items[2].children[0];
        assert_eq!(zoom.kind, Kind::Submenu);
        assert_eq!(zoom.children.iter().map(|c| c.command).collect::<Vec<_>>(), vec![301, 302]);
        // SAFETY: destroys the test menu (and its popups).
        unsafe { DestroyMenu(bar).unwrap() };
    }

    #[test]
    fn the_item_budget_caps_a_huge_menu() {
        // SAFETY: test-owned menu.
        let bar = unsafe {
            let bar = CreatePopupMenu().unwrap();
            for i in 0..500u32 {
                AppendMenuW(bar, MF_STRING, 1000 + i as usize, &HSTRING::from(format!("Item {i}"))).unwrap();
            }
            bar
        };
        let mut budget = 50;
        assert_eq!(read_menu(bar, 0, &mut budget, &mut |_, _| {}).len(), 50);
        unsafe { DestroyMenu(bar).unwrap() };
    }

    #[test]
    fn check_state_changes_are_seen_on_the_next_read() {
        let bar = sample();
        // SAFETY: test-owned menu handles.
        unsafe {
            let edit = windows::Win32::UI::WindowsAndMessaging::GetSubMenu(bar, 1);
            CheckMenuItem(edit, 201, MF_UNCHECKED.0 | MF_BYCOMMAND.0);
        }
        assert!(!read(bar)[1].children[0].checked);
        unsafe { DestroyMenu(bar).unwrap() };
    }

    #[test]
    fn a_window_without_a_menu_reads_as_none() {
        assert!(read_menu_bar(0).is_none());
        assert!(!is_window(0));
    }
}
