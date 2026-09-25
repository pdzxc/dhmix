//! Design tokens and shared controls, styled after a hardware console (Voicemeeter Potato):
//! charcoal panels, cyan uppercase headings, LED buttons, rotary knobs, ruler faders and
//! segmented meters.

use egui::{Align2, Color32, FontId, Margin, Pos2, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2};
use std::ops::RangeInclusive;

// ---- Layout tokens ---------------------------------------------------------------------------

/// Gap between neighbouring controls; the theme's item spacing.
pub const ITEM_SPACING: f32 = 4.0;
pub const PANEL_PADDING: f32 = 8.0;
pub const PANEL_RADIUS: f32 = 4.0;
pub const CONTROL_RADIUS: f32 = 3.0;
pub const SECTION_GAP: f32 = 6.0;
pub const FADER_WIDTH: f32 = 50.0;
pub const METER_WIDTH: f32 = 22.0;
pub const METER_SEGMENTS: usize = 30;
/// Faders stretch with the window between these.
pub const FADER_MIN_HEIGHT: f32 = 110.0;
pub const FADER_MAX_HEIGHT: f32 = 420.0;
pub const KNOB_SIZE: f32 = 36.0;
/// Outer width of a knob widget.
pub const KNOB_WIDTH: f32 = KNOB_SIZE + 12.0;
pub const LED_HEIGHT: f32 = 22.0;
/// Padding inside an LED; small so short labels fit narrow routing LEDs.
pub const LED_PADDING: f32 = 2.0;
pub const SMALL_COMBO_WIDTH: f32 = 96.0;
/// The "Send to" picker in the Applications window; wide enough for a device plus its meaning.
pub const ROUTE_COMBO_WIDTH: f32 = 230.0;
/// Sliders in the floating fine-tune window, which has room to spare.
pub const FINE_TUNE_SLIDER_WIDTH: f32 = 144.0;
/// Column and row gaps of the Applications table.
pub const TABLE_SPACING: Vec2 = Vec2::new(12.0, 6.0);
pub const PAD_HEIGHT: f32 = 26.0;
pub const PAD_SPACING: f32 = 4.0;
pub const XY_PAD_HEIGHT: f32 = 70.0;
/// Share of the mixer area given to the input row; outputs get the rest.
pub const INPUT_ROW_SHARE: f32 = 0.58;
/// Height of a `section` caption row, used when splitting the window between the two rows.
pub const SECTION_CAPTION_HEIGHT: f32 = 16.0;
/// Narrowest routing LED that still shows "A1" legibly.
pub const MIN_LED_ROUTE_WIDTH: f32 = 20.0;
/// Narrowest column, derived so five routing LEDs at their minimum fit beside the fader and meter.
pub const MIN_COLUMN_WIDTH: f32 =
    5.0 * MIN_LED_ROUTE_WIDTH + 4.0 * ITEM_SPACING + ITEM_SPACING + FADER_WIDTH + METER_WIDTH + 2.0 * PANEL_PADDING;
/// Fixed content of an input column besides its fader: header, device, knobs, pad, pan row.
pub const INPUT_FIXED_HEIGHT: f32 = 230.0;
/// Fixed content of an output column besides its fader: header, device, tone knobs, LEDs.
pub const OUTPUT_FIXED_HEIGHT: f32 = 110.0;
/// Top bar plus the first-run banner.
pub const CHROME_HEIGHT: f32 = 140.0;
/// Smallest window: seven columns at their minimum width, and both rows at their minimum height.
pub const MIN_WINDOW: Vec2 = Vec2::new(
    crate::NUM_STRIPS as f32 * (MIN_COLUMN_WIDTH + ITEM_SPACING) + 2.0 * SECTION_GAP + ITEM_SPACING,
    CHROME_HEIGHT + 2.0 * (SECTION_GAP + SECTION_CAPTION_HEIGHT) + INPUT_FIXED_HEIGHT + OUTPUT_FIXED_HEIGHT + 2.0 * FADER_MIN_HEIGHT,
);

