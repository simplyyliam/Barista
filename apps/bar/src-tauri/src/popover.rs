//! Popovers: an app's own window, shown under its bar icon (ADR 0014). The bar never draws
//! anything for it. It decides WHEN (only after a user click), WHERE (the final placed rect, in
//! physical pixels) and when to take it away again; the app only moves, shows and hides its window.
//!
//! ```text
//!  left click ─► route_click ─► item_click ─► Machine (toggle / swap, same rules as menus)
//!     show: close the menu, place the rect, AllowSetForegroundWindow(pid), send popover.show,
//!           start the timeout ──► app shows its window ──► popover.shown (clears the timeout)
//!  hide (the bar decides): same icon again, another item, another program got the foreground
//!           (after a short grace), the app never answered, the item or its app went away
//!           └► popover.hide ──► app hides its window (and does NOT answer with `closed`)
//!  the app closed it itself (Esc, window closed) ──► popover.closed ──► the bar forgets it
//! ```

use crate::bar::Rect;
use crate::dismiss::{Action, Machine, BLUR_GRACE};
use crate::placement::inset_work;
use crate::registry::{anchor, Registry};
use barista_protocol::{BarMessage, ScreenRect, Tone};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

/// How long the app has to answer `popover.show` with `popover.shown`. After that the bar hides
/// the popover and the item goes back to idle, so a stuck app never leaves it "open".
pub const SHOWN_TIMEOUT: Duration = Duration::from_millis(1500);

/// Where things are right now, from the controller.
pub struct Geo {
    pub bar: Rect,
    pub work: Rect,
    pub scale: f64,
    pub monitor: String,
    pub dark: bool,
}

type ItemRectFn = Box<dyn Fn(&str, &Rect, f64) -> Rect + Send + Sync>;

/// Everything the coordinator needs from the outside. Closures so tests can fake the screen,
/// the UI events and the Win32 call.
pub struct Env {
    pub registry: Arc<Registry>,
    pub geometry: Box<dyn Fn() -> Option<Geo> + Send + Sync>,
    pub item_rect: ItemRectFn,
    /// The open item changed (`None` = nothing open), for the UI's "open" state.
    pub emit: Box<dyn Fn(Option<String>) + Send + Sync>,
    pub close_menu: Box<dyn Fn() + Send + Sync>,
    pub allow_foreground: fn(u32),
    /// Which process has the foreground right now.
    pub foreground_pid: Box<dyn Fn() -> Option<u32> + Send + Sync>,
    pub shown_timeout: Duration,
}

/// The popover that is up (or being shown).
struct Open {
    key: String,
    pid: u32,
    /// The app confirmed with `popover.shown`.
    shown: bool,
    /// Tells a late timeout it belongs to an older popover.
    generation: u64,
}

#[derive(Default)]
struct Inner {
    machine: Machine,
    open: Option<Open>,
    generation: u64,
}

struct Shared {
    env: Env,
    inner: Mutex<Inner>,
}

/// Cheap to clone (an `Arc`), because the timers need their own handle.
#[derive(Clone)]
pub struct Popovers(Arc<Shared>);

static POPOVERS: OnceLock<Popovers> = OnceLock::new();

pub fn get() -> Option<&'static Popovers> {
    POPOVERS.get()
}

/// Where the popover goes: under the icon, `size` in logical px converted once to physical px,
/// kept inside the work area with the same margin menus use. The app gets this rect as is.
pub fn place_popover(item: &Rect, size: (u32, u32), scale: f64, work: &Rect) -> Rect {
    let px = |v: u32| (f64::from(v) * scale).round() as i32;
    let (w, h) = (px(size.0), px(size.1));
    anchor::place(item, (w, h), w, &inset_work(work, scale)).0
}

/// Makes the coordinator and starts watching the foreground window. Call once from `setup`.
pub fn init(app: &tauri::AppHandle, registry: Arc<Registry>) {
    use crate::events::PopoverChanged;
    use tauri_specta::Event;
    let handle = app.clone();
    let env = Env {
        registry,
        geometry: Box::new(|| {
            let (bar, work, scale) = crate::placement::geometry()?;
            let info = crate::controller::get()?.info();
            Some(Geo {
                bar,
                work,
                scale,
                monitor: info.monitor.id,
                dark: crate::appearance::current().tone
                    == crate::appearance::luminance::Tone::Dark,
            })
        }),
        item_rect: Box::new(crate::placement::item_rect),
        emit: Box::new(move |item_id| {
            let _ = PopoverChanged { item_id }.emit(&handle);
        }),
        close_menu: Box::new(|| {
            if let Some(menu) = crate::menu_window::get() {
                menu.close();
            }
            // A popover replaces an open panel too (one thing open at a time).
            crate::panel::close();
        }),
        allow_foreground: crate::platform::windows::foreground::allow,
        foreground_pid: Box::new(crate::platform::windows::foreground::current_pid),
        shown_timeout: SHOWN_TIMEOUT,
    };
    if POPOVERS.set(Popovers::new(env)).is_ok() {
        crate::platform::windows::foreground::watch(foreground_changed);
    }
}

