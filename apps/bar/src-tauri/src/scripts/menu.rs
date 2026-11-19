//! Builds what the bar draws for a script item: the menu entries, and a table of what each entry
//! does when clicked. The registry only stores the entries; the actions stay with the script
//! manager, so a menu line can never carry a command the UI could change.

use super::parse::{Entry, Node, ShellCommand};
use crate::registry::menu::MenuEntry;
use barista_protocol::MenuItemType;
use std::collections::HashMap;

/// What clicking a menu line does. Only what the parser accepted ends up here, and `href` and
/// `shell` are checked again when the click happens.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Action {
    pub href: Option<String>,
    pub shell: Option<ShellCommand>,
    pub refresh: bool,
    pub builtin: Option<Builtin>,
}

/// The lines Barista adds to an error menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    RunAgain,
    OpenScript,
    Disable,
}

#[derive(Debug, Default)]
pub struct Built {
    pub entries: Vec<MenuEntry>,
    pub actions: HashMap<String, Action>,
}

fn entry(id: Option<String>, kind: MenuItemType, label: &str, color: Option<String>, enabled: bool, submenu: Vec<MenuEntry>) -> MenuEntry {
    MenuEntry {
        id,
        kind,
        label: label.to_string(),
        checked: false,
        enabled,
        accelerator: None,
        role: None,
        icon: None,
        info: false,
        color,
        submenu,
    }
}

fn separator() -> MenuEntry {
    entry(None, MenuItemType::Separator, "", None, true, vec![])
}

/// A short hash of a line's text and actions. Only has to tell "changed" from "same", not resist an attacker.
fn line_hash(label: &str, href: &Option<String>, shell: &Option<ShellCommand>, refresh: bool) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (label, href, shell.as_ref().map(|s| (&s.program, &s.args)), refresh).hash(&mut hasher);
    hasher.finish() as u32
}

/// Ids are `m<position>-<hash>` in drawing order, unique across submenus.
pub fn from_output(nodes: &[Node]) -> Built {
    fn walk(nodes: &[Node], next: &mut usize, actions: &mut HashMap<String, Action>) -> Vec<MenuEntry> {
        nodes
            .iter()
            .map(|node| match node {
                Node::Separator => separator(),
                Node::Item { entry: e, children } => {
                    let Entry { label, color, href, shell, refresh } = e.clone();
                    // The id includes a hash of what the line does. If the next run puts a different
                    // line at this position, a click that was meant for the old one finds nothing
                    // instead of running the new line's `shell` or `href`.
                    let id = format!("m{}-{:08x}", *next, line_hash(&label, &href, &shell, refresh));
                    *next += 1;
                    if href.is_some() || shell.is_some() || refresh {
                        actions.insert(id.clone(), Action { href, shell, refresh, builtin: None });
                    }
                    let submenu = walk(children, next, actions);
                    let kind = if submenu.is_empty() { MenuItemType::Normal } else { MenuItemType::Submenu };
                    // A line with no action and no submenu is only text: the UI draws it without a hover.
                    let info = submenu.is_empty() && !actions.contains_key(&id);
                    MenuEntry { info, ..entry(Some(id), kind, &label, color, true, submenu) }
                }
            })
            .collect()
    }
    let (mut next, mut actions) = (0, HashMap::new());
    let entries = walk(nodes, &mut next, &mut actions);
    Built { entries, actions }
}

