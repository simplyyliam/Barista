//! Notifications (ADR 0017 and 0018). Windows keeps the toasts; we read them, group them by app and
//! hand the UI one snapshot. Two sources are merged by toast id:
//!
//! ```text
//! UserNotificationListener  ids, app name and id, title and body (public API, text only)
//! wpndatabase.db (read-only) the toast XML: images, attribution line, button labels
//! ```
//!
//! Nothing here trusts what an app wrote in its toast: texts are clipped, image paths are checked
//! (no network paths, no `..`) and files are only read when they really are a small image.

use super::Service;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Never show more than this many notifications, lines or characters, whatever an app sends.
const MAX_ITEMS: usize = 100;
const MAX_LINES: usize = 8;
const MAX_TITLE: usize = 200;
const MAX_LINE: usize = 500;
/// Biggest image file we read (bytes).
pub const MAX_IMAGE_BYTES: u64 = 4 * 1024 * 1024;

/// May the bar read notifications? `Unknown` also covers "Windows has not decided yet".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Access {
    Allowed,
    Denied,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// The toast id (what `dismissNotification` and `openNotification` take).
    pub id: u32,
    pub title: String,
    /// The other text lines, in order.
    pub lines: Vec<String>,
    /// The small grey line some apps add below the text.
    pub attribution: Option<String>,
    /// Unix seconds.
    pub arrived_at: u32,
    /// There is a picture to fetch with `getNotificationImage(id)`.
    pub has_image: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    /// The app's user model id (what `getNotificationAppIcon` takes).
    pub app_id: String,
    pub app_name: String,
    /// Newest first.
    pub items: Vec<Item>,
}

/// The full snapshot. Groups are ordered by their newest notification.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Notifications {
    pub access: Access,
    /// Something arrived since the panel was last opened.
    pub unread: bool,
    pub groups: Vec<Group>,
}

/// One toast as the listener reports it.
#[derive(Debug, Clone, PartialEq)]
pub struct RawToast {
    pub id: u32,
    pub app_id: String,
    pub app_name: String,
    pub texts: Vec<String>,
    pub arrived: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub src: String,
    pub placement: Option<String>,
}

/// What the toast XML says.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Toast {
    pub texts: Vec<String>,
    pub attribution: Option<String>,
    pub images: Vec<Image>,
    pub actions: Vec<String>,
}

/// Reads the toast XML. `None` if it is not XML we can parse (the toast then shows text only).
pub fn parse_toast(xml: &str) -> Option<Toast> {
    let doc = roxmltree::Document::parse(xml).ok()?;
    let mut toast = Toast::default();
    for node in doc.descendants().filter(|n| n.is_element()) {
        match node.tag_name().name() {
            "text" => {
                let text: String = node
                    .descendants()
                    .filter(|n| n.is_text())
                    .filter_map(|n| n.text())
                    .collect();
                if node.attribute("placement") == Some("attribution") {
                    toast.attribution = Some(text);
                } else if !text.trim().is_empty() {
                    toast.texts.push(text);
                }
            }
            "image" => {
                if let Some(src) = node.attribute("src") {
                    toast.images.push(Image {
                        src: src.to_string(),
                        placement: node.attribute("placement").map(str::to_string),
                    });
                }
            }
            "action" => {
                if let Some(label) = node.attribute("content") {
                    toast.actions.push(label.to_string());
                }
            }
            _ => {}
        }
    }
    Some(toast)
}

