//! An item's menu, as the bar keeps it. Pure functions on the template the client sent:
//! - `arrange` gives roles their standard place (about and settings first, quit last),
//! - `view` turns it into what the UI draws (hidden entries dropped, role labels filled in),
//! - `click` applies a click: checkboxes flip, radios pick one of their group.
//!
//! The bar owns the checked state (ADR 0011), so every client gets the same radio rules and the
//! UI can redraw right away without waiting for the app.

use barista_protocol::{MenuItem, MenuItemType, MenuRole};
use serde::{Deserialize, Serialize};
use specta::Type;

/// One menu entry as the UI draws it. Separators have no `id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MenuEntry {
    pub id: Option<String>,
    pub kind: MenuItemType,
    pub label: String,
    /// Draw a check mark (checkbox) or dot (radio).
    pub checked: bool,
    /// False = draw greyed out and ignore clicks.
    pub enabled: bool,
    /// Text like `Ctrl+S`, drawn on the right. Only shown, never registered as a hotkey.
    pub accelerator: Option<String>,
    pub role: Option<MenuRole>,
    /// A small `data:` URL, drawn at 16 px.
    pub icon: Option<String>,
    /// Entries of a `submenu` entry; empty for the others.
    pub submenu: Vec<MenuEntry>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MenuClickError {
    /// No entry with that id (the menu may have just changed).
    NotFound,
    /// A separator, a submenu, or a disabled or hidden entry.
    NotClickable,
}

fn separator() -> MenuItem {
    MenuItem {
        kind: Some(MenuItemType::Separator),
        ..Default::default()
    }
}

fn is_separator(item: &MenuItem) -> bool {
    item.kind() == MenuItemType::Separator
}

/// Moves top-level role entries to their standard place, with a separator between the groups:
/// `About`, `Settings` | the app's own entries | `Quit`. Roles inside submenus stay where the
/// app put them.
pub fn arrange(items: Vec<MenuItem>) -> Vec<MenuItem> {
    if !items.iter().any(|i| i.role.is_some()) {
        return items;
    }
    let (mut about, mut settings, mut quit, mut rest) = (vec![], vec![], vec![], vec![]);
    for item in items {
        match item.role {
            Some(MenuRole::About) => about.push(item),
            Some(MenuRole::Settings) => settings.push(item),
            Some(MenuRole::Quit) => quit.push(item),
            None => rest.push(item),
        }
    }
    // The app's own separators at the edges would double up with ours.
    while rest.first().is_some_and(is_separator) {
        rest.remove(0);
    }
    while rest.last().is_some_and(is_separator) {
        rest.pop();
    }
    let mut out = about;
    out.append(&mut settings);
    for group in [rest, quit] {
        if group.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(separator());
        }
        out.extend(group);
    }
    out
}

/// What the UI draws. `app_name` fills in role labels the app left out.
pub fn view(items: &[MenuItem], app_name: &str) -> Vec<MenuEntry> {
    let mut out: Vec<MenuEntry> = Vec::with_capacity(items.len());
    for item in items.iter().filter(|i| i.visible != Some(false)) {
        let kind = item.kind();
        // Hiding entries can leave two separators in a row, or one at an edge: drop those.
        if kind == MenuItemType::Separator
            && out.last().map_or(true, |e| e.kind == MenuItemType::Separator)
        {
            continue;
        }
        let label = item.label.clone().unwrap_or_else(|| match item.role {
            Some(MenuRole::About) => format!("About {app_name}"),
            Some(MenuRole::Settings) => "Settings…".into(),
            Some(MenuRole::Quit) => format!("Quit {app_name}"),
            None => String::new(),
        });
        out.push(MenuEntry {
            id: item.id.clone(),
            kind,
            label,
            checked: item.checked.unwrap_or(false),
            enabled: item.enabled != Some(false),
            accelerator: item.accelerator.clone(),
            role: item.role,
            icon: item.icon.clone(),
            submenu: item
                .submenu
                .as_deref()
                .map(|s| view(s, app_name))
                .unwrap_or_default(),
        });
    }
    if out
        .last()
        .is_some_and(|e| e.kind == MenuItemType::Separator)
    {
        out.pop();
    }
    out
}

/// Applies a click on entry `id`. Returns the new `checked` for checkbox and radio entries.
pub fn click(items: &mut [MenuItem], id: &str) -> Result<Option<bool>, MenuClickError> {
    click_in(items, id, true).unwrap_or(Err(MenuClickError::NotFound))
}

/// `None` = not in this level or below. `reachable` is false under a disabled or hidden
/// submenu: the user can't see those entries, so they can't be clicked either.
fn click_in(
    level: &mut [MenuItem],
    id: &str,
    reachable: bool,
) -> Option<Result<Option<bool>, MenuClickError>> {
    if let Some(i) = level.iter().position(|e| e.id.as_deref() == Some(id)) {
        return Some(apply(level, i, reachable));
    }
    for entry in level.iter_mut() {
        let open = reachable && entry.enabled != Some(false) && entry.visible != Some(false);
        if let Some(sub) = entry.submenu.as_mut() {
            if let Some(result) = click_in(sub, id, open) {
                return Some(result);
            }
        }
    }
    None
}

