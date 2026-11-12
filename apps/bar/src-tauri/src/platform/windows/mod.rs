//! Win32 code behind small modules, so the rest of the crate reads like plain Rust:
//! `appbar` (reserve screen space), `monitors` (list, work areas, DPI), `pipe` (named-pipe access and client pid), `window` (styles,
//! placement, message hook) and `fullscreen` (is a full-screen app in front?) and `wallpaper` (how bright is the wallpaper under the bar?).
//! Tray bridging helpers come later.

pub mod app_icon;
pub mod appbar;
pub mod audio;
pub mod folder_watch;
pub mod foreground;
pub mod locale;
pub mod fullscreen;
pub mod monitors;
pub mod notification_db;
pub mod notifications;
pub mod pipe;
pub mod power;
pub mod shell;
pub mod wallpaper;
pub mod wallpaper_watch;
pub mod windhawk;
pub mod window;
