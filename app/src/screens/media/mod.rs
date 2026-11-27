//! Media screen — pixel port of `Media.vue` (v0.0.6 reference): URL card with
//! browser-cookie tabs + format scan / quality picker modal, and the three
//! helper-card lists (binaries, JS runtimes, media merger).
//!
//! Layout split mirrors the old template's two halves:
//! - [`downloader`]: the `.media-downloader-card` + `.nd-overlay` scan modal,
//!   the scan/start/download state machine and yt-log stream.
//! - [`tools`]: the `.media-binaries-card` stacks (yt-dlp/EJS, JS runtimes,
//!   FFmpeg mergers) with their install/uninstall/select flows.

pub mod downloader;
pub mod tools;

use super::Ctx;
use crate::state::UiState;

/// All Media-screen state. Each half owns its own sub-state so the ports stay
/// independent (the old component had one big `setup()`).
#[derive(Default)]
pub struct MediaUi {
    pub dl: downloader::DlUi,
    pub tools: tools::ToolsUi,
}

/// `.nd-card` surface: `var(--bg-card)`, 1px `var(--border)`, 14px radius and
/// `var(--shadow-sm)` (`0 2px 8px rgba(0,0,0,.25)`).
pub fn card_frame() -> egui::Frame {
    let p = crate::ui::widgets::pal();
    egui::Frame::new()
        .fill(p.bg_card)
        .stroke(egui::Stroke::new(1.0, p.border))
        .corner_radius(egui::CornerRadius::same(14))
        .shadow(egui::epaint::Shadow {
            offset: [0, 2],
            blur: 8,
            spread: 0,
            color: egui::Color32::from_black_alpha(64),
        })
}

pub(super) use crate::ui::shade::{grad135, soft_shadow};

/// `.media-page` — `width: min(720px, 100%); margin: 0 auto; padding: 4px 0
/// 20px; gap: 14px` (the `content-area` scroll + 20px sides come from the
/// shell, exactly like the web app).
pub fn show(state: &mut UiState, ctx: &Ctx, ui: &mut egui::Ui) {
    // `.media-page` is `margin: 0 auto` *inside* the shell's page scroll, so the
    // scroll region is the whole content pane and only the cards are 720px wide.
    // (Narrowing the ScrollArea to 720px made the wheel dead outside that column
    // and pinned the bar to the card edge — a box, not a page.)
    let vrect = super::page_viewport(ui);
    ui.scope_builder(egui::UiBuilder::new().max_rect(vrect), |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                let full = ui.available_width() - super::PAGE_GUTTER;
                let w = 720.0_f32.min(full);
                let pad = ((full - w) * 0.5).max(0.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                    ui.add_space(pad);
                    ui.vertical(|ui| {
                        ui.set_width(w);
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 14.0);
                        ui.add_space(4.0);
                        downloader::card(state, ctx, ui, w);
                        tools::cards(state, ctx, ui, w);
                        ui.add_space(20.0);
                    });
                });
            });
    });
    // `.nd-overlay` modal lives at the viewport level (fixed inset:0).
    downloader::modal(state, ctx);
}

/// Routes the backend events the Media screen listens to (`yt-*`, installer
/// progress, runtime/merger installs). Returns `true` when consumed so the
/// generic debug log doesn't echo it.
pub fn apply_event(state: &mut UiState, name: &str, payload: &serde_json::Value) -> bool {
    if downloader::apply_event(state, name, payload) {
        return true;
    }
    tools::apply_event(state, name, payload)
}