/// The picture to show: the first one that is not the small app logo override.
pub fn pick_image(toast: &Toast) -> Option<&Image> {
    toast
        .images
        .iter()
        .find(|i| i.placement.as_deref() != Some("appLogoOverride"))
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            // Work on bytes: slicing the str could land inside a multi-byte character.
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(v) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `C:\x` or `C:/x`: a local drive path (and not `\\server\share`, which would make Windows send
/// our login to that server).
fn local_drive_path(p: &str) -> Option<PathBuf> {
    let b = p.as_bytes();
    let ok = b.len() > 3
        && b[0].is_ascii_alphabetic()
        && b[1] == b':'
        && (b[2] == b'\\' || b[2] == b'/');
    if !ok || has_dot_dot(p) {
        return None;
    }
    Some(PathBuf::from(p.replace('/', "\\")))
}

fn has_dot_dot(p: &str) -> bool {
    p.split(['/', '\\']).any(|part| part == "..")
}

/// Where an image `src` lives on disk, or `None` if we won't read it (web addresses, relative
/// package assets, network shares, anything with `..`).
///
/// `ms-appdata:///local/x.png` is `%LOCALAPPDATA%\Packages\<package family>\LocalState\x.png`;
/// the package family is the app id up to the `!`.
pub fn resolve_image(src: &str, app_id: &str, local_app_data: &Path) -> Option<PathBuf> {
    let lower = src.to_ascii_lowercase();
    if lower.starts_with("file:///") {
        return local_drive_path(&percent_decode(&src[8..]));
    }
    if lower.starts_with("ms-appdata:///") {
        let rest = percent_decode(&src[14..]);
        let (scope, file) = rest.split_once('/')?;
        let folder = match scope.to_ascii_lowercase().as_str() {
            "local" => "LocalState",
            "temp" => "TempState",
            "roaming" => "RoamingState",
            _ => return None,
        };
        let family = app_id.split_once('!')?.0;
        // Only packaged apps have a family (`Name_publisherid`), and it must be a plain name.
        if !family.contains('_') || family.contains(['/', '\\', ':']) || has_dot_dot(&rest) {
            return None;
        }
        let mut path = local_app_data.join("Packages").join(family).join(folder);
        for part in file.split('/').filter(|p| !p.is_empty()) {
            path.push(part);
        }
        return Some(path);
    }
    local_drive_path(&percent_decode(src))
}

/// The image type from the first bytes, or `None` if it is not PNG, JPEG, GIF or WebP.
pub fn image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF8") {
        Some("image/gif")
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

fn clip(s: &str, max: usize) -> String {
    let s = s.trim();
    match s.char_indices().nth(max) {
        Some((i, _)) => format!("{}…", &s[..i]),
        None => s.to_string(),
    }
}

/// Builds the snapshot: newest first, grouped by app, clipped, with the picture and attribution
/// from the XML where there is one. `seen_at` is when the panel was last opened.
pub fn build_snapshot(
    mut raws: Vec<RawToast>,
    xml: &HashMap<u32, Toast>,
    access: Access,
    seen_at: u32,
    local_app_data: &Path,
) -> Notifications {
    raws.sort_by(|a, b| b.arrived.cmp(&a.arrived).then(b.id.cmp(&a.id)));
    raws.truncate(MAX_ITEMS);
    let unread = raws.iter().any(|r| r.arrived > seen_at);

    let mut groups: Vec<Group> = Vec::new();
    for raw in raws {
        let toast = xml.get(&raw.id);
        let texts = if raw.texts.is_empty() {
            toast.map(|t| t.texts.clone()).unwrap_or_default()
        } else {
            raw.texts.clone()
        };
        let mut texts = texts.into_iter();
        let title = clip(&texts.next().unwrap_or_default(), MAX_TITLE);
        let lines = texts.take(MAX_LINES).map(|l| clip(&l, MAX_LINE)).collect();
        let has_image = toast
            .and_then(pick_image)
            .and_then(|i| resolve_image(&i.src, &raw.app_id, local_app_data))
            .is_some();
        let item = Item {
            id: raw.id,
            title,
            lines,
            attribution: toast
                .and_then(|t| t.attribution.as_deref())
                .map(|a| clip(a, MAX_LINE))
                .filter(|a| !a.is_empty()),
            arrived_at: raw.arrived,
            has_image,
        };
        match groups.iter_mut().find(|g| g.app_id == raw.app_id) {
            Some(group) => group.items.push(item),
            None => groups.push(Group {
                app_name: if raw.app_name.is_empty() {
                    raw.app_id.clone()
                } else {
                    clip(&raw.app_name, MAX_TITLE)
                },
                app_id: raw.app_id,
                items: vec![item],
            }),
        }
    }
    Notifications {
        access,
        unread,
        groups,
    }
}

pub fn service() -> &'static Service<Notifications> {
    static S: OnceLock<Service<Notifications>> = OnceLock::new();
    S.get_or_init(|| Service::new(Notifications::default()))
}

// ---- the live part: threads, Windows and settings ----------------------------------------

use crate::platform::windows::{app_icon, folder_watch, notification_db, notifications as listener};
use crate::settings::Mode;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// What the last read found, kept so a dismissal or "seen" can rebuild without asking Windows.
struct Last {
    raws: Vec<RawToast>,
    xml: HashMap<u32, Toast>,
    access: Access,
}

static LAST: Mutex<Option<Last>> = Mutex::new(None);
/// False in `windows` mode (CLAUDE.md rule 11): events are ignored and nothing is read.
static ENABLED: AtomicBool = AtomicBool::new(false);

fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

fn lock_last() -> std::sync::MutexGuard<'static, Option<Last>> {
    LAST.lock().unwrap_or_else(|e| e.into_inner())
}

