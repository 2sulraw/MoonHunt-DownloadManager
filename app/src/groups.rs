//! `useGroups.ts` port: download groups (colour, members, collapse state,
//! per-group speed/download limits). The old app kept the array in
//! `localStorage['moonhunt_groups']`; here the same JSON blob lives in the
//! `settings` table under the same key, so nothing else changes shape.

use crate::state::{chrono_now_ms, UiState};
use egui::Color32;
use serde::{Deserialize, Serialize};

/// `groupColorOptions` — 16 swatches, in the reference order.
pub const GROUP_COLORS: [&str; 16] = [
    "#ef5350", "#e91e63", "#ab47bc", "#7e57c2", "#5c6bc0", "#42a5f5", "#26c6da", "#26a69a",
    "#66bb6a", "#9ccc65", "#d4e157", "#ffca28", "#ffa726", "#ff7043", "#8d6e63", "#78909c",
];

const STORAGE_KEY: &str = "moonhunt_groups";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DownloadGroup {
    pub id: String,
    pub name: String,
    pub color: String,
    pub member_gids: Vec<String>,
    pub on_complete: String,
    pub collapsed: bool,
    /// Per-group speed cap in KB/s; 0 = unlimited.
    pub speed_limit: u64,
    pub download_limit_enabled: bool,
    pub download_limit_bytes: u64,
    pub download_limit_used_bytes: u64,
    /// Epoch ms the group was created (date-added sorting).
    #[serde(rename = "added_at", alias = "addedAt")]
    pub added_at: i64,
}

impl Default for DownloadGroup {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "Group".to_owned(),
            color: "#ef5350".to_owned(),
            member_gids: Vec::new(),
            on_complete: "none".to_owned(),
            collapsed: false,
            speed_limit: 0,
            download_limit_enabled: false,
            download_limit_bytes: 0,
            download_limit_used_bytes: 0,
            added_at: 0,
        }
    }
}

impl DownloadGroup {
    /// `normalizeGroup`: defaults for every missing/invalid field.
    pub fn normalized(mut self) -> Self {
        if self.id.is_empty() {
            self.id = new_id();
        }
        if self.name.is_empty() {
            self.name = "Group".to_owned();
        }
        if self.color.is_empty() {
            self.color = "#ef5350".to_owned();
        }
        if self.on_complete.is_empty() {
            self.on_complete = "none".to_owned();
        }
        if self.added_at <= 0 {
            self.added_at = added_at_from_id(&self.id);
        }
        self
    }

    pub fn rgb(&self) -> (u8, u8, u8) {
        hex_to_rgb(&self.color)
    }

    pub fn egui_color(&self) -> Color32 {
        let (r, g, b) = self.rgb();
        Color32::from_rgb(r, g, b)
    }

    /// `color-mix(in srgb, <color> <pct>%, transparent)` over the page.
    pub fn tint(&self, pct: f32) -> Color32 {
        let (r, g, b) = self.rgb();
        Color32::from_rgba_unmultiplied(r, g, b, (pct * 2.55).round() as u8)
    }

    pub fn has_member(&self, gid: &str) -> bool {
        self.member_gids.iter().any(|g| g == gid)
    }
}

/// `hexToRgb` for the `#rrggbb` values in `GROUP_COLORS` (fallback: red).
pub fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 {
        return (0xef, 0x53, 0x50);
    }
    let parse = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
    (parse(0), parse(2), parse(4))
}

fn added_at_from_id(id: &str) -> i64 {
    id.strip_prefix("g-")
        .and_then(|rest| rest.split('-').next())
        .and_then(|ms| ms.parse::<i64>().ok())
        .unwrap_or(0)
}

fn new_id() -> String {
    let ms = chrono_now_ms();
    // `Math.random().toString(36).slice(2, 7)` equivalent: 5 base-36 chars.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0) as u64;
    let mut n = nanos ^ (ms as u64).rotate_left(17);
    let mut suffix = String::with_capacity(5);
    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    for _ in 0..5 {
        suffix.push(ALPHABET[(n % 36) as usize] as char);
        n /= 36;
    }
    format!("g-{ms}-{suffix}")
}

pub fn load_groups() -> Vec<DownloadGroup> {
    moonhunt_core::db::get_setting_json(STORAGE_KEY)
        .and_then(|v| v.as_array().cloned())
        .map(|rows| {
            rows.into_iter()
                .filter_map(|row| serde_json::from_value::<DownloadGroup>(row).ok())
                .map(DownloadGroup::normalized)
                .collect()
        })
        .unwrap_or_default()
}

pub fn save_groups(groups: &[DownloadGroup]) {
    if let Ok(v) = serde_json::to_value(groups) {
        let _ = moonhunt_core::db::set_setting_json(STORAGE_KEY, &v);
    }
}

impl UiState {
    /// `reloadGroups`.
    pub fn groups_reload(&mut self) {
        self.groups = load_groups();
    }

    pub fn group_by_id(&self, id: &str) -> Option<&DownloadGroup> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// `createGroup` (name, colour, members); the modal owns the limit fields,
    /// which it patches in right after creation.
    pub fn group_create(&mut self, name: &str, color: &str, members: Vec<String>) -> String {
        let name = name.trim();
        let group = DownloadGroup {
            id: new_id(),
            name: if name.is_empty() {
                "New Group".to_owned()
            } else {
                name.to_owned()
            },
            color: color.to_owned(),
            member_gids: members,
            added_at: chrono_now_ms(),
            ..DownloadGroup::default()
        };
        let id = group.id.clone();
        self.groups.push(group);
        save_groups(&self.groups);
        id
    }

    /// `updateGroup(id, patch)` — the call sites only touch name/colour/members
    /// and the limit fields, so the patch is applied through a closure.
    pub fn group_update<F: FnOnce(&mut DownloadGroup)>(&mut self, id: &str, patch: F) {
        if let Some(g) = self.groups.iter_mut().find(|g| g.id == id) {
            patch(g);
        }
        save_groups(&self.groups);
    }

    /// `deleteGroup` — returns the removed group so the caller can clear the
    /// per-member speed caps.
    pub fn group_delete(&mut self, id: &str) -> Option<DownloadGroup> {
        let pos = self.groups.iter().position(|g| g.id == id)?;
        let g = self.groups.remove(pos);
        save_groups(&self.groups);
        Some(g)
    }

    /// `addMember`: apply the group cap first, then record membership once.
    pub fn group_add_member(&mut self, group_id: &str, gid: &str) {
        let Some(g) = self.groups.iter_mut().find(|g| g.id == group_id) else {
            return;
        };
        if g.has_member(gid) {
            return;
        }
        g.member_gids.push(gid.to_owned());
        save_groups(&self.groups);
    }

    pub fn group_remove_member(&mut self, group_id: &str, gid: &str) {
        if let Some(g) = self.groups.iter_mut().find(|g| g.id == group_id) {
            g.member_gids.retain(|m| m != gid);
            save_groups(&self.groups);
        }
    }

    pub fn group_toggle_collapse(&mut self, id: &str) {
        if let Some(g) = self.groups.iter_mut().find(|g| g.id == id) {
            g.collapsed = !g.collapsed;
            save_groups(&self.groups);
        }
    }
}
