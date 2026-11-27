//! Shared placeholder for screens arriving in later port milestones (Home
//! P2, Settings P3, Media P4, Addon P5). Draws stat cards / coming-soon text
//! over the MoonKit look until the real ports land.

use crate::screens::Ctx;
use crate::state::UiState;
use crate::ui::widgets as mk;
use egui::{vec2, RichText};

pub fn show(ui_state: &mut UiState, _ctx: &Ctx, ui: &mut egui::Ui, title: &str, stat_rows: &[(&str, usize)]) {
    mk::gap(ui, 12.0);
    if !stat_rows.is_empty() {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
            for (label, n) in stat_rows {
                mk::card(ui, |ui| {
                    ui.set_min_size(vec2(150.0, 74.0));
                    ui.label(
                        RichText::new(n.to_string())
                            .heading()
                            .color(mk::pal().accent),
                    );
                    ui.label(RichText::new(*label).weak().small());
                });
            }
        });
    } else {
        mk::card(ui, |ui| {
            ui.set_min_height(120.0);
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                ui.label(RichText::new(title).heading());
                ui.label(
                    RichText::new(format!(
                        "{} tab — full port queued (next milestone)",
                        title
                    ))
                    .weak(),
                );
            });
        });
    }
    let _ = ui_state;
}
