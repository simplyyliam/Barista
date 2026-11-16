//! Where each item goes in the bar (ADR 0028): saved order, pinned or not, bar or overflow.
//! Pure Rust with no registry or Windows in it, so every rule is tested with plain values.

use crate::settings::ItemLayout;
use barista_protocol::ItemSource;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;

/// Where the UI draws an item: in the bar, or in the overflow list (a panel behind an arrow).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum Placement {
    Bar,
    Overflow,
}

/// What the layout needs to know about one item. `width` is its last known width in the bar
/// (logical px), or a guess.
pub struct Entry {
    /// Stable across runs: `app:<app>/<item>`, `tray:...`.
    pub stable: String,
    pub source: ItemSource,
    pub width: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    /// Index into the entries the caller gave.
    pub index: usize,
    pub pinned: bool,
    pub placement: Placement,
}

/// Client items start pinned (an app made them on purpose); tray icons start unpinned (there can be
/// dozens). A saved choice always wins.
pub fn is_pinned(layout: &ItemLayout, stable: &str, source: ItemSource) -> bool {
    layout
        .pinned
        .get(stable)
        .copied()
        .unwrap_or(source == ItemSource::App)
}

/// Every entry in display order with its placement. `space` is the room the bar has for items
/// (logical px) and the gap between two of them; `None` = unlimited, so nothing overflows for lack
/// of room. Items not in the saved order keep their arrival order after the ones that are.
pub fn arrange(entries: &[Entry], layout: &ItemLayout, space: Option<(f64, f64)>) -> Vec<Placed> {
    let position: HashMap<&str, usize> = layout
        .order
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();
    let mut placed: Vec<Placed> = entries
        .iter()
        .enumerate()
        .map(|(index, e)| {
            let pinned = is_pinned(layout, &e.stable, e.source);
            let placement = if layout.collapsed && !pinned {
                Placement::Overflow
            } else {
                Placement::Bar
            };
            Placed { index, pinned, placement }
        })
        .collect();
    // A stable sort: ties (all the unknown items) stay in arrival order.
    placed.sort_by_key(|p| {
        position
            .get(entries[p.index].stable.as_str())
            .copied()
            .unwrap_or(usize::MAX)
    });
    if let Some((room, gap)) = space {
        fit(&mut placed, entries, room, gap);
    }
    placed
}

/// Moves items to overflow until the rest fit: the left-most unpinned one first, then the left-most
/// pinned one.
fn fit(placed: &mut [Placed], entries: &[Entry], room: f64, gap: f64) {
    let used = |placed: &[Placed]| {
        let widths: Vec<f64> = placed
            .iter()
            .filter(|p| p.placement == Placement::Bar)
            .map(|p| entries[p.index].width)
            .collect();
        widths.iter().sum::<f64>() + gap * widths.len().saturating_sub(1) as f64
    };
    while used(placed) > room {
        let victim = placed
            .iter()
            .position(|p| p.placement == Placement::Bar && !p.pinned)
            .or_else(|| placed.iter().position(|p| p.placement == Placement::Bar));
        match victim {
            Some(i) => placed[i].placement = Placement::Overflow,
            None => break,
        }
    }
}