fn now() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().min(u32::MAX as u64) as u32)
        .unwrap_or(0)
}

fn local_app_data() -> PathBuf {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default()
}

pub fn mode() -> Mode {
    crate::controller::get()
        .map(|c| c.settings().notifications_mode)
        .unwrap_or_default()
}

/// Starts the reader (once, from `system::init`). It sleeps until Windows writes to its
/// notification folder.
pub fn start() {
    ENABLED.store(mode() == Mode::Barista, Ordering::Relaxed);
    listener::start(publish);
    if let Some(dir) = notification_db::folder() {
        folder_watch::start(dir, folder_changed);
    }
    *lock_fingerprint() = notification_db::fingerprint();
    refresh();
}

/// What the toast set looked like when we last asked; see `folder_changed`.
static FINGERPRINT: Mutex<Option<u64>> = Mutex::new(None);
/// When the last unconditional refresh ran, for the case where the fingerprint can't be read.
static LAST_BLIND: Mutex<Option<std::time::Instant>> = Mutex::new(None);

fn lock_fingerprint() -> std::sync::MutexGuard<'static, Option<u64>> {
    FINGERPRINT.lock().unwrap_or_else(|e| e.into_inner())
}

/// Reads again only if the set of toasts really changed (a cheap database check). Used when the panel
/// opens: it catches removals Windows never signalled, and when nothing changed it does nothing, so
/// the open is not followed by a second layout.
pub fn refresh_if_changed() {
    folder_changed();
}

/// Windows wrote in its notification folder. Reading through the listener writes there too, so a
/// plain "refresh on every write" would feed itself. Instead ask the database cheaply whether the set
/// of toasts really changed, and only then read for real.
fn folder_changed() {
    if !enabled() {
        return;
    }
    match notification_db::fingerprint() {
        Some(now) => {
            let mut last = lock_fingerprint();
            if *last == Some(now) {
                return;
            }
            *last = Some(now);
        }
        None => {
            // Can't tell: refresh, but at most every 10 s so we never spin.
            let mut blind = LAST_BLIND.lock().unwrap_or_else(|e| e.into_inner());
            if blind.is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(10)) {
                return;
            }
            *blind = Some(std::time::Instant::now());
        }
    }
    refresh();
}

/// Reads again (debounced by the worker). Does nothing in `windows` mode.
pub fn refresh() {
    if enabled() {
        listener::send(listener::Job::Refresh);
    }
}

/// Runs on the worker thread with what the listener found.
fn publish(raws: Vec<RawToast>, access: Access) {
    if !enabled() {
        return;
    }
    let xml = notification_db::read_toasts()
        .into_iter()
        .filter_map(|(id, text)| Some((id, parse_toast(&text)?)))
        .collect();
    *lock_last() = Some(Last { raws, xml, access });
    rebuild();
}

fn rebuild() {
    let seen_at = crate::controller::get()
        .map(|c| c.settings().notifications_seen_at)
        .unwrap_or(0);
    let guard = lock_last();
    let Some(last) = guard.as_ref() else { return };
    service().set(build_snapshot(
        last.raws.clone(),
        &last.xml,
        last.access,
        seen_at,
        &local_app_data(),
    ));
}

/// The panel was opened: everything so far counts as seen.
pub fn mark_seen() {
    if let Some(c) = crate::controller::get() {
        c.update_settings(|s| s.notifications_seen_at = now());
    }
    rebuild();
}

/// Removes one toast from Windows too (so its own list does not pile up). The UI sees it go at once.
pub fn dismiss(id: u32) {
    if !enabled() {
        return;
    }
    if let Some(last) = lock_last().as_mut() {
        last.raws.retain(|r| r.id != id);
    }
    rebuild();
    listener::send(listener::Job::Remove(id));
}

pub fn clear() {
    if !enabled() {
        return;
    }
    if let Some(last) = lock_last().as_mut() {
        last.raws.clear();
    }
    rebuild();
    listener::send(listener::Job::Clear);
}