fn apply(
    level: &mut [MenuItem],
    i: usize,
    reachable: bool,
) -> Result<Option<bool>, MenuClickError> {
    let entry = &level[i];
    if !reachable || entry.enabled == Some(false) || entry.visible == Some(false) {
        return Err(MenuClickError::NotClickable);
    }
    match entry.kind() {
        MenuItemType::Normal => Ok(None),
        MenuItemType::Separator | MenuItemType::Submenu => Err(MenuClickError::NotClickable),
        MenuItemType::Checkbox => {
            let now = !level[i].checked.unwrap_or(false);
            level[i].checked = Some(now);
            Ok(Some(now))
        }
        MenuItemType::Radio => {
            // Electron's rule: a radio group is a run of radio entries next to each other.
            let is_radio = |e: &MenuItem| e.kind() == MenuItemType::Radio;
            let start = level[..i]
                .iter()
                .rposition(|e| !is_radio(e))
                .map_or(0, |p| p + 1);
            let end = level[i..]
                .iter()
                .position(|e| !is_radio(e))
                .map_or(level.len(), |p| i + p);
            for (j, e) in level[start..end].iter_mut().enumerate() {
                e.checked = Some(start + j == i);
            }
            Ok(Some(true))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str) -> MenuItem {
        MenuItem {
            id: Some(id.into()),
            label: Some(id.into()),
            ..Default::default()
        }
    }

    fn typed(id: &str, kind: MenuItemType) -> MenuItem {
        MenuItem {
            kind: Some(kind),
            ..entry(id)
        }
    }

    fn role(id: &str, role: MenuRole) -> MenuItem {
        MenuItem {
            id: Some(id.into()),
            role: Some(role),
            ..Default::default()
        }
    }

    fn ids(items: &[MenuItem]) -> Vec<&str> {
        items
            .iter()
            .map(|i| i.id.as_deref().unwrap_or("-"))
            .collect()
    }

    #[test]
    fn roles_get_their_standard_place() {
        let menu = vec![
            role("q", MenuRole::Quit),
            separator(),
            entry("a"),
            role("s", MenuRole::Settings),
            entry("b"),
            role("i", MenuRole::About),
            separator(),
        ];
        assert_eq!(ids(&arrange(menu)), ["i", "s", "-", "a", "b", "-", "q"]);
        assert_eq!(ids(&arrange(vec![role("q", MenuRole::Quit)])), ["q"]);
        // No roles: untouched, separators and all.
        assert_eq!(ids(&arrange(vec![separator(), entry("a")])), ["-", "a"]);
    }

    #[test]
    fn view_fills_role_labels_and_drops_hidden_entries() {
        let hidden = MenuItem {
            visible: Some(false),
            ..entry("h")
        };
        let menu = arrange(vec![
            entry("a"),
            separator(),
            hidden,
            separator(),
            role("q", MenuRole::Quit),
        ]);
        let v = view(&menu, "Timer");
        let labels: Vec<_> = v.iter().map(|e| e.label.as_str()).collect();
        assert_eq!(labels, ["a", "", "Quit Timer"]);
        assert!(v[0].enabled && !v[0].checked);
        assert_eq!(
            view(&[role("i", MenuRole::About)], "Timer")[0].label,
            "About Timer"
        );
    }

    #[test]
    fn checkbox_flips_and_reports_the_new_state() {
        let mut menu = vec![typed("sound", MenuItemType::Checkbox)];
        assert_eq!(click(&mut menu, "sound"), Ok(Some(true)));
        assert_eq!(click(&mut menu, "sound"), Ok(Some(false)));
        assert_eq!(menu[0].checked, Some(false));
    }

    #[test]
    fn radio_picks_one_in_its_group_only() {
        let mut menu = vec![
            typed("r1", MenuItemType::Radio),
            typed("r2", MenuItemType::Radio),
            separator(),
            MenuItem {
                checked: Some(true),
                ..typed("other", MenuItemType::Radio)
            },
        ];
        assert_eq!(click(&mut menu, "r2"), Ok(Some(true)));
        assert_eq!(click(&mut menu, "r1"), Ok(Some(true)));
        let checked: Vec<_> = menu.iter().map(|e| e.checked).collect();
        assert_eq!(checked, [Some(true), Some(false), None, Some(true)]);
    }

    #[test]
    fn clicks_find_entries_in_submenus() {
        let presets = MenuItem {
            submenu: Some(vec![
                typed("p25", MenuItemType::Radio),
                typed("p50", MenuItemType::Radio),
            ]),
            ..entry("presets")
        };
        let mut menu = vec![entry("start"), presets];
        assert_eq!(click(&mut menu, "p50"), Ok(Some(true)));
        assert_eq!(click(&mut menu, "start"), Ok(None));
        assert_eq!(menu[1].submenu.as_ref().unwrap()[1].checked, Some(true));
    }

    #[test]
    fn unclickable_entries_are_refused() {
        let disabled = MenuItem {
            enabled: Some(false),
            ..entry("off")
        };
        let locked = MenuItem {
            enabled: Some(false),
            submenu: Some(vec![entry("inner")]),
            ..entry("locked")
        };
        let mut menu = vec![disabled, separator(), locked];
        assert_eq!(click(&mut menu, "off"), Err(MenuClickError::NotClickable));
        assert_eq!(
            click(&mut menu, "locked"),
            Err(MenuClickError::NotClickable)
        );
        assert_eq!(click(&mut menu, "inner"), Err(MenuClickError::NotClickable));
        assert_eq!(click(&mut menu, "nope"), Err(MenuClickError::NotFound));
    }
}
