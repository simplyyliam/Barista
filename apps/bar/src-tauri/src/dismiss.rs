//! The open/close rules shared by menus and popovers: one thing open at a time, toggle on the
//! same icon, a short grace after the focus is lost so the click that caused the blur decides.
//! Pure logic, no windows, so it is easy to test.

use std::time::{Duration, Instant};

/// After the menu loses focus, wait this long before hiding it. If a click on a bar icon
/// arrives in between, it decides instead: same icon = stay closed, another icon = switch
/// menus without hiding (no flicker).
pub const BLUR_GRACE: Duration = Duration::from_millis(150);

/// What the open/close logic decided. Kept apart from Tauri so it can be tested.
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Nothing,
    Open(String),
    /// Another item's menu replaces the open one; the window stays up.
    Swap(String),
    Close,
}

#[derive(Default)]
pub struct Machine {
    /// The item whose menu is showing.
    pub open: Option<String>,
    /// Set when the window lost focus while open.
    blurred_at: Option<Instant>,
    /// The item whose menu the blur timer just closed, and when.
    closed_by_blur: Option<(String, Instant)>,
}

impl Machine {
    pub fn click(&mut self, key: &str, now: Instant) -> Action {
        let recent = |t: Instant| now.duration_since(t) < BLUR_GRACE;
        match (&self.open, self.blurred_at) {
            (Some(open), _) if open == key => {
                self.reset();
                Action::Close
            }
            (Some(_), _) => {
                self.open = Some(key.to_string());
                self.blurred_at = None;
                Action::Swap(key.to_string())
            }
            (None, _) => {
                // The blur closed this very menu a moment ago: the click was meant to close it.
                if let Some((closed, at)) = &self.closed_by_blur {
                    if closed == key && recent(*at) {
                        self.closed_by_blur = None;
                        return Action::Nothing;
                    }
                }
                self.open = Some(key.to_string());
                self.blurred_at = None;
                Action::Open(key.to_string())
            }
        }
    }

    /// The mouse moved onto another bar item while a menu is open: switch menus, no click.
    /// Never opens a menu on its own and never closes one.
    pub fn hover(&mut self, key: &str) -> Action {
        match &self.open {
            Some(open) if open != key => {
                self.open = Some(key.to_string());
                self.blurred_at = None;
                Action::Swap(key.to_string())
            }
            _ => Action::Nothing,
        }
    }

    pub fn blur(&mut self, now: Instant) {
        if self.open.is_some() {
            self.blurred_at = Some(now);
        }
    }

    pub fn focus(&mut self) {
        self.blurred_at = None;
    }

    /// The grace timer fired.
    pub fn blur_timeout(&mut self, now: Instant) -> Action {
        match (&self.open, self.blurred_at) {
            (Some(open), Some(at)) if now.duration_since(at) >= BLUR_GRACE => {
                self.closed_by_blur = Some((open.clone(), now));
                self.open = None;
                self.blurred_at = None;
                Action::Close
            }
            _ => Action::Nothing,
        }
    }

    pub fn reset(&mut self) {
        self.open = None;
        self.blurred_at = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn later(t: Instant, ms: u64) -> Instant {
        t + Duration::from_millis(ms)
    }

    #[test]
    fn clicking_the_same_icon_toggles() {
        let mut m = Machine::default();
        let t = Instant::now();
        assert_eq!(m.click("s1/a", t), Action::Open("s1/a".into()));
        assert_eq!(m.click("s1/a", t), Action::Close);
        assert_eq!(m.click("s1/a", t), Action::Open("s1/a".into()));
    }

    #[test]
    fn clicking_another_icon_swaps_without_closing() {
        let mut m = Machine::default();
        let t = Instant::now();
        m.click("s1/a", t);
        assert_eq!(m.click("s2/b", t), Action::Swap("s2/b".into()));
        assert_eq!(m.open.as_deref(), Some("s2/b"));
    }

    #[test]
    fn hovering_another_icon_swaps_only_while_a_menu_is_open() {
        let mut m = Machine::default();
        let t = Instant::now();
        // Nothing open: hovering never opens a menu.
        assert_eq!(m.hover("s1/a"), Action::Nothing);
        m.click("s1/a", t);
        // Same icon: nothing. Another icon: swap.
        assert_eq!(m.hover("s1/a"), Action::Nothing);
        assert_eq!(m.hover("s2/b"), Action::Swap("s2/b".into()));
        assert_eq!(m.open.as_deref(), Some("s2/b"));
        // Hovering never closes: a click on the open icon still does.
        assert_eq!(m.click("s2/b", t), Action::Close);
        assert_eq!(m.hover("s1/a"), Action::Nothing);
    }

    #[test]
    fn hover_swap_cancels_a_pending_blur() {
        let mut m = Machine::default();
        let t = Instant::now();
        m.click("s1/a", t);
        m.blur(t);
        assert_eq!(m.hover("s2/b"), Action::Swap("s2/b".into()));
        assert_eq!(m.blur_timeout(later(t, 200)), Action::Nothing);
    }

    #[test]
    fn blur_closes_after_the_grace_time() {
        let mut m = Machine::default();
        let t = Instant::now();
        m.click("s1/a", t);
        m.blur(t);
        assert_eq!(m.blur_timeout(later(t, 50)), Action::Nothing);
        assert_eq!(m.blur_timeout(later(t, 200)), Action::Close);
        assert!(m.open.is_none());
    }

    #[test]
    fn a_click_during_the_grace_time_wins_over_the_blur() {
        let mut m = Machine::default();
        let t = Instant::now();
        m.click("s1/a", t);
        m.blur(t);
        // Another icon: swap, and the pending blur is cancelled.
        assert_eq!(m.click("s2/b", later(t, 40)), Action::Swap("s2/b".into()));
        assert_eq!(m.blur_timeout(later(t, 200)), Action::Nothing);
        // Same icon during a blur: close.
        m.blur(later(t, 300));
        assert_eq!(m.click("s2/b", later(t, 320)), Action::Close);
    }

    #[test]
    fn same_icon_right_after_the_blur_closed_it_stays_closed() {
        let mut m = Machine::default();
        let t = Instant::now();
        m.click("s1/a", t);
        m.blur(t);
        assert_eq!(m.blur_timeout(later(t, 150)), Action::Close);
        assert_eq!(m.click("s1/a", later(t, 200)), Action::Nothing);
        // Later clicks open it again.
        assert_eq!(m.click("s1/a", later(t, 1000)), Action::Open("s1/a".into()));
    }

    #[test]
    fn focus_back_cancels_the_blur() {
        let mut m = Machine::default();
        let t = Instant::now();
        m.click("s1/a", t);
        m.blur(t);
        m.focus();
        assert_eq!(m.blur_timeout(later(t, 500)), Action::Nothing);
    }
}