pub const FADER_MIN_DB: f32 = -60.0;
pub const FADER_MAX_DB: f32 = 12.0;
pub const METER_FLOOR_DB: f32 = -60.0;
pub const METER_GREEN_TOP_DB: f32 = -18.0;
pub const METER_AMBER_TOP_DB: f32 = -6.0;

/// Sizes of one column's controls, derived from the width the window gives it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    /// Width inside the panel padding.
    pub inner: f32,
    /// Width left of the fader and meter.
    pub left: f32,
    pub led_route: Vec2,
    pub led_triple: Vec2,
    pub led_pair: Vec2,
    /// The tone / echo pad beside the two knobs.
    pub xy_pad: Vec2,
    /// One soundboard pad in a three-column grid that spans `left`.
    pub pad: Vec2,
}

impl Geometry {
    pub fn for_column(width: f32) -> Self {
        // Clamped on purpose: if a window manager ignores the minimum size, controls keep their
        // legible size and the panel clips them at its edge instead of shrinking them to nothing.
        let inner = width.max(MIN_COLUMN_WIDTH) - 2.0 * PANEL_PADDING;
        let left = inner - FADER_WIDTH - METER_WIDTH - ITEM_SPACING;
        Self {
            inner,
            left,
            led_route: Vec2::new((left - 4.0 * ITEM_SPACING) / 5.0, LED_HEIGHT),
            led_triple: Vec2::new((left - 2.0 * ITEM_SPACING) / 3.0, LED_HEIGHT),
            led_pair: Vec2::new((left - ITEM_SPACING) / 2.0, LED_HEIGHT),
            xy_pad: Vec2::new(inner - 2.0 * KNOB_WIDTH - 2.0 * ITEM_SPACING, XY_PAD_HEIGHT),
            pad: Vec2::new((left - 2.0 * PAD_SPACING) / 3.0, PAD_HEIGHT),
        }
    }
}

/// Width of each of `count` equal columns across `available`, leaving one spacing spare so the
/// last column never touches the window edge.
pub fn column_width(available: f32, count: usize) -> f32 {
    (available - count as f32 * ITEM_SPACING) / count as f32
}

/// What one column hands its panel each frame.
#[derive(Clone, Copy, Debug)]
pub struct ColumnFrame {
    pub geo: Geometry,
    pub fader_height: f32,
    pub any_solo: bool,
}

/// Nudges a stretchable height towards filling `target`, given what the panel measured last frame.
pub fn stretch_towards(current: f32, measured: f32, target: f32) -> f32 {
    let diff = target - measured;
    if diff.abs() < 1.0 {
        current
    } else {
        (current + diff).clamp(FADER_MIN_HEIGHT, FADER_MAX_HEIGHT)
    }
}

// ---- Colour tokens ---------------------------------------------------------------------------

pub const COLOR_BG: Color32 = Color32::from_rgb(0x1a, 0x1c, 0x20);
pub const COLOR_PANEL: Color32 = Color32::from_rgb(0x22, 0x24, 0x29);
/// One strip or bus.
pub const COLOR_STRIP: Color32 = Color32::from_rgb(0x2a, 0x2d, 0x33);
pub const COLOR_STRIP_STROKE: Color32 = Color32::from_rgb(0x3d, 0x41, 0x4a);
/// Recessed surfaces: LED buttons when off, fader tracks, meter wells, knob faces.
pub const COLOR_INSET: Color32 = Color32::from_rgb(0x14, 0x16, 0x19);
pub const COLOR_HANDLE: Color32 = Color32::from_rgb(0xc9, 0xce, 0xd6);
pub const COLOR_TEXT: Color32 = Color32::from_rgb(0xd8, 0xdc, 0xe3);
pub const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(0x9a, 0xa3, 0xb0);
/// Panel headings, the console's signature cyan.
pub const COLOR_HEADING: Color32 = Color32::from_rgb(0x8f, 0xd3, 0xea);
/// Hardware routes and "on" states.
pub const COLOR_ACTIVE: Color32 = Color32::from_rgb(0x5d, 0xf2, 0x7a);
/// Virtual (stream / call) routes and loaded pads.
pub const COLOR_VIRTUAL: Color32 = Color32::from_rgb(0x5a, 0xb8, 0xff);
pub const COLOR_SOLO: Color32 = Color32::from_rgb(0xff, 0xd8, 0x4a);
pub const COLOR_MUTE: Color32 = Color32::from_rgb(0xff, 0x9b, 0x3d);
pub const COLOR_CLIP: Color32 = Color32::from_rgb(0xff, 0x4f, 0x4f);
pub const COLOR_ASSIGNED: Color32 = COLOR_VIRTUAL;
pub const COLOR_ERROR: Color32 = COLOR_CLIP;