/// The error state's menu: "Show error" with the message lines, then Run again, Open script,
/// Disable.
pub fn error_menu(message: &str, detail: &str) -> Built {
    let mut lines: Vec<String> = vec![message.to_string()];
    lines.extend(
        detail
            .lines()
            .map(|l| l.chars().filter(|c| !c.is_control()).take(120).collect::<String>())
            .filter(|l| !l.trim().is_empty())
            .take(9),
    );
    let shown = lines
        .iter()
        .enumerate()
        .map(|(i, l)| entry(Some(format!("e{i}")), MenuItemType::Normal, l, None, false, vec![]))
        .collect();
    let mut actions = HashMap::new();
    let mut item = |id: &str, label: &str, builtin| {
        actions.insert(id.to_string(), Action { builtin: Some(builtin), ..Default::default() });
        entry(Some(id.to_string()), MenuItemType::Normal, label, None, true, vec![])
    };
    let entries = vec![
        entry(Some("show-error".into()), MenuItemType::Submenu, "Show error", None, true, shown),
        separator(),
        item("run-again", "Run again", Builtin::RunAgain),
        item("open-script", "Open script", Builtin::OpenScript),
        item("disable", "Disable", Builtin::Disable),
    ];
    Built { entries, actions }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripts::parse::parse;

    fn build(text: &str) -> Built {
        from_output(&parse(text.as_bytes()).unwrap().menu)
    }

    #[test]
    fn ids_are_unique_across_submenus() {
        let b = build("l\n---\nTop\n--a\n--b\nNext");
        let mut ids = vec![];
        fn collect(entries: &[MenuEntry], ids: &mut Vec<String>) {
            for e in entries {
                if let Some(id) = &e.id {
                    ids.push(id.clone());
                }
                collect(&e.submenu, ids);
            }
        }
        collect(&b.entries, &mut ids);
        let positions: Vec<&str> = ids.iter().map(|id| id.split('-').next().unwrap()).collect();
        assert_eq!(positions, vec!["m0", "m1", "m2", "m3"]);
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len());
        assert_eq!(b.entries[0].kind, MenuItemType::Submenu);
        assert_eq!(b.entries[1].kind, MenuItemType::Normal);
    }

    #[test]
    fn only_lines_with_params_get_actions() {
        let b = build("l\n---\nplain\nweb | href=https://example.com\nrun | shell=taskmgr\nagain | refresh=true");
        let id = |label: &str| b.entries.iter().find(|e| e.label == label).unwrap().id.clone().unwrap();
        assert!(!b.actions.contains_key(&id("plain")));
        assert!(b.actions[&id("web")].href.is_some());
        assert_eq!(b.actions[&id("run")].shell.as_ref().unwrap().program, "taskmgr");
        assert!(b.actions[&id("again")].refresh);
    }

    #[test]
    fn a_different_line_at_the_same_position_gets_a_different_id() {
        // The menu reorders between two runs: a click made on the first menu must not hit the second's line.
        let first = build("l
---
Open | shell=taskmgr");
        let second = build("l
---
Open | shell=calc");
        assert_ne!(first.entries[0].id, second.entries[0].id);
        // The same menu twice keeps its ids, so an open menu doesn't redraw for nothing.
        assert_eq!(first.entries[0].id, build("l
---
Open | shell=taskmgr").entries[0].id);
    }

    #[test]
    fn lines_without_an_action_are_info_and_the_others_are_not() {
        let b = build("l
---
plain
web | href=https://example.com
Top
--child");
        let by = |label: &str| b.entries.iter().find(|e| e.label == label).unwrap();
        assert!(by("plain").info);
        assert!(!by("web").info);
        assert!(!by("Top").info, "a submenu is not info");
        assert!(by("Top").submenu[0].info);
    }

    #[test]
    fn color_reaches_the_entry() {
        let b = build("l\n---\nhot | color=red");
        assert_eq!(b.entries[0].color.as_deref(), Some("red"));
    }

    #[test]
    fn separators_have_no_id() {
        let b = build("l\n---\na\n---\nb");
        assert_eq!(b.entries[1].kind, MenuItemType::Separator);
        assert!(b.entries[1].id.is_none());
    }

    #[test]
    fn the_error_menu_has_the_four_lines() {
        let b = error_menu("The script exited with code 1", "boom\n\n  at x\n");
        let labels: Vec<_> = b.entries.iter().map(|e| e.label.as_str()).collect();
        assert_eq!(labels, vec!["Show error", "", "Run again", "Open script", "Disable"]);
        let shown = &b.entries[0].submenu;
        assert_eq!(shown.len(), 3);
        assert!(shown.iter().all(|e| !e.enabled));
        assert_eq!(b.actions["disable"].builtin, Some(Builtin::Disable));
        assert_eq!(b.actions.len(), 3);
    }

    #[test]
    fn the_error_text_is_capped() {
        let b = error_menu("m", &"line\n".repeat(100));
        assert_eq!(b.entries[0].submenu.len(), 10);
    }
}
