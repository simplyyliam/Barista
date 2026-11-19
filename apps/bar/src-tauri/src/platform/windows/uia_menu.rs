//! Reads and runs the menu bar of modern apps through UI Automation (ADR 0036). These apps have no
//! classic `HMENU`. Their `MenuBar` gives the titles for free, but the items of a menu only exist while
//! it is expanded (the app draws its own popup for about 200 ms), so a menu is expanded, read and
//! collapsed again, and a pick expands the same path and invokes the item.
//!
//! Every UIA call runs on one worker thread, with a timeout on the caller's side. UIA calls into the
//! other app and can block for as long as that app is hung: the bar's threads never wait for it
//! (CLAUDE.md rule 3). A worker that does not answer is abandoned and a new one is made.

use super::win32_menu::{Kind, RawItem, MAX_ITEMS};
use std::sync::mpsc::{channel, Sender};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use windows::core::Interface;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationExpandCollapsePattern, IUIAutomationInvokePattern,
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

static WORKER: Mutex<Option<Sender<Task>>> = Mutex::new(None);

fn start_worker() -> Sender<Task> {
    let (tx, rx) = channel::<Task>();
    let spawned = std::thread::Builder::new().name("uia-menus".into()).spawn(move || {
        // SAFETY: COM is initialised on this thread before anything uses it, and only this thread does.
        let automation: Option<IUIAutomation> = unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()
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
    let sender = WORKER.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(start_worker).clone();
    let task: Task = Box::new(move |automation| {
        let _ = reply.send(job(automation));
    });
    if sender.send(task).is_err() {
        *WORKER.lock().unwrap_or_else(|e| e.into_inner()) = None;
        return None;
    }
    match wait.recv_timeout(timeout) {
        Ok(value) => Some(value),
        Err(_) => {
            *WORKER.lock().unwrap_or_else(|e| e.into_inner()) = None;
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

/// The window's menu bar element, if the app exposes one.
fn find_bar(automation: &IUIAutomation, hwnd: isize) -> Option<(IUIAutomationElement, IUIAutomationElement)> {
    let cond = control_type_condition(automation, UIA_MenuBarControlTypeId)?;
    // SAFETY: plain COM calls.
    unsafe {
        let window = automation.ElementFromHandle(HWND(hwnd as *mut _)).ok()?;
        let bar = window.FindFirst(TreeScope_Descendants, &cond).ok()?;
        Some((window, bar))
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
        let n = all.Length().unwrap_or(0).min(MAX_TITLES);
        (0..n).filter_map(|i| all.GetElement(i).ok()).collect()
    }
}

/// The titles of the window's menu bar, or `None` if it has none.
pub fn titles(hwnd: isize) -> Option<Vec<String>> {
    run(Duration::from_millis(1200), move |automation| {
        let (_, bar) = find_bar(automation, hwnd)?;
        let names: Vec<String> = bar_titles(&bar, automation).iter().map(name_of).filter(|n| !n.is_empty()).collect();
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
        let mut known = visible_items(automation, &window);
        Some(read_level(automation, &window, title, &mut known, 0, Instant::now() + READ_BUDGET))
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
