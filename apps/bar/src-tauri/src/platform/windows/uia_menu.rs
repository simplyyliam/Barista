//! Reads and runs the menu bar of modern apps through UI Automation (ADR 0036). These apps have no
//! classic `HMENU`. Their `MenuBar` gives the titles for free, but the items of a menu only exist while
//! it is expanded (the app draws its own popup for about 200 ms), so a menu is expanded, read and
//! collapsed again, and a pick expands the same path and invokes the item.
//!
//! Every UIA call runs on one worker thread, with a timeout on the caller's side. UIA calls into the
//! other app and can block for as long as that app is hung: the bar's threads never wait for it
//! (CLAUDE.md rule 3).
//!
//! Found while building this: a minimized window shows no popup, so its menu items do not exist for UIA
//! (nothing to read). Apps in front are never minimized, so this only matters for tests. A worker that does not answer is abandoned and a new one is made.

use super::win32_menu::{Kind, RawItem, MAX_ITEMS};
use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use windows::core::Interface;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::{
    CUIAutomation8, IUIAutomation, IUIAutomationElement, IUIAutomationExpandCollapsePattern, IUIAutomationInvokePattern,
    IUIAutomationTogglePattern, ToggleState_On, TreeScope_Children, TreeScope_Descendants, UIA_ControlTypePropertyId,
    UIA_ExpandCollapsePatternId, UIA_InvokePatternId, UIA_MenuBarControlTypeId, UIA_MenuItemControlTypeId,
    UIA_TogglePatternId,
};

/// A menu read takes at most this long in total, however many submenus there are.
const READ_BUDGET: Duration = Duration::from_millis(2500);
/// How long to wait for a popup to show its items after `Expand`.
const POPUP_WAIT: Duration = Duration::from_millis(600);
const MAX_DEPTH: usize = 2;
const MAX_TITLES: i32 = 24;

type Task = Box<dyn FnOnce(&IUIAutomation) + Send>;

/// The worker and its generation number: a timeout only drops the worker it timed out on, not a newer one.
static WORKER: Mutex<Option<(u64, Sender<Task>)>> = Mutex::new(None);
static GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn start_worker() -> Sender<Task> {
    let (tx, rx) = channel::<Task>();
    let spawned = std::thread::Builder::new().name("uia-menus".into()).spawn(move || {
        // SAFETY: COM is initialised on this thread before anything uses it, and only this thread does.
        let automation: Option<IUIAutomation> = unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER).ok()
        };
        let Some(automation) = automation else { return };
        for task in rx {
            task(&automation);
        }
    });
    if spawned.is_err() {
        eprintln!("barista: could not start the UI Automation worker");
    }
    tx
}

/// Runs `job` on the worker and waits up to `timeout`. `None` if it did not finish: the worker is
/// then dropped (it may be stuck in a hung app) and the next call starts a fresh one.
fn run<T: Send + 'static>(timeout: Duration, job: impl FnOnce(&IUIAutomation) -> T + Send + 'static) -> Option<T> {
    let (reply, wait) = channel();
    let (generation, sender) = WORKER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_or_insert_with(|| (GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst), start_worker()))
        .clone();
    let drop_this_worker = move || {
        let mut worker = WORKER.lock().unwrap_or_else(|e| e.into_inner());
        if worker.as_ref().is_some_and(|(g, _)| *g == generation) {
            *worker = None;
        }
    };
    let task: Task = Box::new(move |automation| {
        let _ = reply.send(job(automation));
    });
    if sender.send(task).is_err() {
        drop_this_worker();
        return None;
    }
    match wait.recv_timeout(timeout) {
        Ok(value) => Some(value),
        Err(_) => {
            drop_this_worker();
            None
        }
    }
}

