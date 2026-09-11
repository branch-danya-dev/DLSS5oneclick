//! Small reusable visual helpers for the redesign. No business logic.

use crate::theme::{self as t};
use eframe::egui::{
    self, Align, Color32, Frame, Layout, Margin, RichText, Sense, Stroke, Vec2,
};

/// Centre a column capped at [`t::CONTENT_MAX_WIDTH`].
pub fn content_column(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    let avail = ui.available_width();
    let width = avail.min(t::CONTENT_MAX_WIDTH);
    let side = ((avail - width) * 0.5).max(0.0);
    ui.horizontal(|ui| {
        if side > 0.0 {
            ui.add_space(side);
        }
        ui.allocate_ui_with_layout(
            Vec2::new(width, ui.available_height()),
            Layout::top_down(Align::Min),
            |ui| {
                ui.set_min_width(width);
                ui.set_max_width(width);
                add_contents(ui);
            },
        );
    });
}

pub fn card_frame() -> Frame {
    Frame::new()
        .fill(t::SURFACE)
        .stroke(Stroke::new(1.0, t::BORDER))
        .corner_radius(t::card_rounding())
        .inner_margin(Margin::same(14))
}

pub fn card_elevated() -> Frame {
    Frame::new()
        .fill(t::SURFACE_ALT)
        .stroke(Stroke::new(1.0, t::BORDER))
        .corner_radius(t::card_rounding())
        .inner_margin(Margin::same(14))
}

pub fn section_card(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    card_frame().show(ui, |ui| {
        ui.label(
            RichText::new(title)
                .font(t::plex_semibold(13.5))
                .color(t::TEXT),
        );
        ui.add_space(8.0);
        add_contents(ui);
    });
}

pub fn page_title(ui: &mut egui::Ui, title: &str, subtitle: Option<&str>) {
    ui.label(RichText::new(title).font(t::sora(22.0)).color(t::TEXT));
    if let Some(sub) = subtitle {
        ui.add_space(2.0);
        ui.label(
            RichText::new(sub)
                .font(t::plex(13.0))
                .color(t::TEXT_SECONDARY),
        );
    }
    ui.add_space(12.0);
}

#[derive(Clone, Copy)]
pub enum ChipTone {
    Neutral,
    Primary,
    Success,
    Warning,
    Danger,
}

pub fn chip(ui: &mut egui::Ui, text: &str, tone: ChipTone) {
    let (fg, bg, border) = match tone {
        ChipTone::Neutral => (t::TEXT_SECONDARY, t::SURFACE_ALT, t::BORDER),
        ChipTone::Primary => (t::PRIMARY_HOVER, t::PRIMARY_SOFT, t::BORDER_ACTIVE),
        ChipTone::Success => (t::SUCCESS, t::SUCCESS_SOFT, Color32::from_rgb(0x2e, 0x6b, 0x45)),
        ChipTone::Warning => (t::WARNING, t::WARNING_SOFT, Color32::from_rgb(0x7a, 0x5a, 0x22)),
        ChipTone::Danger => (t::DANGER, t::DANGER_SOFT, Color32::from_rgb(0x7a, 0x2e, 0x2e)),
    };
    Frame::new()
        .fill(bg)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(t::chip_rounding())
        .inner_margin(Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).font(t::plex_medium(11.0)).color(fg));
        });
}

pub fn primary_button(text: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(
        RichText::new(text.into())
            .font(t::plex_semibold(13.5))
            .color(Color32::WHITE),
    )
    .fill(t::PRIMARY)
    .stroke(Stroke::NONE)
    .corner_radius(t::control_rounding())
    .min_size(Vec2::new(160.0, 40.0))
}

pub fn secondary_button(text: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(
        RichText::new(text.into())
            .font(t::plex_medium(13.0))
            .color(t::TEXT_SECONDARY),
    )
    .fill(Color32::TRANSPARENT)
    .stroke(Stroke::new(1.0, t::BORDER_STRONG))
    .corner_radius(t::control_rounding())
    .min_size(Vec2::new(96.0, 40.0))
}