/// Installs the console theme: palette, radii, spacing and type scale.
pub fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = COLOR_PANEL;
    visuals.window_fill = COLOR_STRIP;
    visuals.extreme_bg_color = COLOR_INSET;
    visuals.faint_bg_color = COLOR_PANEL;
    visuals.override_text_color = Some(COLOR_TEXT);
    visuals.hyperlink_color = COLOR_HEADING;
    visuals.slider_trailing_fill = true;
    visuals.selection.bg_fill = COLOR_HEADING.gamma_multiply(0.45);
    visuals.selection.stroke = Stroke::new(1.0_f32, COLOR_HEADING);

    let w = &mut visuals.widgets;
    w.noninteractive.bg_fill = COLOR_STRIP;
    w.noninteractive.weak_bg_fill = COLOR_STRIP;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, COLOR_STRIP_STROKE);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT_MUTED);
    w.inactive.bg_fill = COLOR_INSET;
    w.inactive.weak_bg_fill = COLOR_INSET;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, COLOR_STRIP_STROKE);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT);
    w.hovered.bg_fill = COLOR_STRIP_STROKE;
    w.hovered.weak_bg_fill = COLOR_STRIP_STROKE;
    w.hovered.bg_stroke = Stroke::new(1.0_f32, COLOR_TEXT_MUTED);
    w.hovered.fg_stroke = Stroke::new(1.5_f32, COLOR_TEXT);
    w.active.bg_fill = COLOR_HEADING.gamma_multiply(0.35);
    w.active.weak_bg_fill = COLOR_HEADING.gamma_multiply(0.35);
    w.active.bg_stroke = Stroke::new(1.0_f32, COLOR_HEADING);
    w.active.fg_stroke = Stroke::new(2.0_f32, COLOR_TEXT);
    w.open.bg_fill = COLOR_STRIP_STROKE;
    w.open.weak_bg_fill = COLOR_STRIP_STROKE;
    for v in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        v.rounding = Rounding::same(CONTROL_RADIUS);
    }
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(ITEM_SPACING, ITEM_SPACING);
    style.spacing.button_padding = Vec2::new(8.0, 3.0);
    style.spacing.interact_size = Vec2::new(36.0, 22.0);
    use egui::{FontFamily::Proportional, TextStyle};
    style.text_styles = [
        (TextStyle::Small, FontId::new(10.5, Proportional)),
        (TextStyle::Body, FontId::new(12.5, Proportional)),
        (TextStyle::Button, FontId::new(12.0, Proportional)),
        (TextStyle::Heading, FontId::new(16.0, Proportional)),
        (TextStyle::Monospace, FontId::monospace(11.5)),
    ]
    .into();
    ctx.set_style(style);
}

// ---- Containers and text ---------------------------------------------------------------------

fn framed(fill: Color32, stroke: Color32) -> egui::Frame {
    egui::Frame::none()
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, stroke))
        .rounding(Rounding::same(PANEL_RADIUS))
        .inner_margin(Margin::same(PANEL_PADDING))
}

/// One strip or bus: a console panel whose outer width is exactly `width`. Returns its rect so the
/// caller can measure how tall the content came out.
pub fn panel(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui)) -> Rect {
    framed(COLOR_STRIP, COLOR_STRIP_STROKE)
        .show(ui, |ui| {
            let inner = width - 2.0 * PANEL_PADDING;
            ui.set_width(inner);
            ui.set_max_width(inner);
            ui.vertical(add);
        })
        .response
        .rect
}