/// Another program's window became the foreground (called by the Win32 hook).
fn foreground_changed(pid: u32) {
    if let Some(p) = get() {
        p.foreground_changed(pid);
    }
}

/// The app confirmed `popover.shown` for `key`.
pub fn shown(key: &str) {
    if let Some(p) = get() {
        p.shown(key);
    }
}

/// The app closed its popover itself.
pub fn closed(key: &str) {
    if let Some(p) = get() {
        p.closed(key);
    }
}

/// Closes whatever popover is open (a menu is opening, or the bar is clicked elsewhere).
pub fn close() {
    if let Some(p) = get() {
        p.close();
    }
}

impl Popovers {
    pub fn new(env: Env) -> Self {
        Self(Arc::new(Shared {
            env,
            inner: Mutex::new(Inner::default()),
        }))
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.0.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The key of the open popover, for `getOpenPopover`.
    pub fn current(&self) -> Option<String> {
        self.lock().open.as_ref().map(|o| o.key.clone())
    }

    /// A left click on an item that has a popover.
    pub fn item_click(&self, key: &str) {
        let action = self.lock().machine.click(key, Instant::now());
        match action {
            Action::Nothing => {}
            Action::Open(key) | Action::Swap(key) => self.show(&key),
            Action::Close => self.hide_open(),
        }
    }

    /// Places the rect, grants focus, sends `popover.show`, starts the timeout. A popover that is
    /// already up (a swap to another item) is hidden first.
    fn show(&self, key: &str) {
        let env = &self.0.env;
        let (Some(target), Some(geo)) = (env.registry.popover_target(key), (env.geometry)()) else {
            // The item vanished between the click and now, or the bar has no monitor: stay closed.
            self.close();
            return;
        };
        // One popover or menu at a time in the whole bar.
        (env.close_menu)();
        let previous = self.lock().open.take();
        if let Some(prev) = previous.filter(|p| p.key != key) {
            self.send_hide(&prev);
        }

        let item = (env.item_rect)(key, &geo.bar, geo.scale);
        let rect = place_popover(&item, target.size, geo.scale, &geo.work);
        // The grant must come right before the message: the app calls focus() when it gets it.
        (env.allow_foreground)(target.pid);
        let generation = {
            let mut inner = self.lock();
            inner.generation += 1;
            inner.open = Some(Open {
                key: key.to_string(),
                pid: target.pid,
                shown: false,
                generation: inner.generation,
            });
            inner.generation
        };
        let sent = env.registry.send_with(key, |id| BarMessage::PopoverShow {
            id,
            rect: ScreenRect {
                x: rect.left,
                y: rect.top,
                width: rect.width(),
                height: rect.height(),
            },
            scale: geo.scale,
            monitor: geo.monitor.clone(),
            tone: if geo.dark { Tone::Dark } else { Tone::Light },
        });
        if sent.is_err() {
            // The app is gone or stuck with a full queue: nothing will answer.
            self.forget(generation);
            return;
        }
        (env.emit)(Some(key.to_string()));
        let this = self.clone();
        let wait = env.shown_timeout;
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(wait).await;
            this.timed_out(generation);
        });
    }

    /// The app never confirmed: take the popover away.
    fn timed_out(&self, generation: u64) {
        let stuck = self
            .lock()
            .open
            .as_ref()
            .is_some_and(|o| o.generation == generation && !o.shown);
        if stuck {
            self.close();
        }
    }

    /// Clears the state for one popover without telling the app (it is gone or never answered).
    fn forget(&self, generation: u64) {
        let mut inner = self.lock();
        if inner.open.as_ref().is_some_and(|o| o.generation == generation) {
            inner.open = None;
            inner.machine.reset();
            drop(inner);
            (self.0.env.emit)(None);
        }
    }

    fn send_hide(&self, open: &Open) {
        // A full queue or a dead client just misses it: the bar never waits for an app.
        let _ = self
            .0
            .env
            .registry
            .send_with(&open.key, |id| BarMessage::PopoverHide { id });
    }

    /// Hides the open popover (the machine already decided to close it).
    fn hide_open(&self) {
        let open = {
            let mut inner = self.lock();
            inner.machine.reset();
            inner.open.take()
        };
        if let Some(open) = open {
            self.send_hide(&open);
            (self.0.env.emit)(None);
        }
    }

    /// Closes the popover if one is open. Safe to call any time.
    pub fn close(&self) {
        self.hide_open();
    }

    /// `popover.shown`: the app took the popover; the timeout no longer applies.
    pub fn shown(&self, key: &str) {
        let pid = {
            let mut inner = self.lock();
            let Some(open) = inner.open.as_mut().filter(|o| o.key == key) else {
                return;
            };
            open.shown = true;
            open.pid
        };
        // If Windows refused the app the focus, another program is still in front and no
        // "foreground changed" event will ever come for it: treat that as a click away now.
        // (If the app is just a moment late, its own focus event cancels the hide.)
        if let Some(front) = (self.0.env.foreground_pid)() {
            if front != pid {
                self.foreground_changed(front);
            }
        }
    }

    /// `popover.closed`: the app closed its window itself. Nothing to hide, just forget it.
    pub fn closed(&self, key: &str) {
        let was_open = {
            let mut inner = self.lock();
            if inner.open.as_ref().is_some_and(|o| o.key == key) {
                inner.open = None;
                inner.machine.reset();
                true
            } else {
                false
            }
        };
        if was_open {
            (self.0.env.emit)(None);
        }
    }

    /// Another window got the foreground. If it is not the client's, the user clicked away: hide
    /// after a short grace (an icon click in that time decides instead, like for menus).
    pub fn foreground_changed(&self, pid: u32) {
        {
            let mut inner = self.lock();
            // Until the app answered, its window may not be up yet: the timeout covers that case.
            let Some(open) = inner.open.as_ref().filter(|o| o.shown) else {
                return;
            };
            if open.pid == pid {
                inner.machine.focus();
                return;
            }
            inner.machine.blur(Instant::now());
        }
        let this = self.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(BLUR_GRACE).await;
            let action = this.lock().machine.blur_timeout(Instant::now());
            if action == Action::Close {
                // The machine already forgot the key; hide whatever is still open.
                let open = this.lock().open.take();
                if let Some(open) = open {
                    this.send_hide(&open);
                    (this.0.env.emit)(None);
                }
            }
        });
    }

    /// After any registry change: close the popover if its item is gone or hidden, its popover
    /// was removed, or its app disconnected.
    pub fn sync(&self) {
        let key = self.lock().open.as_ref().map(|o| o.key.clone());
        if let Some(key) = key {
            if !self.0.env.registry.has_popover(&key) {
                self.close();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::SessionId;
    use tokio::sync::mpsc;

    fn rect(l: i32, t: i32, r: i32, b: i32) -> Rect {
        Rect { left: l, top: t, right: r, bottom: b }
    }

    struct Rig {
        pop: Popovers,
        registry: Arc<Registry>,
        rx: mpsc::Receiver<BarMessage>,
        session: SessionId,
        events: Arc<Mutex<Vec<Option<String>>>>,
        menu_closes: Arc<Mutex<u32>>,
        /// Who the fake desktop says is in front (`None` = the client, 4242).
        front: Arc<Mutex<Option<u32>>>,
    }

    static GRANTS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

    fn rig(work: Rect, scale: f64, timeout_ms: u64) -> Rig {
        let registry = Arc::new(Registry::new());
        let (tx, rx) = mpsc::channel(16);
        let session = registry.add_session("App".into(), 4242, tx);
        registry.create(session, "note".into(), None, None, None).unwrap();
        registry.set_popover(session, "note", Some((320, 380))).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let menu_closes = Arc::new(Mutex::new(0));
        let (e, m) = (events.clone(), menu_closes.clone());
        let front = Arc::new(Mutex::new(None));
        let f = front.clone();
        let bar = rect(work.left, work.top - 36, work.right, work.top);
        let env = Env {
            registry: registry.clone(),
            geometry: Box::new(move || {
                Some(Geo { bar, work, scale, monitor: "m1".into(), dark: true })
            }),
            // The icon sits at x 300..324 on whichever monitor the bar is on.
            item_rect: Box::new(move |_, bar, _| rect(bar.left + 300, bar.top, bar.left + 324, bar.bottom)),
            emit: Box::new(move |v| e.lock().unwrap().push(v)),
            close_menu: Box::new(move || *m.lock().unwrap() += 1),
            allow_foreground: |pid| GRANTS.lock().unwrap().push(pid),
            foreground_pid: Box::new(move || f.lock().unwrap().or(Some(4242))),
            shown_timeout: Duration::from_millis(timeout_ms),
        };
        Rig {
            pop: Popovers::new(env),
            registry,
            rx,
            session,
            events,
            menu_closes,
            front,
        }
    }

    fn hd() -> Rect {
        rect(0, 36, 1920, 1080)
    }

    #[test]
    fn placement_sits_under_the_icon_at_every_scale() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let item = rect(300, 0, 324, (36.0 * scale) as i32);
            let work = rect(0, item.bottom, (1920.0 * scale) as i32, (1080.0 * scale) as i32);
            let r = place_popover(&item, (320, 380), scale, &work);
            assert_eq!(r.top, item.bottom, "scale {scale}");
            assert_eq!(r.left, 300, "scale {scale}");
            assert_eq!(r.width(), (320.0 * scale).round() as i32, "scale {scale}");
            assert_eq!(r.height(), (380.0 * scale).round() as i32, "scale {scale}");
        }
    }

    #[test]
    fn placement_works_on_a_monitor_left_of_the_primary_one() {
        // Negative coordinates: the second monitor sits to the left.
        let work = rect(-1920, 36, 0, 1080);
        let item = rect(-1620, 0, -1596, 36);
        let r = place_popover(&item, (320, 380), 1.0, &work);
        assert_eq!((r.left, r.top), (-1620, 36));
    }

    #[test]
    fn placement_near_the_right_edge_moves_left_and_stays_in_the_work_area() {
        let work = hd();
        let item = rect(1900, 0, 1910, 36);
        let r = place_popover(&item, (320, 380), 1.0, &work);
        assert_eq!(r.right, 1910); // inset by the 10 px margin
        assert!(r.left >= work.left && r.right <= work.right);
    }

    #[test]
    fn a_huge_popover_is_clamped_to_the_work_area() {
        let work = hd();
        let item = rect(300, 0, 324, 36);
        let r = place_popover(&item, (1024, 1024), 1.0, &work);
        assert!(r.bottom <= work.bottom && r.height() <= work.height());
    }

    #[tokio::test]
    async fn left_click_sends_show_with_the_placed_rect_and_a_grant() {
        let mut t = rig(hd(), 1.5, 5000);
        t.pop.item_click("s1/note");
        let BarMessage::PopoverShow { id, rect, scale, monitor, tone } = t.rx.try_recv().unwrap()
        else {
            panic!("expected popover.show");
        };
        assert_eq!((id.as_str(), scale, monitor.as_str(), tone), ("note", 1.5, "m1", Tone::Dark));
        assert_eq!((rect.x, rect.y, rect.width, rect.height), (300, 36, 480, 570));
        assert!(GRANTS.lock().unwrap().contains(&4242));
        assert_eq!(*t.menu_closes.lock().unwrap(), 1);
        assert_eq!(t.events.lock().unwrap().last(), Some(&Some("s1/note".to_string())));
        assert_eq!(t.pop.current().as_deref(), Some("s1/note"));
    }

    #[tokio::test]
    async fn clicking_the_icon_again_toggles() {
        let mut t = rig(hd(), 1.0, 5000);
        t.pop.item_click("s1/note");
        t.rx.try_recv().unwrap();
        t.pop.shown("s1/note");
        t.pop.item_click("s1/note");
        assert_eq!(t.rx.try_recv().unwrap(), BarMessage::PopoverHide { id: "note".into() });
        assert!(t.pop.current().is_none());
        assert_eq!(t.events.lock().unwrap().last(), Some(&None));
        // And a third click opens it again.
        t.pop.item_click("s1/note");
        assert!(matches!(t.rx.try_recv().unwrap(), BarMessage::PopoverShow { .. }));
    }

    #[tokio::test]
    async fn a_client_that_never_answers_times_out() {
        let mut t = rig(hd(), 1.0, 40);
        t.pop.item_click("s1/note");
        t.rx.try_recv().unwrap();
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert_eq!(t.rx.try_recv().unwrap(), BarMessage::PopoverHide { id: "note".into() });
        assert!(t.pop.current().is_none(), "no stuck open state");
        assert_eq!(t.events.lock().unwrap().last(), Some(&None));
    }

    #[tokio::test]
    async fn shown_cancels_the_timeout() {
        let mut t = rig(hd(), 1.0, 40);
        t.pop.item_click("s1/note");
        t.rx.try_recv().unwrap();
        t.pop.shown("s1/note");
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert!(t.rx.try_recv().is_err());
        assert!(t.pop.current().is_some());
    }

    #[tokio::test]
    async fn the_app_closing_itself_clears_the_open_state_without_a_hide() {
        let mut t = rig(hd(), 1.0, 5000);
        t.pop.item_click("s1/note");
        t.rx.try_recv().unwrap();
        t.pop.shown("s1/note");
        t.pop.closed("s1/note");
        assert!(t.pop.current().is_none());
        assert!(t.rx.try_recv().is_err());
        // A late or repeated `closed` is harmless.
        t.pop.closed("s1/note");
        t.pop.closed("s1/ghost");
    }

    #[tokio::test]
    async fn another_program_in_front_hides_it_after_the_grace_time() {
        let mut t = rig(hd(), 1.0, 5000);
        t.pop.item_click("s1/note");
        t.rx.try_recv().unwrap();
        t.pop.shown("s1/note");
        // The client's own window getting focus changes nothing.
        t.pop.foreground_changed(4242);
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert!(t.rx.try_recv().is_err());
        // Another program does.
        t.pop.foreground_changed(999);
        assert!(t.rx.try_recv().is_err(), "not before the grace time");
        tokio::time::sleep(Duration::from_millis(350)).await;
        assert_eq!(t.rx.try_recv().unwrap(), BarMessage::PopoverHide { id: "note".into() });
        assert!(t.pop.current().is_none());
    }

    #[tokio::test]
    async fn the_client_getting_focus_back_cancels_the_hide() {
        let mut t = rig(hd(), 1.0, 5000);
        t.pop.item_click("s1/note");
        t.rx.try_recv().unwrap();
        t.pop.shown("s1/note");
        t.pop.foreground_changed(999);
        t.pop.foreground_changed(4242);
        tokio::time::sleep(Duration::from_millis(350)).await;
        assert!(t.rx.try_recv().is_err());
        assert!(t.pop.current().is_some());
    }

    #[tokio::test]
    async fn shown_while_another_program_still_has_the_focus_hides_it() {
        // Windows refused the app the focus: no foreground event will ever come, so the check
        // at `shown` is the only chance to notice.
        let mut t = rig(hd(), 1.0, 5000);
        *t.front.lock().unwrap() = Some(999);
        t.pop.item_click("s1/note");
        t.rx.try_recv().unwrap();
        t.pop.shown("s1/note");
        *t.front.lock().unwrap() = None;
        tokio::time::sleep(Duration::from_millis(350)).await;
        assert_eq!(t.rx.try_recv().unwrap(), BarMessage::PopoverHide { id: "note".into() });
        assert!(t.pop.current().is_none());
    }

    #[tokio::test]
    async fn foreground_changes_before_shown_are_ignored() {
        let mut t = rig(hd(), 1.0, 5000);
        t.pop.item_click("s1/note");
        t.rx.try_recv().unwrap();
        t.pop.foreground_changed(999);
        tokio::time::sleep(Duration::from_millis(350)).await;
        assert!(t.rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn clicking_another_item_swaps_and_hides_the_first() {
        let mut t = rig(hd(), 1.0, 5000);
        t.registry.create(t.session, "other".into(), None, None, None).unwrap();
        t.registry.set_popover(t.session, "other", Some((200, 200))).unwrap();
        t.pop.item_click("s1/note");
        t.rx.try_recv().unwrap();
        t.pop.shown("s1/note");
        t.pop.item_click("s1/other");
        assert_eq!(t.rx.try_recv().unwrap(), BarMessage::PopoverHide { id: "note".into() });
        assert!(matches!(
            t.rx.try_recv().unwrap(),
            BarMessage::PopoverShow { id, .. } if id == "other"
        ));
        assert_eq!(t.pop.current().as_deref(), Some("s1/other"));
    }

    #[tokio::test]
    async fn removing_the_popover_or_the_session_closes_it() {
        let mut t = rig(hd(), 1.0, 5000);
        t.pop.item_click("s1/note");
        t.rx.try_recv().unwrap();
        t.pop.shown("s1/note");
        t.registry.set_popover(t.session, "note", None).unwrap();
        t.pop.sync();
        assert_eq!(t.rx.try_recv().unwrap(), BarMessage::PopoverHide { id: "note".into() });
        // A dead client: nothing to send to, but the state is cleared.
        t.registry.set_popover(t.session, "note", Some((320, 380))).unwrap();
        t.pop.item_click("s1/note");
        t.rx.try_recv().unwrap();
        t.registry.remove_session(t.session);
        t.pop.sync();
        assert!(t.pop.current().is_none());
    }

    #[tokio::test]
    async fn an_unreachable_client_leaves_nothing_open() {
        let t = rig(hd(), 1.0, 5000);
        drop(t.rx);
        t.pop.item_click("s1/note");
        assert!(t.pop.current().is_none());
        assert_eq!(t.events.lock().unwrap().last(), Some(&None));
    }
}