pub fn ghost_button(text: impl Into<String>) -> egui::Button<'static> {
    egui::Button::new(
        RichText::new(text.into())
            .font(t::plex_medium(12.5))
            .color(t::TEXT_SECONDARY),
    )
    .fill(Color32::TRANSPARENT)
    .stroke(Stroke::NONE)
    .corner_radius(t::control_rounding())
}

pub fn nav_tab(ui: &mut egui::Ui, label: &str, active: bool, enabled: bool) -> egui::Response {
    let (fill, stroke, fg) = if active {
        (t::PRIMARY_SOFT, Stroke::new(1.0, t::BORDER_ACTIVE), t::TEXT)
    } else {
        (
            Color32::TRANSPARENT,
            Stroke::new(1.0, Color32::TRANSPARENT),
            t::TEXT_SECONDARY,
        )
    };
    let btn = egui::Button::new(RichText::new(label).font(t::plex_medium(13.0)).color(fg))
        .fill(fill)
        .stroke(stroke)
        .corner_radius(t::control_rounding())
        .min_size(Vec2::new(88.0, 34.0));
    ui.add_enabled(enabled, btn)
}

pub fn summary_stat(ui: &mut egui::Ui, label: &str, value: impl Into<String>) {
    Frame::new()
        .fill(t::SURFACE_ALT)
        .stroke(Stroke::new(1.0, t::BORDER))
        .corner_radius(t::control_rounding())
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_min_width(88.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(label)
                        .font(t::plex(11.0))
                        .color(t::TEXT_MUTED),
                );
                ui.label(
                    RichText::new(value.into())
                        .font(t::plex_semibold(18.0))
                        .color(t::TEXT),
                );
            });
        });
}

pub fn info_banner(ui: &mut egui::Ui, title: &str, body: &str) -> bool {
    let mut dismiss = false;
    Frame::new()
        .fill(t::PRIMARY_SOFT)
        .stroke(Stroke::new(1.0, t::BORDER_ACTIVE))
        .corner_radius(t::control_rounding())
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(title)
                        .font(t::plex_semibold(12.5))
                        .color(t::PRIMARY_HOVER),
                );
                ui.label(
                    RichText::new(body)
                        .font(t::plex(12.5))
                        .color(t::TEXT_SECONDARY),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(ghost_button("×").min_size(Vec2::new(28.0, 24.0)))
                        .clicked()
                    {
                        dismiss = true;
                    }
                });
            });
        });
    dismiss
}

pub fn selectable_option(
    ui: &mut egui::Ui,
    selected: bool,
    enabled: bool,
    title: &str,
    detail: &str,
) -> egui::Response {
    let (fill, stroke) = if selected {
        (t::PRIMARY_SOFT, Stroke::new(1.5, t::BORDER_ACTIVE))
    } else if enabled {
        (t::SURFACE_ALT, Stroke::new(1.0, t::BORDER))
    } else {
        (t::SURFACE, Stroke::new(1.0, t::BORDER))
    };
    let title_c = if enabled { t::TEXT } else { t::TEXT_MUTED };
    let detail_c = if enabled {
        t::TEXT_SECONDARY
    } else {
        t::TEXT_DIM
    };
    let inner = Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(t::control_rounding())
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(title)
                    .font(t::plex_semibold(13.0))
                    .color(title_c),
            );
            ui.label(RichText::new(detail).font(t::plex(11.5)).color(detail_c));
        });
    let resp = inner.response.interact(if enabled {
        Sense::click()
    } else {
        Sense::hover()
    });
    if enabled {
        resp
    } else {
        resp.on_hover_text(detail)
    }
}

pub fn truncate_path(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        return s.to_owned();
    }
    let keep = max_chars.saturating_sub(1);
    let start = keep / 2;
    let end = keep - start;
    let head: String = chars.iter().take(start).collect();
    let tail: String = chars.iter().rev().take(end).collect::<Vec<_>>().into_iter().rev().collect();
    format!("{head}…{tail}")
}