/// Full-width notice, used for first-run guidance.
pub fn banner(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    framed(COLOR_PANEL, COLOR_VIRTUAL.gamma_multiply(0.6)).show(ui, |ui| ui.vertical(add));
}

/// Console heading in cyan capitals, with an optional status word on the right.
pub fn panel_header(ui: &mut Ui, title: &str, status: Option<(&str, Color32)>) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title.to_uppercase()).size(12.0).strong().color(COLOR_HEADING));
        if let Some((text, color)) = status {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(text).size(10.0).strong().color(color));
            });
        }
    });
}

/// Small uppercase caption naming the group of controls beneath it.
pub fn section(ui: &mut Ui, label: &str) {
    ui.add_space(SECTION_GAP);
    ui.label(egui::RichText::new(label.to_uppercase()).size(9.5).strong().color(COLOR_TEXT_MUTED));
}

/// Secondary text: hints, device status, captions.
pub fn hint(ui: &mut Ui, text: impl Into<String>) {
    ui.label(egui::RichText::new(text.into()).small().color(COLOR_TEXT_MUTED));
}

/// Error text, same style everywhere it appears.
pub fn error_label(ui: &mut Ui, text: &str) {
    ui.colored_label(COLOR_ERROR, text);
}

// ---- Controls --------------------------------------------------------------------------------

/// Text colour that stays readable on a filled control of `fill`.
pub fn on_color(fill: Color32) -> Color32 {
    let [r, g, b, _] = fill.to_array();
    let luma = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
    if luma > 150.0 {
        COLOR_BG
    } else {
        Color32::WHITE
    }
}

/// A recessed button whose label lights up in `color` when on, like a console LED.
pub fn led(ui: &mut Ui, on: &mut bool, label: &str, color: Color32, size: Vec2, tip: &str) -> bool {
    let (text, stroke) = if *on { (color, color.gamma_multiply(0.7)) } else { (COLOR_TEXT_MUTED, COLOR_STRIP_STROKE) };
    let button = egui::Button::new(egui::RichText::new(label).size(10.0).strong().color(text))
        .fill(COLOR_INSET)
        .stroke(Stroke::new(1.0_f32, stroke))
        .rounding(Rounding::same(CONTROL_RADIUS))
        .min_size(size);
    // Tight padding so the LED is exactly `size`, not text plus the theme's button padding.
    let mut response = ui.scope(|ui| {
        ui.spacing_mut().button_padding = Vec2::new(LED_PADDING, LED_PADDING);
        ui.add(button)
    }).inner;
    if *on {
        ui.painter().rect_stroke(response.rect.expand(1.5), Rounding::same(CONTROL_RADIUS + 1.0), Stroke::new(1.0_f32, color.gamma_multiply(0.3)));
    }
    if !tip.is_empty() {
        response = response.on_hover_text(tip);
    }
    if response.clicked() {
        *on = !*on;
        return true;
    }
    false
}

/// A filled rectangular button of a given size: soundboard pads and primary actions.
pub fn tile_button(ui: &mut Ui, text: egui::RichText, fill: Color32, size: Vec2) -> Response {
    ui.add(egui::Button::new(text).fill(fill).rounding(Rounding::same(CONTROL_RADIUS)).min_size(size))
}

/// A filled primary action, for the few commands that are not toggles.
pub fn action_button(ui: &mut Ui, label: &str, color: Color32) -> Response {
    tile_button(ui, egui::RichText::new(label).strong().color(on_color(color)), color, Vec2::ZERO)
}

/// The name of whatever is currently loaded or selected.
pub fn value_label(ui: &mut Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(12.0).strong());
}

/// Draws an arc as short segments; no allocation, since knobs redraw every frame.
fn arc(painter: &egui::Painter, center: Pos2, radius: f32, from: f32, to: f32, stroke: Stroke) {
    const STEPS: usize = 24;
    let point = |i: usize| center + Vec2::angled(from + (to - from) * i as f32 / STEPS as f32) * radius;
    for i in 0..STEPS {
        painter.line_segment([point(i), point(i + 1)], stroke);
    }
}

