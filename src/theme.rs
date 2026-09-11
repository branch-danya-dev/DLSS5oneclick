//! Palette, fonts and widget style — dark desktop utility look.
//!
//! Fonts are bundled (all SIL OFL, licences in assets/fonts):
//! Sora Bold for branding titles, IBM Plex Sans for UI (Cyrillic),
//! JetBrains Mono for paths and the log.

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle,
};
use std::sync::Arc;

// ── semantic surfaces ──────────────────────────────────────────────
pub const BG: Color32 = Color32::from_rgb(0x0b, 0x0f, 0x17);
pub const SURFACE: Color32 = Color32::from_rgb(0x11, 0x17, 0x22);
pub const SURFACE_ALT: Color32 = Color32::from_rgb(0x16, 0x1e, 0x2b);
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(0x1b, 0x25, 0x34);
#[allow(dead_code)]
pub const SURFACE_ELEVATED: Color32 = SURFACE_ALT;

// ── borders ────────────────────────────────────────────────────────
pub const BORDER: Color32 = Color32::from_rgb(0x26, 0x32, 0x46);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(0x32, 0x40, 0x58);
pub const BORDER_ACTIVE: Color32 = Color32::from_rgb(0x42, 0x63, 0xeb);

// ── text ───────────────────────────────────────────────────────────
pub const TEXT: Color32 = Color32::from_rgb(0xf1, 0xf4, 0xf8);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0xaa, 0xb4, 0xc3);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x69, 0x75, 0x87);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x52, 0x5c, 0x6c);

// ── accents ────────────────────────────────────────────────────────
pub const PRIMARY: Color32 = Color32::from_rgb(0x4f, 0x6b, 0xff);
pub const PRIMARY_HOVER: Color32 = Color32::from_rgb(0x61, 0x7a, 0xff);
/// Soft primary wash (opaque stand-in for translucent fill).
pub const PRIMARY_SOFT: Color32 = Color32::from_rgb(0x18, 0x22, 0x3c);
pub const SUCCESS: Color32 = Color32::from_rgb(0x4e, 0xcb, 0x71);
pub const SUCCESS_SOFT: Color32 = Color32::from_rgb(0x14, 0x28, 0x1e);
pub const WARNING: Color32 = Color32::from_rgb(0xe6, 0xa8, 0x3c);
pub const WARNING_SOFT: Color32 = Color32::from_rgb(0x2a, 0x22, 0x14);
pub const DANGER: Color32 = Color32::from_rgb(0xff, 0x62, 0x62);
pub const DANGER_SOFT: Color32 = Color32::from_rgb(0x2a, 0x16, 0x18);

// ── radii ──────────────────────────────────────────────────────────
pub const CARD_RADIUS: u8 = 12;
pub const CONTROL_RADIUS: u8 = 8;
pub const CHIP_RADIUS: u8 = 6;

/// Max width of the centred content column (logical px).
pub const CONTENT_MAX_WIDTH: f32 = 1480.0;
pub const CONTENT_PAD_X: f32 = 20.0;
pub const CONTENT_PAD_Y: f32 = 12.0;

// ── aliases kept so existing gui code compiles during the redesign ──
pub const PANEL: Color32 = SURFACE;
pub const HEADER: Color32 = SURFACE;
pub const TILE: Color32 = SURFACE_ALT;
pub const TEXT_SOFT: Color32 = TEXT_SECONDARY;
pub const TEXT_OFF: Color32 = TEXT_SECONDARY;
pub const RING_OFF: Color32 = BORDER_STRONG;
pub const ACCENT: Color32 = PRIMARY;
pub const WARN: Color32 = WARNING;

pub const SORA: &str = "sora";
pub const PLEX_MEDIUM: &str = "plex-medium";
pub const PLEX_SEMIBOLD: &str = "plex-semibold";

pub fn sora(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(SORA.into()))
}
pub fn plex(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}
pub fn plex_medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(PLEX_MEDIUM.into()))
}
pub fn plex_semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(PLEX_SEMIBOLD.into()))
}
pub fn mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