fn app_id_of(id: u32) -> Option<String> {
    service()
        .get()
        .groups
        .into_iter()
        .find(|g| g.items.iter().any(|i| i.id == id))
        .map(|g| g.app_id)
}

/// A click on a notification: start (or focus) the app that sent it, then remove the toast.
pub fn open(id: u32) {
    if let Some(app_id) = app_id_of(id) {
        app_icon::launch(&app_id);
    }
    dismiss(id);
}

/// The notification's picture as a data URL, or `None` (no picture, not a small real image, gone).
pub fn image(id: u32) -> Option<String> {
    let app_id = app_id_of(id)?;
    let path = {
        let guard = lock_last();
        let toast = guard.as_ref()?.xml.get(&id)?;
        resolve_image(&pick_image(toast)?.src, &app_id, &local_app_data())?
    };
    let meta = std::fs::metadata(&path).ok()?;
    if !meta.is_file() || meta.len() > MAX_IMAGE_BYTES {
        return None;
    }
    let bytes = std::fs::read(&path).ok()?;
    let mime = image_mime(&bytes)?;
    Some(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
}

/// An app's icon as a PNG data URL, remembered for next time (also remembers "none").
pub fn app_icon(app_id: &str) -> Option<String> {
    static CACHE: Mutex<Option<HashMap<String, Option<String>>>> = Mutex::new(None);
    if let Some(hit) = CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .and_then(|c| c.get(app_id))
    {
        return hit.clone();
    }
    let icon = app_icon::icon_png(app_id)
        .map(|png| format!("data:image/png;base64,{}", STANDARD.encode(png)));
    CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_or_insert_with(HashMap::new)
        .insert(app_id.to_string(), icon.clone());
    icon
}

/// Switches between our panel and Windows' own (CLAUDE.md rule 11). Saved.
pub fn set_mode(mode: Mode) {
    if let Some(c) = crate::controller::get() {
        c.update_settings(|s| s.notifications_mode = mode);
    }
    ENABLED.store(mode == Mode::Barista, Ordering::Relaxed);
    if mode == Mode::Barista {
        refresh();
    } else {
        *lock_last() = None;
        service().set(Notifications::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = r#"<toast><visual><binding template="ToastGeneric">
        <text>Screenshot</text><text>Copied to clipboard</text>
        <text placement="attribution">via Snipping Tool</text>
        <image src="file:///C:/Users/me/My%20Pics/shot.png"/>
        <image placement="appLogoOverride" src="C:\logo.png"/>
        </binding></visual><actions><action content="Open" arguments="o"/><action content="Share" arguments="s"/></actions></toast>"#;

    fn local() -> PathBuf {
        PathBuf::from(r"C:\Users\me\AppData\Local")
    }

    #[test]
    fn parses_texts_attribution_images_and_buttons() {
        let t = parse_toast(XML).unwrap();
        assert_eq!(t.texts, ["Screenshot", "Copied to clipboard"]);
        assert_eq!(t.attribution.as_deref(), Some("via Snipping Tool"));
        assert_eq!(t.images.len(), 2);
        assert_eq!(t.actions, ["Open", "Share"]);
        // The logo override is not the picture.
        assert_eq!(pick_image(&t).unwrap().src, "file:///C:/Users/me/My%20Pics/shot.png");
    }

    #[test]
    fn broken_xml_is_none() {
        assert!(parse_toast("<toast><text>").is_none());
        assert!(parse_toast("not xml").is_none());
    }

    #[test]
    fn resolves_file_urls_and_decodes_spaces() {
        assert_eq!(
            resolve_image("file:///C:/Users/me/My%20Pics/shot.png", "x", &local()),
            Some(PathBuf::from(r"C:\Users\me\My Pics\shot.png"))
        );
        assert_eq!(
            resolve_image(r"D:\a\b.jpg", "x", &local()),
            Some(PathBuf::from(r"D:\a\b.jpg"))
        );
    }

    #[test]
    fn resolves_package_folders() {
        assert_eq!(
            resolve_image("ms-appdata:///local/img/a.png", "Pkg.App_abc123!App", &local()),
            Some(local().join("Packages").join("Pkg.App_abc123").join("LocalState").join("img").join("a.png"))
        );
        assert_eq!(
            resolve_image("ms-appdata:///temp/a.png", "Pkg.App_abc123!App", &local()),
            Some(local().join("Packages").join("Pkg.App_abc123").join("TempState").join("a.png"))
        );
        // Not a packaged app: nothing to resolve against.
        assert_eq!(resolve_image("ms-appdata:///local/a.png", "com.github.githubapp", &local()), None);
    }

    #[test]
    fn refuses_network_relative_and_escaping_paths() {
        for src in [
            "https://example.com/a.png",
            "http://example.com/a.png",
            r"\\server\share\a.png",
            "file://server/share/a.png",
            "file:////server/share/a.png",
            "Assets/logo.png",
            "ms-appx:///Assets/logo.png",
            "file:///C:/a/../../secret.png",
            "ms-appdata:///local/../../x.png",
            "ms-appdata:///other/a.png",
        ] {
            assert_eq!(resolve_image(src, "Pkg.App_abc123!App", &local()), None, "{src}");
        }
    }

    #[test]
    fn only_real_images_pass() {
        assert_eq!(image_mime(&[0x89, b'P', b'N', b'G', 0, 0]), Some("image/png"));
        assert_eq!(image_mime(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
        assert_eq!(image_mime(b"GIF89a"), Some("image/gif"));
        assert_eq!(image_mime(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));
        assert_eq!(image_mime(b"MZ\x90\0 an exe"), None);
        assert_eq!(image_mime(b"<svg xmlns"), None);
        assert_eq!(image_mime(b""), None);
    }

    fn raw(id: u32, app: &str, arrived: u32, texts: &[&str]) -> RawToast {
        RawToast {
            id,
            app_id: app.into(),
            app_name: app.to_uppercase(),
            texts: texts.iter().map(|t| t.to_string()).collect(),
            arrived,
        }
    }

    #[test]
    fn groups_by_app_newest_first() {
        let raws = vec![
            raw(1, "a", 10, &["old a"]),
            raw(2, "b", 30, &["new b", "body"]),
            raw(3, "a", 20, &["mid a"]),
        ];
        let n = build_snapshot(raws, &HashMap::new(), Access::Allowed, 0, &local());
        let apps: Vec<_> = n.groups.iter().map(|g| g.app_id.as_str()).collect();
        assert_eq!(apps, ["b", "a"]);
        assert_eq!(n.groups[0].items[0].lines, ["body"]);
        assert_eq!(n.groups[1].items.iter().map(|i| i.id).collect::<Vec<_>>(), [3, 1]);
        assert_eq!(n.groups[1].app_name, "A");
    }

    #[test]
    fn unread_means_newer_than_last_seen() {
        let raws = vec![raw(1, "a", 100, &["x"])];
        let at = |seen| build_snapshot(raws.clone(), &HashMap::new(), Access::Allowed, seen, &local()).unread;
        assert!(at(99));
        assert!(!at(100));
        assert!(!build_snapshot(vec![], &HashMap::new(), Access::Allowed, 0, &local()).unread);
    }

    #[test]
    fn image_and_attribution_come_from_the_xml() {
        let mut xml = HashMap::new();
        xml.insert(7, parse_toast(XML).unwrap());
        let n = build_snapshot(vec![raw(7, "a", 5, &["Screenshot"])], &xml, Access::Allowed, 0, &local());
        let item = &n.groups[0].items[0];
        assert!(item.has_image);
        assert_eq!(item.attribution.as_deref(), Some("via Snipping Tool"));
        // No XML for the toast: text only, no image.
        let n = build_snapshot(vec![raw(8, "a", 5, &["x"])], &xml, Access::Allowed, 0, &local());
        assert!(!n.groups[0].items[0].has_image);
    }

    #[test]
    fn long_and_many_texts_are_clipped() {
        let long = "x".repeat(2000);
        let many: Vec<&str> = std::iter::repeat("line").take(30).collect();
        let mut texts = vec![long.as_str()];
        texts.extend(many);
        let n = build_snapshot(vec![raw(1, "a", 1, &texts)], &HashMap::new(), Access::Allowed, 0, &local());
        let item = &n.groups[0].items[0];
        assert_eq!(item.title.chars().count(), MAX_TITLE + 1);
        assert_eq!(item.lines.len(), MAX_LINES);
        let raws: Vec<_> = (0..300).map(|i| raw(i, "a", i, &["x"])).collect();
        let n = build_snapshot(raws, &HashMap::new(), Access::Allowed, 0, &local());
        assert_eq!(n.groups[0].items.len(), MAX_ITEMS);
    }
}