/// A rotary knob: drag up or down to change, double-click to reset to `reset`.
pub fn knob(ui: &mut Ui, value: &mut f32, range: RangeInclusive<f32>, reset: f32, label: &str, tip: &str) -> bool {
    let (min, max) = (*range.start(), *range.end());
    debug_assert!(max > min, "knob range must not be empty");
    let size = Vec2::new(KNOB_WIDTH, KNOB_SIZE + 14.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    if max <= min {
        return false;
    }
    let mut changed = false;
    if response.dragged() {
        *value = (*value - response.drag_delta().y / 110.0 * (max - min)).clamp(min, max);
        changed = true;
    }
    if response.double_clicked() {
        *value = reset;
        changed = true;
    }
    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    let center = Pos2::new(rect.center().x, rect.top() + KNOB_SIZE / 2.0 + 3.0);
    let radius = KNOB_SIZE / 2.0 - 3.0;
    let start = 135f32.to_radians();
    let sweep = 270f32.to_radians();
    let painter = ui.painter();
    painter.circle_filled(center, radius, COLOR_INSET);
    painter.circle_stroke(center, radius, Stroke::new(1.0_f32, COLOR_STRIP_STROKE));
    arc(painter, center, radius + 3.0, start, start + sweep, Stroke::new(2.0_f32, COLOR_STRIP_STROKE));
    if t > 0.0 {
        arc(painter, center, radius + 3.0, start, start + sweep * t, Stroke::new(2.0_f32, COLOR_ACTIVE));
    }
    let pointer = Vec2::angled(start + sweep * t);
    let pointer_color = if t > 0.0 { COLOR_TEXT } else { COLOR_TEXT_MUTED };
    painter.line_segment([center + pointer * (radius * 0.4), center + pointer * (radius * 0.9)], Stroke::new(2.0_f32, pointer_color));
    painter.text(Pos2::new(center.x, rect.bottom() - 5.0), Align2::CENTER_CENTER, label, FontId::proportional(10.0), COLOR_TEXT_MUTED);
    response.on_hover_text(format!("{tip}\n{label}: {value:.1}. Drag up or down; double-click resets."));
    changed
}

const FADER_TICKS: [(f32, &str); 8] = [
    (12.0, "12"),
    (6.0, ""),
    (0.0, "0"),
    (-6.0, ""),
    (-12.0, "-12"),
    (-24.0, "-24"),
    (-40.0, "-40"),
    (-60.0, "-60"),
];

/// Vertical fader with a dB ruler and a rectangular cap. Double-click resets to 0 dB.
pub fn fader(ui: &mut Ui, value: &mut f32, height: f32) -> bool {
    let size = Vec2::new(FADER_WIDTH, height + 16.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let track_top = rect.top() + 8.0;
    let track_bottom = rect.top() + height - 8.0;
    let span = FADER_MAX_DB - FADER_MIN_DB;
    let to_y = |db: f32| track_bottom - (db - FADER_MIN_DB) / span * (track_bottom - track_top);
    let mut changed = false;
    if response.dragged() {
        *value = (*value - response.drag_delta().y / (track_bottom - track_top) * span).clamp(FADER_MIN_DB, FADER_MAX_DB);
        changed = true;
    }
    if response.double_clicked() {
        *value = 0.0;
        changed = true;
    }

    let painter = ui.painter();
    let ruler_x = rect.left() + 20.0;
    for (db, label) in FADER_TICKS {
        let y = to_y(db);
        let len = if label.is_empty() { 3.0 } else { 5.0 };
        painter.line_segment([Pos2::new(ruler_x - len, y), Pos2::new(ruler_x, y)], Stroke::new(1.0_f32, COLOR_TEXT_MUTED));
        if !label.is_empty() {
            painter.text(Pos2::new(ruler_x - len - 2.0, y), Align2::RIGHT_CENTER, label, FontId::proportional(9.0), COLOR_TEXT_MUTED);
        }
    }
    let track_x = ruler_x + 12.0;
    let track = Rect::from_min_max(Pos2::new(track_x - 3.0, track_top), Pos2::new(track_x + 3.0, track_bottom));
    painter.rect_filled(track, Rounding::same(2.0), COLOR_INSET);
    painter.rect_stroke(track, Rounding::same(2.0), Stroke::new(1.0_f32, COLOR_STRIP_STROKE));
    let unity = to_y(0.0);
    painter.line_segment([Pos2::new(track_x - 8.0, unity), Pos2::new(track_x + 8.0, unity)], Stroke::new(1.0_f32, COLOR_TEXT_MUTED));
    let cap = Rect::from_center_size(Pos2::new(track_x, to_y(*value)), Vec2::new(22.0, 13.0));
    painter.rect_filled(cap, Rounding::same(2.0), COLOR_HANDLE);
    painter.line_segment([Pos2::new(cap.left() + 3.0, cap.center().y), Pos2::new(cap.right() - 3.0, cap.center().y)], Stroke::new(1.5_f32, COLOR_BG));
    let value_color = if *value > 0.0 { COLOR_SOLO } else { COLOR_TEXT };
    painter.text(Pos2::new(rect.center().x + 6.0, rect.bottom() - 6.0), Align2::CENTER_CENTER, format!("{value:+.1}"), FontId::monospace(11.0), value_color);
    response.on_hover_text("Drag to set level in dB. Double-click resets to 0 dB.");
    changed
}

fn db_fraction(db: f32) -> f32 {
    ((db - METER_FLOOR_DB) / -METER_FLOOR_DB).clamp(0.0, 1.0)
}

fn meter_fraction(peak: f32) -> f32 {
    db_fraction(crate::gain_to_db(peak))
}

/// Colour of the meter segment whose top sits at `fraction` of full scale.
fn segment_color(fraction: f32) -> Color32 {
    if fraction > db_fraction(METER_AMBER_TOP_DB) {
        COLOR_CLIP
    } else if fraction > db_fraction(METER_GREEN_TOP_DB) {
        COLOR_SOLO
    } else {
        COLOR_ACTIVE
    }
}

/// Two columns of LED segments, aligned with the fader track beside them.
pub fn meter(ui: &mut Ui, peaks: [f32; 2], height: f32) {
    let size = Vec2::new(METER_WIDTH, height + 16.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let top = rect.top() + 8.0;
    let bottom = rect.top() + height - 8.0;
    let well = Rect::from_min_max(Pos2::new(rect.left(), top - 2.0), Pos2::new(rect.right(), bottom + 2.0));
    let painter = ui.painter();
    painter.rect_filled(well, Rounding::same(2.0), COLOR_INSET);
    let seg_h = (bottom - top) / METER_SEGMENTS as f32;
    let col_w = (METER_WIDTH - 6.0) / 2.0;
    for (ch, peak) in peaks.iter().enumerate() {
        let level = meter_fraction(*peak);
        let x0 = rect.left() + 2.0 + ch as f32 * (col_w + 2.0);
        for i in 0..METER_SEGMENTS {
            let seg_top_frac = (i + 1) as f32 / METER_SEGMENTS as f32;
            let lit = level >= (i as f32 + 0.5) / METER_SEGMENTS as f32;
            let color = segment_color(seg_top_frac);
            let color = if lit { color } else { color.gamma_multiply(0.12) };
            let y1 = bottom - seg_h * i as f32;
            let seg = Rect::from_min_max(Pos2::new(x0, y1 - seg_h + 1.0), Pos2::new(x0 + col_w, y1));
            painter.rect_filled(seg, Rounding::ZERO, color);
        }
    }
}

/// Two-axis pad: `x` in -1..1 (left label .. right label), `y` in 0..1 (bottom .. top label).
/// Drag anywhere; double-click resets both to zero.
pub fn xy_pad(ui: &mut Ui, x: &mut f32, y: &mut f32, size: Vec2, labels: [&str; 3], tip: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let mut changed = false;
    if response.dragged() || response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            *x = ((pos.x - rect.center().x) / (rect.width() / 2.0 - 6.0)).clamp(-1.0, 1.0);
            *y = ((rect.bottom() - pos.y - 6.0) / (rect.height() - 12.0)).clamp(0.0, 1.0);
            changed = true;
        }
    }
    if response.double_clicked() {
        *x = 0.0;
        *y = 0.0;
        changed = true;
    }
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(CONTROL_RADIUS), COLOR_INSET);
    painter.rect_stroke(rect, Rounding::same(CONTROL_RADIUS), Stroke::new(1.0_f32, COLOR_STRIP_STROKE));
    let grid = Stroke::new(1.0_f32, COLOR_STRIP_STROKE);
    painter.line_segment([Pos2::new(rect.center().x, rect.top() + 4.0), Pos2::new(rect.center().x, rect.bottom() - 4.0)], grid);
    painter.line_segment([Pos2::new(rect.left() + 4.0, rect.bottom() - 6.0), Pos2::new(rect.right() - 4.0, rect.bottom() - 6.0)], grid);
    let font = FontId::proportional(9.0);
    painter.text(Pos2::new(rect.left() + 4.0, rect.bottom() - 3.0), Align2::LEFT_BOTTOM, labels[0], font.clone(), COLOR_TEXT_MUTED);
    painter.text(Pos2::new(rect.right() - 4.0, rect.bottom() - 3.0), Align2::RIGHT_BOTTOM, labels[1], font.clone(), COLOR_TEXT_MUTED);
    painter.text(Pos2::new(rect.center().x, rect.top() + 3.0), Align2::CENTER_TOP, labels[2], font, COLOR_TEXT_MUTED);
    let dot = Pos2::new(
        rect.center().x + *x * (rect.width() / 2.0 - 6.0),
        rect.bottom() - 6.0 - *y * (rect.height() - 12.0),
    );
    painter.circle_filled(dot, 6.0, COLOR_MUTE);
    painter.circle_stroke(dot, 6.0, Stroke::new(1.0_f32, COLOR_BG));
    response.on_hover_text(tip);
    changed
}