pub fn card_rounding() -> CornerRadius {
    CornerRadius::same(CARD_RADIUS)
}
pub fn control_rounding() -> CornerRadius {
    CornerRadius::same(CONTROL_RADIUS)
}
pub fn chip_rounding() -> CornerRadius {
    CornerRadius::same(CHIP_RADIUS)
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let add = |fonts: &mut FontDefinitions, key: &str, bytes: &'static [u8]| {
        fonts
            .font_data
            .insert(key.to_owned(), Arc::new(FontData::from_static(bytes)));
    };
    add(
        &mut fonts,
        "plex",
        include_bytes!("../assets/fonts/IBMPlexSans-Regular.ttf"),
    );
    add(
        &mut fonts,
        PLEX_MEDIUM,
        include_bytes!("../assets/fonts/IBMPlexSans-Medium.ttf"),
    );
    add(
        &mut fonts,
        PLEX_SEMIBOLD,
        include_bytes!("../assets/fonts/IBMPlexSans-SemiBold.ttf"),
    );
    add(
        &mut fonts,
        SORA,
        include_bytes!("../assets/fonts/Sora-Bold.ttf"),
    );
    add(
        &mut fonts,
        "jbmono",
        include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf"),
    );

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "plex".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "jbmono".to_owned());
    for name in [SORA, PLEX_MEDIUM, PLEX_SEMIBOLD] {
        // Fall back to Plex for glyphs the face lacks (Sora has little Cyrillic).
        let mut chain = vec![name.to_owned()];
        chain.extend(fonts.families[&FontFamily::Proportional].iter().cloned());
        fonts.families.insert(FontFamily::Name(name.into()), chain);
    }
    ctx.set_fonts(fonts);

    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Heading, sora(18.0)),
            (TextStyle::Body, plex(13.0)),
            (TextStyle::Button, plex_medium(13.0)),
            (TextStyle::Small, plex(11.5)),
            (TextStyle::Monospace, mono(12.0)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        style.spacing.interact_size = egui::vec2(44.0, 30.0);
        style.spacing.indent = 16.0;

        let v = &mut style.visuals;
        v.dark_mode = true;
        v.override_text_color = Some(TEXT);
        v.panel_fill = SURFACE;
        v.window_fill = SURFACE;
        v.window_stroke = Stroke::new(1.0, BORDER);
        v.window_corner_radius = CornerRadius::same(CARD_RADIUS);
        v.extreme_bg_color = BG;
        v.faint_bg_color = SURFACE_ALT;
        v.code_bg_color = SURFACE_ALT;
        v.hyperlink_color = PRIMARY_HOVER;
        v.error_fg_color = DANGER;
        v.warn_fg_color = WARNING;
        v.selection.bg_fill = PRIMARY_SOFT;
        v.selection.stroke = Stroke::new(1.0, PRIMARY);

        let w = &mut v.widgets;
        for wv in [
            &mut w.noninteractive,
            &mut w.inactive,
            &mut w.hovered,
            &mut w.active,
            &mut w.open,
        ] {
            wv.corner_radius = CornerRadius::same(CONTROL_RADIUS);
            wv.expansion = 0.0;
        }
        w.noninteractive.bg_fill = SURFACE;
        w.noninteractive.weak_bg_fill = SURFACE;
        w.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
        w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
        w.inactive.bg_fill = SURFACE_ALT;
        w.inactive.weak_bg_fill = SURFACE_ALT;
        w.inactive.bg_stroke = Stroke::new(1.0, BORDER);
        w.inactive.fg_stroke = Stroke::new(1.0, TEXT_SECONDARY);
        w.hovered.bg_fill = SURFACE_HOVER;
        w.hovered.weak_bg_fill = SURFACE_HOVER;
        w.hovered.bg_stroke = Stroke::new(1.0, BORDER_STRONG);
        w.hovered.fg_stroke = Stroke::new(1.0, TEXT);
        w.active.bg_fill = SURFACE_HOVER;
        w.active.weak_bg_fill = SURFACE_HOVER;
        w.active.bg_stroke = Stroke::new(1.0, BORDER_ACTIVE);
        w.active.fg_stroke = Stroke::new(1.0, TEXT);
        w.open.bg_fill = SURFACE_ALT;
        w.open.weak_bg_fill = SURFACE_ALT;
        w.open.bg_stroke = Stroke::new(1.0, BORDER_STRONG);
        w.open.fg_stroke = Stroke::new(1.0, TEXT);
    });
}