fn control_type_condition(
    automation: &IUIAutomation,
    id: windows::Win32::UI::Accessibility::UIA_CONTROLTYPE_ID,
) -> Option<windows::Win32::UI::Accessibility::IUIAutomationCondition> {
    // SAFETY: plain COM calls on objects of this thread.
    unsafe { automation.CreatePropertyCondition(UIA_ControlTypePropertyId, &VARIANT::from(id.0)).ok() }
}

/// Windows exposes the title bar's system menu (Restore, Move, Close) as a `MenuBar` with one item,
/// "System", on every window. That is not the app's menu.
fn is_system_bar(automation: &IUIAutomation, bar: &IUIAutomationElement) -> bool {
    // SAFETY: plain COM calls.
    let id = unsafe { bar.CurrentAutomationId().map(|b| b.to_string()).unwrap_or_default() };
    if id == "SystemMenuBar" {
        return true;
    }
    let titles = bar_titles(bar, automation);
    titles.len() == 1 && unsafe { titles[0].CurrentAutomationId().map(|b| b.to_string()).unwrap_or_default() } == "System"
}

/// The window's own menu bar element (not the title bar's system menu), if the app exposes one.
fn find_bar(automation: &IUIAutomation, hwnd: isize) -> Option<(IUIAutomationElement, IUIAutomationElement)> {
    let cond = control_type_condition(automation, UIA_MenuBarControlTypeId)?;
    // SAFETY: plain COM calls.
    unsafe {
        let window = automation.ElementFromHandle(HWND(hwnd as *mut _)).ok()?;
        let bars = window.FindAll(TreeScope_Descendants, &cond).ok()?;
        for i in 0..bars.Length().unwrap_or(0).min(8) {
            let Ok(bar) = bars.GetElement(i) else { continue };
            if !is_system_bar(automation, &bar) && !bar_titles(&bar, automation).is_empty() {
                return Some((window, bar));
            }
        }
        None
    }
}

fn name_of(element: &IUIAutomationElement) -> String {
    // SAFETY: plain COM call.
    unsafe { element.CurrentName().map(|b| b.to_string()).unwrap_or_default() }
}

fn bar_titles(bar: &IUIAutomationElement, automation: &IUIAutomation) -> Vec<IUIAutomationElement> {
    // SAFETY: plain COM calls.
    unsafe {
        let Ok(all) = automation.CreateTrueCondition().and_then(|t| bar.FindAll(TreeScope_Children, &t)) else { return vec![] };
        let n = all.Length().unwrap_or(0).min(MAX_TITLES * 2);
        // Unnamed children (an overflow button, an icon) are not titles. `titles`, `read_title` and `invoke`
        // all use this one list, so an index means the same title everywhere.
        (0..n)
            .filter_map(|i| all.GetElement(i).ok())
            .filter(|e| name_of(e).chars().any(|c| !c.is_control() && !c.is_whitespace()))
            .take(MAX_TITLES as usize)
            .collect()
    }
}

/// The titles of the window's menu bar, or `None` if it has none.
pub fn titles(hwnd: isize) -> Option<Vec<String>> {
    run(Duration::from_millis(1200), move |automation| {
        let (_, bar) = find_bar(automation, hwnd)?;
        let names: Vec<String> = bar_titles(&bar, automation).iter().map(name_of).collect();
        (!names.is_empty()).then_some(names)
    })
    .flatten()
}

fn expand_pattern(element: &IUIAutomationElement) -> Option<IUIAutomationExpandCollapsePattern> {
    // SAFETY: plain COM call; the cast checks the interface.
    unsafe { element.GetCurrentPattern(UIA_ExpandCollapsePatternId).ok()?.cast().ok() }
}

/// Every menu item currently on screen under the window (the bar's own titles too).
fn visible_items(automation: &IUIAutomation, window: &IUIAutomationElement) -> Vec<IUIAutomationElement> {
    let Some(cond) = control_type_condition(automation, UIA_MenuItemControlTypeId) else { return vec![] };
    // SAFETY: plain COM calls.
    unsafe {
        let Ok(all) = window.FindAll(TreeScope_Descendants, &cond) else { return vec![] };
        let n = all.Length().unwrap_or(0).min(MAX_ITEMS as i32 * 2);
        (0..n).filter_map(|i| all.GetElement(i).ok()).collect()
    }
}

