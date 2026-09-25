//! Shared controls: dB fader, stereo peak meter, small toggle buttons, dB formatting.

use egui::{Color32, Rect, Rounding, Sense, Stroke, Ui, Vec2};

pub const FADER_HEIGHT: f32 = 150.0;
pub const METER_FLOOR_DB: f32 = -60.0;
pub const STRIP_WIDTH: f32 = 168.0;
pub const PLAYER_WIDTH: f32 = 330.0;
/// Width of the small parameter sliders inside FX and tone sections.
pub const DETAIL_SLIDER_WIDTH: f32 = 90.0;
/// Width of the music seek and volume sliders in the wider PLAYER column.
pub const PLAYER_SLIDER_WIDTH: f32 = 200.0;
/// Smallest hit box for any on/off button.
pub const TOGGLE_MIN_SIZE: Vec2 = Vec2::new(36.0, 24.0);
/// Width for the small pickers in the top bar (record bus, hotkey strip).
pub const SMALL_COMBO_WIDTH: f32 = 90.0;

pub const COLOR_ACTIVE: Color32 = Color32::from_rgb(0x2e, 0xa0, 0x43);
pub const COLOR_MUTE: Color32 = Color32::from_rgb(0xc9, 0x3a, 0x3a);
pub const COLOR_SOLO: Color32 = Color32::from_rgb(0xd9, 0xa4, 0x1c);
/// A pad or slot that has something loaded into it.
pub const COLOR_ASSIGNED: Color32 = Color32::from_rgb(0x2b, 0x4c, 0x7e);
pub const COLOR_ERROR: Color32 = Color32::LIGHT_RED;

/// Secondary text: hints, device status, section captions.
pub fn hint(ui: &mut Ui, text: impl Into<String>) {
    ui.label(egui::RichText::new(text.into()).weak().small());
}

/// Error text, same style everywhere it appears.
pub fn error_label(ui: &mut Ui, text: &str) {
    ui.colored_label(COLOR_ERROR, text);
}

/// A vertical dB fader with the value shown underneath.
pub fn fader(ui: &mut Ui, value: &mut f32, range: std::ops::RangeInclusive<f32>) -> bool {
    let mut changed = false;
    ui.vertical(|ui| {
        ui.spacing_mut().slider_width = FADER_HEIGHT;
        let response = ui.add(egui::Slider::new(value, range).vertical().show_value(false));
        if response.double_clicked() {
            *value = 0.0;
            changed = true;
        }
        changed |= response.changed();
        ui.label(format!("{value:+.1} dB"));
    });
    changed
}

fn meter_fraction(peak: f32) -> f32 {
    let db = crate::gain_to_db(peak);
    ((db - METER_FLOOR_DB) / -METER_FLOOR_DB).clamp(0.0, 1.0)
}

/// Two vertical bars, green to red, sized to sit beside a fader.
pub fn meter(ui: &mut Ui, peaks: [f32; 2]) {
    let size = Vec2::new(16.0, FADER_HEIGHT);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(2.0), Color32::from_gray(30));
    let bar_w = (rect.width() - 3.0) / 2.0;
    for (ch, peak) in peaks.iter().enumerate() {
        let frac = meter_fraction(*peak);
        let x0 = rect.left() + 1.0 + ch as f32 * (bar_w + 1.0);
        let top = rect.bottom() - rect.height() * frac;
        let bar = Rect::from_min_max(egui::pos2(x0, top), egui::pos2(x0 + bar_w, rect.bottom()));
        let color = if *peak >= 0.98 {
            COLOR_MUTE
        } else if frac > 0.8 {
            COLOR_SOLO
        } else {
            COLOR_ACTIVE
        };
        painter.rect_filled(bar, Rounding::ZERO, color);
    }
    painter.rect_stroke(rect, Rounding::same(2.0), Stroke::new(1.0_f32, Color32::from_gray(60)));
}

/// A compact on/off button that lights up in `color` when on.
pub fn toggle(ui: &mut Ui, on: &mut bool, label: &str, color: Color32) -> bool {
    let fill = if *on { color } else { ui.visuals().widgets.inactive.bg_fill };
    let text = if *on { Color32::WHITE } else { ui.visuals().text_color() };
    let button = egui::Button::new(egui::RichText::new(label).color(text).small()).fill(fill).min_size(TOGGLE_MIN_SIZE);
    let response = ui.add(button);
    if response.clicked() {
        *on = !*on;
        return true;
    }
    false
}

/// Device picker with a "None" entry. Returns true when the choice changed.
pub fn device_combo(ui: &mut Ui, id: impl std::hash::Hash, selected: &mut Option<String>, names: &[String]) -> bool {
    let mut changed = false;
    let label = selected.as_deref().unwrap_or("— none —");
    egui::ComboBox::from_id_salt(id).width(STRIP_WIDTH - 20.0).selected_text(shorten(label, 22)).show_ui(ui, |ui| {
        if ui.selectable_label(selected.is_none(), "— none —").clicked() {
            *selected = None;
            changed = true;
        }
        for name in names {
            let is_sel = selected.as_deref() == Some(name.as_str());
            if ui.selectable_label(is_sel, name).clicked() {
                *selected = Some(name.clone());
                changed = true;
            }
        }
    });
    changed
}

pub fn shorten(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let head: String = s.chars().take(max - 1).collect();
        format!("{head}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meter_fraction_maps_db_floor_to_zero_and_full_scale_to_one() {
        assert_eq!(meter_fraction(0.0), 0.0);
        assert!((meter_fraction(1.0) - 1.0).abs() < 1e-6);
        assert!((meter_fraction(crate::db_to_gain(-30.0)) - 0.5).abs() < 1e-3);
    }

    #[test]
    fn shorten_adds_ellipsis_only_when_needed() {
        assert_eq!(shorten("short", 10), "short");
        assert_eq!(shorten("a very long device name", 8), "a very …");
    }
}