/// One combo box for every picker: shows `selected_text`, lists `choices` as (label, value,
/// is_current) and returns the value the user clicked, if any. The list is only walked while the
/// popup is open, so callers can hand it borrowed labels without per-frame allocation.
pub fn choice_combo<'a, T>(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    width: f32,
    selected_text: &str,
    choices: impl IntoIterator<Item = (std::borrow::Cow<'a, str>, T, bool)>,
) -> Option<T> {
    let mut picked = None;
    egui::ComboBox::from_id_salt(id).width(width).selected_text(shorten(selected_text, 24)).show_ui(ui, |ui| {
        for (label, value, current) in choices {
            if ui.selectable_label(current, label.as_ref()).clicked() {
                picked = Some(value);
            }
        }
    });
    picked
}

/// Device picker with a "None" entry. Returns true when the choice changed.
pub fn device_combo(ui: &mut Ui, id: impl std::hash::Hash, selected: &mut Option<String>, names: &[String], empty_label: &str, width: f32) -> bool {
    use std::borrow::Cow;
    let label = selected.as_deref().unwrap_or(empty_label);
    let none = std::iter::once((Cow::Borrowed("— none —"), None, selected.is_none()));
    let devices = names.iter().map(|n| (Cow::Borrowed(n.as_str()), Some(n.as_str()), selected.as_deref() == Some(n.as_str())));
    match choice_combo(ui, id, width, label, none.chain(devices)) {
        Some(value) => {
            *selected = value.map(str::to_string);
            true
        }
        None => false,
    }
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
    fn db_fraction_clamps_at_the_floor_and_at_zero_db() {
        assert_eq!(db_fraction(METER_FLOOR_DB), 0.0);
        assert_eq!(db_fraction(METER_FLOOR_DB - 20.0), 0.0);
        assert_eq!(db_fraction(0.0), 1.0);
        assert_eq!(db_fraction(20.0), 1.0);
    }

    #[test]
    fn meter_segments_go_green_amber_red_from_bottom_to_top() {
        assert_eq!(segment_color(0.1), COLOR_ACTIVE);
        assert_eq!(segment_color(db_fraction(-10.0)), COLOR_SOLO);
        assert_eq!(segment_color(1.0), COLOR_CLIP);
    }

    #[test]
    fn geometry_fills_its_column_exactly() {
        let g = Geometry::for_column(230.0);
        assert_eq!(g.inner, 214.0);
        assert!((5.0 * g.led_route.x + 4.0 * ITEM_SPACING - g.left).abs() < 1e-4);
        assert!((2.0 * KNOB_WIDTH + g.xy_pad.x + 2.0 * ITEM_SPACING - g.inner).abs() < 1e-4);
        assert!((3.0 * g.pad.x + 2.0 * PAD_SPACING - g.left).abs() < 1e-4);
    }

    #[test]
    fn geometry_never_shrinks_below_the_minimum_column() {
        let g = Geometry::for_column(120.0);
        assert_eq!(g, Geometry::for_column(MIN_COLUMN_WIDTH));
        assert!(g.led_route.x >= MIN_LED_ROUTE_WIDTH - 1e-4, "routing LEDs stay readable: {}", g.led_route.x);
        assert!(g.pad.x >= 30.0);
    }

    #[test]
    fn columns_share_the_width_with_gaps_between() {
        let w = column_width(1000.0, 7);
        assert!((7.0 * w + 7.0 * ITEM_SPACING - 1000.0).abs() < 1e-4);
    }

    #[test]
    fn column_width_for_a_single_column_leaves_one_spacing_spare() {
        let w = column_width(500.0, 1);
        assert_eq!(w, 500.0 - ITEM_SPACING);
    }

    #[test]
    fn stretch_moves_towards_the_target_and_stays_in_range() {
        assert_eq!(stretch_towards(150.0, 300.0, 340.0), 190.0);
        assert_eq!(stretch_towards(150.0, 300.0, 300.5), 150.0);
        assert_eq!(stretch_towards(150.0, 300.0, 5000.0), FADER_MAX_HEIGHT);
        assert_eq!(stretch_towards(150.0, 300.0, 10.0), FADER_MIN_HEIGHT);
    }

    #[test]
    fn stretch_towards_is_idempotent_once_measured_equals_target() {
        // Once the panel has actually measured up to the target height, repeatedly asking to
        // stretch towards that same target must not keep nudging the stored height.
        for current in [FADER_MIN_HEIGHT, 150.0, 300.0, FADER_MAX_HEIGHT] {
            let once = stretch_towards(current, 220.0, 220.0);
            assert_eq!(once, current, "no movement when measured already equals target");
            let twice = stretch_towards(once, 220.0, 220.0);
            assert_eq!(twice, once, "applying again from a settled state must be a no-op");
        }
    }

    /// The narrowest window the app claims to support: MIN_WINDOW.x px split across every strip and bus
    /// column, minus the layout slack the app reserves. Every size the column derives from that
    /// width must still be usable, and a routing LED must be wide enough to read its label.
    #[test]
    fn geometry_at_the_minimum_window_width_keeps_every_derived_size_usable() {
        let column = column_width(MIN_WINDOW.x - 2.0 * SECTION_GAP, 7);
        let g = Geometry::for_column(column);
        assert!(g.inner > 0.0, "inner width must be positive, got {}", g.inner);
        assert!(g.left > 0.0, "left width must be positive, got {}", g.left);
        assert!(g.led_triple.x > 0.0);
        assert!(g.led_pair.x > 0.0);
        assert!(g.xy_pad.x > 0.0);
        assert!(g.pad.x > 0.0);
        assert!(
            g.led_route.x >= 14.0,
            "routing LED must stay at least 14 px wide to hold its label, got {} at column width {}",
            g.led_route.x,
            column,
        );
    }

    #[test]
    fn shorten_adds_ellipsis_only_when_needed() {
        assert_eq!(shorten("short", 10), "short");
        assert_eq!(shorten("a very long device name", 8), "a very …");
    }

    #[test]
    fn chip_text_is_dark_on_bright_leds_and_light_on_dark_fills() {
        assert_eq!(on_color(COLOR_SOLO), COLOR_BG);
        assert_eq!(on_color(COLOR_VIRTUAL), COLOR_BG);
        assert_eq!(on_color(COLOR_STRIP), Color32::WHITE);
    }
}