fn same(automation: &IUIAutomation, a: &IUIAutomationElement, b: &IUIAutomationElement) -> bool {
    // SAFETY: plain COM call.
    unsafe { automation.CompareElements(a, b).map(|r| r.as_bool()).unwrap_or(false) }
}

/// Expands `item` and returns the items that appeared (and were not in `known`), waiting for the popup.
fn expand_and_collect(
    automation: &IUIAutomation,
    window: &IUIAutomationElement,
    item: &IUIAutomationElement,
    known: &[IUIAutomationElement],
) -> Vec<IUIAutomationElement> {
    let Some(pattern) = expand_pattern(item) else { return vec![] };
    // SAFETY: plain COM call.
    if unsafe { pattern.Expand() }.is_err() {
        return vec![];
    }
    let started = Instant::now();
    loop {
        let fresh: Vec<IUIAutomationElement> =
            visible_items(automation, window).into_iter().filter(|e| !known.iter().any(|k| same(automation, k, e))).collect();
        if !fresh.is_empty() || started.elapsed() > POPUP_WAIT {
            return fresh;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn collapse(item: &IUIAutomationElement) {
    if let Some(pattern) = expand_pattern(item) {
        // SAFETY: plain COM call; failing to collapse just leaves the app's popup up until it is dismissed.
        let _ = unsafe { pattern.Collapse() };
    }
}

/// Makes sure the app's own popup is really gone before the bar opens its menu. If it lingers, the app's
/// menu mode steals the focus from the bar's menu window, which then closes right after opening.
fn settle_closed(
    automation: &IUIAutomation,
    window: &IUIAutomationElement,
    title: &IUIAutomationElement,
    baseline: &[IUIAutomationElement],
) {
    let started = Instant::now();
    loop {
        let left_over = visible_items(automation, window).into_iter().any(|e| !baseline.iter().any(|k| same(automation, k, &e)));
        if !left_over || started.elapsed() > Duration::from_millis(700) {
            return;
        }
        collapse(title);
        std::thread::sleep(Duration::from_millis(40));
    }
}

fn raw_of(element: &IUIAutomationElement, submenu: bool) -> RawItem {
    // SAFETY: plain COM calls.
    unsafe {
        let enabled = element.CurrentIsEnabled().map(|b| b.as_bool()).unwrap_or(true);
        let accelerator = element.CurrentAcceleratorKey().map(|b| b.to_string()).ok().filter(|a| !a.is_empty());
        let checked = element
            .GetCurrentPattern(UIA_TogglePatternId)
            .ok()
            .and_then(|p| p.cast::<IUIAutomationTogglePattern>().ok())
            .and_then(|t| t.CurrentToggleState().ok())
            .is_some_and(|s| s == ToggleState_On);
        RawItem {
            kind: if submenu { Kind::Submenu } else { Kind::Normal },
            label: name_of(element),
            accelerator,
            command: 0,
            checked,
            enabled,
            children: vec![],
        }
    }
}

/// Reads the items of an expanded menu element, descending into submenus up to `MAX_DEPTH`.
fn read_level(
    automation: &IUIAutomation,
    window: &IUIAutomationElement,
    parent: &IUIAutomationElement,
    known: &mut Vec<IUIAutomationElement>,
    depth: usize,
    deadline: Instant,
) -> Vec<RawItem> {
    let items = expand_and_collect(automation, window, parent, known);
    known.extend(items.iter().cloned());
    let mut out = Vec::new();
    for (position, element) in items.iter().enumerate() {
        if out.len() >= MAX_ITEMS {
            break;
        }
        let is_submenu = expand_pattern(element).is_some();
        let mut raw = raw_of(element, is_submenu);
        if raw.label.is_empty() {
            continue;
        }
        // `command` carries the item's position in the popup, which `invoke` uses to find it again.
        raw.command = position as u32;
        if is_submenu && depth < MAX_DEPTH && Instant::now() < deadline {
            raw.children = read_level(automation, window, element, known, depth + 1, deadline);
        }
        out.push(raw);
    }
    collapse(parent);
    out
}

/// Expands the title at `index`, reads its items, and collapses it again.
pub fn read_title(hwnd: isize, index: usize) -> Option<Vec<RawItem>> {
    run(READ_BUDGET + Duration::from_millis(800), move |automation| {
        let (window, bar) = find_bar(automation, hwnd)?;
        let titles = bar_titles(&bar, automation);
        let title = titles.get(index)?;
        let _veil = super::veil::Veil::start(super::app_info::pid_of(hwnd));
        let baseline = visible_items(automation, &window);
        let mut known = baseline.clone();
        let items = read_level(automation, &window, title, &mut known, 0, Instant::now() + READ_BUDGET);
        settle_closed(automation, &window, title, &baseline);
        Some(items)
    })
    .flatten()
}

/// Replays `path` (the index of each choice, starting with the title) and invokes the last item.
/// `names` is what was on screen when the menu was read: an item whose name changed is not clicked.
pub fn invoke(hwnd: isize, path: Vec<usize>, names: Vec<String>) -> bool {
    run(READ_BUDGET + Duration::from_millis(800), move |automation| {
        let (window, bar) = find_bar(automation, hwnd)?;
        let titles = bar_titles(&bar, automation);
        let mut current = titles.get(*path.first()?)?.clone();
        if name_of(&current) != *names.first()? {
            return None;
        }
        let _veil = super::veil::Veil::start(super::app_info::pid_of(hwnd));
        let mut known = visible_items(automation, &window);
        for (depth, (&choice, expected)) in path.iter().zip(names.iter()).enumerate().skip(1) {
            let items = expand_and_collect(automation, &window, &current, &known);
            known.extend(items.iter().cloned());
            let picked = items.get(choice).filter(|e| name_of(e) == *expected)?.clone();
            if depth == path.len() - 1 {
                // SAFETY: plain COM calls on the element we just found.
                unsafe {
                    let pattern = picked.GetCurrentPattern(UIA_InvokePatternId).ok()?.cast::<IUIAutomationInvokePattern>().ok()?;
                    pattern.Invoke().ok()?;
                }
                return Some(());
            }
            current = picked;
        }
        None
    })
    .flatten()
    .is_some()
}

#[cfg(test)]
mod live {
    use super::*;
    use windows::core::w;
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

    /// Needs a running Windows 11 Notepad. `cargo test -p barista --lib uia_menu::live -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn read_notepads_menus_in_a_loop() {
        // SAFETY: plain lookup by class name.
        let hwnd = unsafe { FindWindowW(w!("Notepad"), None) }.unwrap().0 as isize;
        for _ in 0..25 {
            assert!(read_title(hwnd, 1).is_some_and(|v| !v.is_empty()));
        }
    }

    #[test]
    #[ignore]
    fn read_notepads_menus() {
        // SAFETY: plain lookup by class name.
        let hwnd = unsafe { FindWindowW(w!("Notepad"), None) }.unwrap().0 as isize;
        eprintln!("hwnd {hwnd}");
        eprintln!("titles: {:?}", titles(hwnd));
        let t = std::time::Instant::now();
        let edit = read_title(hwnd, 1);
        eprintln!("Edit read in {:?}: {:?}", t.elapsed(), edit.as_ref().map(|v| v.iter().map(|i| (i.label.clone(), i.command, i.accelerator.clone())).collect::<Vec<_>>()));
        assert!(edit.is_some_and(|v| !v.is_empty()));
    }
}