/// The new saved order after the user arranged `given` (the ids they can see, left to right).
/// Only those ids change places, and only among the slots they already held, so items that are not
/// running right now keep their spot. Ids never seen before join at the end first.
pub fn merge_order(saved: &[String], given: &[String]) -> Vec<String> {
    let mut order = saved.to_vec();
    for id in given {
        if !order.contains(id) {
            order.push(id.clone());
        }
    }
    let slots: Vec<usize> = order
        .iter()
        .enumerate()
        .filter(|(_, id)| given.contains(id))
        .map(|(i, _)| i)
        .collect();
    for (slot, id) in slots.into_iter().zip(given) {
        order[slot] = id.clone();
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(stable: &str, source: ItemSource, width: f64) -> Entry {
        Entry { stable: stable.into(), source, width }
    }
    fn ids(entries: &[Entry], placed: &[Placed]) -> Vec<String> {
        placed.iter().map(|p| entries[p.index].stable.clone()).collect()
    }
    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }
    fn open() -> ItemLayout {
        ItemLayout { collapsed: false, ..Default::default() }
    }

    #[test]
    fn defaults_pin_client_items_and_not_tray_icons() {
        let layout = ItemLayout::default();
        assert!(is_pinned(&layout, "app:A/x", ItemSource::App));
        assert!(!is_pinned(&layout, "tray:discord/1", ItemSource::Tray));
        let mut saved = layout;
        saved.pinned.insert("tray:discord/1".into(), true);
        saved.pinned.insert("app:A/x".into(), false);
        assert!(is_pinned(&saved, "tray:discord/1", ItemSource::Tray));
        assert!(!is_pinned(&saved, "app:A/x", ItemSource::App));
    }

    #[test]
    fn unknown_items_come_after_ordered_ones_in_arrival_order() {
        let entries = [
            entry("app:A/new", ItemSource::App, 28.0),
            entry("tray:b/1", ItemSource::Tray, 28.0),
            entry("tray:a/1", ItemSource::Tray, 28.0),
        ];
        let layout = ItemLayout { order: strings(&["tray:a/1", "tray:b/1"]), collapsed: false, ..Default::default() };
        let placed = arrange(&entries, &layout, None);
        assert_eq!(ids(&entries, &placed), ["tray:a/1", "tray:b/1", "app:A/new"]);
    }

    #[test]
    fn collapsed_sends_every_unpinned_item_to_overflow() {
        let entries = [
            entry("app:A/x", ItemSource::App, 28.0),
            entry("tray:a/1", ItemSource::Tray, 28.0),
            entry("tray:b/1", ItemSource::Tray, 28.0),
        ];
        let mut layout = ItemLayout::default(); // collapsed
        layout.pinned.insert("tray:b/1".into(), true);
        let placed = arrange(&entries, &layout, None);
        let in_bar: Vec<_> = placed.iter().filter(|p| p.placement == Placement::Bar).map(|p| entries[p.index].stable.as_str()).collect();
        assert_eq!(in_bar, ["app:A/x", "tray:b/1"]);
        // Expanding brings them back.
        layout.collapsed = false;
        assert!(arrange(&entries, &layout, None).iter().all(|p| p.placement == Placement::Bar));
    }

    #[test]
    fn what_does_not_fit_overflows_leftmost_unpinned_first() {
        let entries = [
            entry("tray:a/1", ItemSource::Tray, 30.0), // unpinned
            entry("app:A/x", ItemSource::App, 30.0),   // pinned
            entry("tray:b/1", ItemSource::Tray, 30.0), // unpinned
        ];
        // 3 x 30 + 2 gaps of 2 = 94. Room for two items (62): the left-most unpinned one goes.
        let placed = arrange(&entries, &open(), Some((62.0, 2.0)));
        let overflow: Vec<_> = placed.iter().filter(|p| p.placement == Placement::Overflow).map(|p| entries[p.index].stable.as_str()).collect();
        assert_eq!(overflow, ["tray:a/1"]);
        // Room for one: the other unpinned goes, then (only if still needed) pinned ones.
        let placed = arrange(&entries, &open(), Some((30.0, 2.0)));
        let bar: Vec<_> = placed.iter().filter(|p| p.placement == Placement::Bar).map(|p| entries[p.index].stable.as_str()).collect();
        assert_eq!(bar, ["app:A/x"]);
        // No room at all: everything overflows and the loop ends.
        let placed = arrange(&entries, &open(), Some((5.0, 2.0)));
        assert!(placed.iter().all(|p| p.placement == Placement::Overflow));
    }

    #[test]
    fn no_space_limit_means_nothing_overflows_for_lack_of_room() {
        let entries: Vec<Entry> = (0..40).map(|i| entry(&format!("tray:a/{i}"), ItemSource::Tray, 28.0)).collect();
        assert!(arrange(&entries, &open(), None).iter().all(|p| p.placement == Placement::Bar));
    }

    #[test]
    fn merge_reorders_within_the_slots_the_given_ids_held() {
        let saved = strings(&["a", "b", "c", "d"]);
        assert_eq!(merge_order(&saved, &strings(&["c", "a"])), ["c", "b", "a", "d"]);
        // Not running now (b, d) keeps its place.
        assert_eq!(merge_order(&saved, &strings(&["d", "c"])), ["a", "b", "d", "c"]);
    }

    #[test]
    fn merge_appends_new_ids_and_places_them_as_asked() {
        assert_eq!(merge_order(&[], &strings(&["x", "y", "z"])), ["x", "y", "z"]);
        assert_eq!(merge_order(&strings(&["a"]), &strings(&["x", "a"])), ["x", "a"]);
        assert_eq!(merge_order(&strings(&["a", "b"]), &strings(&["a"])), ["a", "b"]);
    }
}
