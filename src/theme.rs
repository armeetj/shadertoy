use eframe::egui;

// Gruvbox Material Dark Hard
pub const BG: egui::Color32 = egui::Color32::from_rgb(0x1d, 0x20, 0x21);
pub const BG1: egui::Color32 = egui::Color32::from_rgb(0x28, 0x28, 0x28);
pub const FG: egui::Color32 = egui::Color32::from_rgb(0xd4, 0xbe, 0x98);
pub const RED: egui::Color32 = egui::Color32::from_rgb(0xea, 0x69, 0x62);
pub const GREEN: egui::Color32 = egui::Color32::from_rgb(0xa9, 0xb6, 0x65);
pub const YELLOW: egui::Color32 = egui::Color32::from_rgb(0xd8, 0xa6, 0x57);
pub const BLUE: egui::Color32 = egui::Color32::from_rgb(0x7d, 0xae, 0xa3);
pub const PURPLE: egui::Color32 = egui::Color32::from_rgb(0xd3, 0x86, 0x9b);
pub const AQUA: egui::Color32 = egui::Color32::from_rgb(0x89, 0xb4, 0x82);
pub const ORANGE: egui::Color32 = egui::Color32::from_rgb(0xe7, 0x8a, 0x4e);
pub const GRAY: egui::Color32 = egui::Color32::from_rgb(0x92, 0x83, 0x74);

pub fn apply_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;

    v.panel_fill = BG;
    v.window_fill = BG;
    v.extreme_bg_color = BG;
    v.faint_bg_color = BG;

    v.window_corner_radius = egui::CornerRadius::ZERO;
    v.menu_corner_radius = egui::CornerRadius::ZERO;
    v.widgets.noninteractive.corner_radius = egui::CornerRadius::ZERO;
    v.widgets.inactive.corner_radius = egui::CornerRadius::ZERO;
    v.widgets.hovered.corner_radius = egui::CornerRadius::ZERO;
    v.widgets.active.corner_radius = egui::CornerRadius::ZERO;
    v.widgets.open.corner_radius = egui::CornerRadius::ZERO;

    let no_stroke = egui::Stroke::NONE;
    v.widgets.noninteractive.bg_stroke = no_stroke;
    v.widgets.inactive.bg_stroke = no_stroke;
    v.widgets.hovered.bg_stroke = no_stroke;
    v.widgets.active.bg_stroke = no_stroke;

    v.widgets.noninteractive.bg_fill = BG;
    v.widgets.inactive.bg_fill = BG;
    v.widgets.hovered.bg_fill = BG;
    v.widgets.active.bg_fill = BG1;

    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, FG);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, FG);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, FG);
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, FG);

    v.selection.bg_fill = egui::Color32::from_rgb(0x3c, 0x38, 0x36);
    v.selection.stroke = egui::Stroke::new(0.0, egui::Color32::TRANSPARENT);

    style.spacing.item_spacing = egui::vec2(0.0, 0.0);

    ctx.set_style(style);
}
