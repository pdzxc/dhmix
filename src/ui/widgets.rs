//! Design tokens and shared controls, styled after a hardware console (Voicemeeter Potato):
//! charcoal panels, orange uppercase titles (cyan for focus and the fine-tune opener), LED
//! buttons, rotary knobs, ruler faders and segmented meters.

use egui::{Align, Align2, Color32, FontId, Margin, Pos2, Rect, Response, Rounding, Sense, Shape, Stroke, Ui, UiBuilder, Vec2};
use std::ops::RangeInclusive;

// ---- Layout tokens ---------------------------------------------------------------------------

/// Gap between neighbouring controls; the theme's item spacing.
pub const ITEM_SPACING: f32 = 4.0;
/// Air between a card's edge and its controls.
pub const PANEL_PADDING: f32 = 10.0;
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
/// Outer height of a knob widget: the dial plus its caption.
pub const KNOB_HEIGHT: f32 = KNOB_SIZE + 14.0;
/// The horizontal pan control beneath a strip's knobs.
pub const PAN_HEIGHT: f32 = 18.0;
/// One A / B routing LED; short labels, so they stay small like the console's.
pub const ROUTE_LED_WIDTH: f32 = 44.0;
/// Room a stock slider's trailing label ("seek", "music volume", "pads volume") takes beside it.
pub const SLIDER_LABEL_WIDTH: f32 = 88.0;
/// The "Stop all" button beside the pads volume slider; always present so the row never jumps.
pub const STOP_ALL_SIZE: Vec2 = Vec2::new(64.0, LED_HEIGHT);
/// The Settings window and its toggles, wide enough for "RUN ON STARTUP".
pub const SETTINGS_WINDOW_WIDTH: f32 = 420.0;
pub const SETTINGS_TOGGLE_SIZE: Vec2 = Vec2::new(124.0, LED_HEIGHT);
/// The mixer window is one fixed size, like a hardware console; nothing reflows.
pub const WINDOW_SIZE: Vec2 = Vec2::new(1400.0, 852.0);
/// Buttons stacked beside a fader never grow past this; the app list gets the spare width.
pub const SIDE_BUTTON_MAX_WIDTH: f32 = 64.0;
/// Gap between neighbouring columns in a row.
pub const COLUMN_GAP: f32 = 12.0;
/// Gap between a fader-and-meter pair and the button column or app list beside it, so the
/// meter's segments never read as part of the buttons.
pub const LEVEL_BLOCK_GAP: f32 = COLUMN_GAP;
/// Room above and below a fader's track (and a meter's segments) for the end ticks and cap.
pub const FADER_MARGIN: f32 = 8.0;
/// A meter's well reaches this far past its top and bottom segments.
pub const METER_WELL_OVERHANG: f32 = 2.0;
/// Columns beside a fader start level with the meter well, not with the fader's hidden margin.
pub const LEVEL_BLOCK_TOP: f32 = FADER_MARGIN - METER_WELL_OVERHANG;
/// The vertical rule between the hardware and virtual groups; a column gap sits on each side.
pub const GROUP_DIVIDER_WIDTH: f32 = 1.0;
/// Most app names shown under a virtual input or output before "+n more".
pub const APP_LIST_ROWS: usize = 5;
/// Below this the app list is not drawn at all and the buttons take the row.
pub const MIN_APP_LIST_WIDTH: f32 = 48.0;
/// Subtracted from rows that would otherwise end exactly on the panel edge, so pixel rounding of
/// fractional column widths cannot clip the last control.
pub const ROUNDING_SLACK: f32 = 1.0;
/// The Record / Stop button in the top bar: a square holding only the glyph.
pub const RECORD_BUTTON_SIZE: Vec2 = Vec2::new(34.0, LED_HEIGHT);
/// The record dot (and stop square) in the Record button.
pub const RECORD_GLYPH_RADIUS: f32 = 4.0;
/// Top-bar window toggles (Player, Applications, Help, Settings).
pub const TOP_TOGGLE_SIZE: Vec2 = Vec2::new(92.0, 22.0);
/// Top-bar actions: Refresh, Load…, Save….
pub const TOP_BUTTON_SIZE: Vec2 = Vec2::new(72.0, LED_HEIGHT);
/// The Player's transport buttons: Load…, Play / Pause, Stop, Loop.
pub const TRANSPORT_BUTTON_SIZE: Vec2 = Vec2::new(56.0, LED_HEIGHT);
/// Outer width of a knob widget.
pub const KNOB_WIDTH: f32 = KNOB_SIZE + 12.0;
/// The value printed in the middle of a knob's face.
pub const KNOB_VALUE_FONT_SIZE: f32 = 9.0;
/// The position dot on a knob's rim: where it sits (as a share of the face radius) and how big.
pub const KNOB_DOT_RADIUS_FRACTION: f32 = 0.8;
pub const KNOB_DOT_RADIUS: f32 = 2.0;
/// The field that opens under a knob or fader to type its value: the number box is this wide
/// whether it is showing or editing, so it never jumps.
pub const VALUE_FIELD_WIDTH: f32 = 56.0;
/// The COMP and GATE knobs side by side; the pan control sits beneath them at this width.
pub const KNOB_BLOCK_WIDTH: f32 = 2.0 * KNOB_WIDTH + ITEM_SPACING;
/// Narrowest colour pad that still has room to drag on.
pub const XY_PAD_MIN_WIDTH: f32 = 60.0;
pub const LED_HEIGHT: f32 = 22.0;
/// The bus-name chip in an output's header.
pub const HEADER_TAG_SIZE: Vec2 = Vec2::new(28.0, 16.0);
pub const SMALL_COMBO_WIDTH: f32 = 96.0;
/// The "Send to" picker in the Applications window; wide enough for a device plus its meaning.
pub const ROUTE_COMBO_WIDTH: f32 = 230.0;
/// Sliders in the floating fine-tune window, which has room to spare.
pub const FINE_TUNE_SLIDER_WIDTH: f32 = 144.0;
/// Column and row gaps of the Applications table.
pub const TABLE_SPACING: Vec2 = Vec2::new(12.0, 6.0);
/// The floating Player window: wide enough for the pad grid beside the fader.
pub const PLAYER_WINDOW_WIDTH: f32 = 360.0;
pub const PLAYER_WINDOW_FADER_HEIGHT: f32 = 150.0;
pub const PAD_HEIGHT: f32 = 26.0;
pub const PAD_SPACING: f32 = 4.0;
/// The colour pad is exactly as tall as the knobs and the pan control beside it.
pub const XY_PAD_HEIGHT: f32 = KNOB_HEIGHT + ITEM_SPACING + PAN_HEIGHT;
/// Share of the mixer area given to the input row; outputs get the rest.
pub const INPUT_ROW_SHARE: f32 = 0.58;
/// Height of a `section` caption row, used when splitting the window between the two rows.
pub const SECTION_CAPTION_HEIGHT: f32 = 16.0;
/// Narrowest button in the columns beside a fader that still shows "MUTE" legibly.
pub const MIN_SIDE_BUTTON_WIDTH: f32 = 40.0;
/// Narrowest column that still fits a strip's top row: the colour pad beside the knob block.
const MIN_COLUMN_FOR_STRIP: f32 = XY_PAD_MIN_WIDTH + ITEM_SPACING + KNOB_BLOCK_WIDTH + 2.0 * PANEL_PADDING;
/// Narrowest column that still fits a bus row: the two-knob column, fader and meter.
const MIN_COLUMN_FOR_BUS: f32 = KNOB_BLOCK_WIDTH + FADER_WIDTH + METER_WIDTH + 2.0 * ITEM_SPACING + LEVEL_BLOCK_GAP + 2.0 * PANEL_PADDING;
/// Narrowest column: whichever row needs more.
pub const MIN_COLUMN_WIDTH: f32 = if MIN_COLUMN_FOR_STRIP > MIN_COLUMN_FOR_BUS { MIN_COLUMN_FOR_STRIP } else { MIN_COLUMN_FOR_BUS };
/// Height of an input column beyond its fader: header, device combo, the colour-and-effects
/// block, the send-to rows, captions and padding. `strip_panel`'s layout test measures the
/// rendered strip against it, so a layout change that forgets to update it fails the build.
pub const INPUT_FIXED_HEIGHT: f32 = 305.0;
/// Same for an output column: header, device combo, caption and padding beside the fader.
pub const OUTPUT_FIXED_HEIGHT: f32 = 109.0;
/// The top bar: controls row and status line.
pub const CHROME_HEIGHT: f32 = 80.0;
/// Smallest window: the larger mixer row at its minimum width, and both rows at minimum height.
pub const MIN_WINDOW: Vec2 = Vec2::new(
    mixer_column_count() as f32 * (MIN_COLUMN_WIDTH + ITEM_SPACING) + 2.0 * SECTION_GAP + ITEM_SPACING,
    CHROME_HEIGHT + 2.0 * (SECTION_GAP + SECTION_CAPTION_HEIGHT) + INPUT_FIXED_HEIGHT + OUTPUT_FIXED_HEIGHT + 2.0 * FADER_MIN_HEIGHT,
);
const _: () = assert!(WINDOW_SIZE.x >= MIN_WINDOW.x && WINDOW_SIZE.y >= MIN_WINDOW.y, "the fixed window must fit every column at its minimum");

const fn mixer_column_count() -> usize {
    if crate::PLAYER_STRIP > crate::NUM_BUSES {
        crate::PLAYER_STRIP
    } else {
        crate::NUM_BUSES
    }
}

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
    /// Width left of the fader and meter (the Player window's pad grid and routing rows).
    pub left: f32,
    /// One A / B routing LED in the send-to rows.
    pub route_led: Vec2,
    pub led_triple: Vec2,
    /// The tone / echo pad, left of the knob block at the top of a strip.
    pub xy_pad: Vec2,
    /// One effect LED in the row of four beneath the colour pad and knobs.
    pub fx_led: Vec2,
    /// One button in the column beside a strip's fader (mono / solo / mute / fine-tune).
    pub side_button: Vec2,
    /// Width of the app list beside a strip's fader, whatever the button column leaves.
    pub strip_apps: f32,
    /// Width of a bus's left column: BASS and TREBLE over LIMIT and MUTE.
    pub bus_left: f32,
    /// The FINE-TUNE opener, the same size wherever a strip is drawn.
    pub fine_tune_button: Vec2,
    /// The MUTE button beside a bus's LIMIT knob.
    pub bus_button: Vec2,
    /// Width of the app list beside a bus's fader, whatever the knobs and fader leave.
    pub bus_apps: f32,
    /// One soundboard pad in a three-column grid that spans `left`.
    pub pad: Vec2,
}

impl Geometry {
    pub fn for_column(width: f32) -> Self {
        // Clamped on purpose: if a window manager ignores the minimum size, controls keep their
        // legible size and the panel clips them at its edge instead of shrinking them to nothing.
        let inner = width.max(MIN_COLUMN_WIDTH) - 2.0 * PANEL_PADDING;
        let left = inner - FADER_WIDTH - METER_WIDTH - ITEM_SPACING;
        // Strip bottom block: fader, meter, a LEVEL_BLOCK_GAP, one button column, another
        // LEVEL_BLOCK_GAP, then the app list takes the rest. A list narrower than
        // MIN_APP_LIST_WIDTH is dropped and the buttons widen instead.
        let after_meter = inner - FADER_WIDTH - ITEM_SPACING - METER_WIDTH - LEVEL_BLOCK_GAP;
        let capped = after_meter.min(SIDE_BUTTON_MAX_WIDTH);
        let apps_if_capped = after_meter - capped - LEVEL_BLOCK_GAP;
        let (side_w, strip_apps) = if apps_if_capped >= MIN_APP_LIST_WIDTH {
            (capped, apps_if_capped)
        } else {
            (after_meter.clamp(MIN_SIDE_BUTTON_WIDTH, SIDE_BUTTON_MAX_WIDTH), 0.0)
        };
        // Bus: two knob rows on the left, fader and meter, a LEVEL_BLOCK_GAP, then the app list.
        let bus_left = KNOB_BLOCK_WIDTH;
        let bus_apps = inner - bus_left - FADER_WIDTH - METER_WIDTH - 2.0 * ITEM_SPACING - LEVEL_BLOCK_GAP;
        let bus_apps = if bus_apps >= MIN_APP_LIST_WIDTH { bus_apps } else { 0.0 };
        Self {
            inner,
            left,
            route_led: Vec2::new(ROUTE_LED_WIDTH, LED_HEIGHT),
            led_triple: Vec2::new((left - 2.0 * ITEM_SPACING) / 3.0, LED_HEIGHT),
            xy_pad: Vec2::new(inner - KNOB_BLOCK_WIDTH - ITEM_SPACING, XY_PAD_HEIGHT),
            fx_led: fx_led_size(inner),
            side_button: Vec2::new(side_w, LED_HEIGHT),
            fine_tune_button: Vec2::new(side_w, LED_HEIGHT),
            strip_apps,
            bus_left,
            bus_button: Vec2::new(KNOB_WIDTH, LED_HEIGHT),
            bus_apps,
            pad: Vec2::new((left - 2.0 * PAD_SPACING) / 3.0, PAD_HEIGHT),
        }
    }
}

/// Four effect LEDs across `width`, one pixel short so rounding never pushes the last one past
/// the panel edge.
pub fn fx_led_size(width: f32) -> Vec2 {
    Vec2::new((width - 3.0 * ITEM_SPACING - ROUNDING_SLACK) / 4.0, LED_HEIGHT)
}

/// Width for a stock slider that shares its row with a trailing label.
pub fn labelled_slider_width(available: f32) -> f32 {
    (available - SLIDER_LABEL_WIDTH).max(40.0)
}

/// Width of each of `count` equal columns across `available`: COLUMN_GAP between columns, room
/// for the group divider, and one spacing spare so the last column never touches the window edge.
pub fn column_width(available: f32, count: usize) -> f32 {
    let gaps = (count as f32 - 1.0) * COLUMN_GAP + COLUMN_GAP + GROUP_DIVIDER_WIDTH + ITEM_SPACING;
    (available - gaps) / count as f32
}

/// A row of columns with COLUMN_GAP between them.
pub fn column_row(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = COLUMN_GAP;
        add(ui);
    });
}

/// The vertical rule between the hardware and virtual groups of a row.
pub fn group_divider(ui: &mut Ui, height: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(GROUP_DIVIDER_WIDTH, height), Sense::hover());
    ui.painter().line_segment([rect.center_top(), rect.center_bottom()], Stroke::new(GROUP_DIVIDER_WIDTH, COLOR_STRIP_STROKE));
}

/// The apps currently on a virtual input or output, listed by name beside its fader. Draws
/// nothing (and takes no gap) when the column has no room for it.
pub fn app_list(ui: &mut Ui, width: f32, names: &[String], empty: &str) {
    if width < MIN_APP_LIST_WIDTH {
        return;
    }
    level_column(ui, width, |ui| {
        caption(ui, "Apps");
        if names.is_empty() {
            hint(ui, empty);
        }
        for name in names.iter().take(APP_LIST_ROWS) {
            ui.label(egui::RichText::new(shorten(name, 14)).small());
        }
        if names.len() > APP_LIST_ROWS {
            hint(ui, format!("+{} more", names.len() - APP_LIST_ROWS));
        }
    });
}

/// A column beside a fader and meter: a LEVEL_BLOCK_GAP before it, and its content starting
/// level with the meter well.
pub fn level_column(ui: &mut Ui, width: f32, add: impl FnOnce(&mut Ui)) {
    ui.add_space(LEVEL_BLOCK_GAP - ITEM_SPACING);
    ui.vertical(|ui| {
        ui.set_width(width);
        ui.add_space(LEVEL_BLOCK_TOP);
        add(ui);
    });
}

/// What one column hands its panel each frame.
#[derive(Clone, Copy, Debug)]
pub struct ColumnFrame {
    pub geo: Geometry,
    pub fader_height: f32,
    pub any_solo: bool,
}

/// Fader height that makes a panel with `fixed` px of other content exactly `row_height` tall.
/// Heights come only from the window size, so nothing the user clicks can change a panel's size.
pub fn fader_height_in(row_height: f32, fixed: f32) -> f32 {
    (row_height - fixed).clamp(FADER_MIN_HEIGHT, FADER_MAX_HEIGHT)
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
/// The console's accent, orange: the wordmark, panel titles, and the header's buttons when lit.
pub const COLOR_ACCENT: Color32 = Color32::from_rgb(0xff, 0x9b, 0x3d);
/// The console's cyan: the FINE-TUNE opener, keyboard focus, selections and links.
pub const COLOR_HEADING: Color32 = Color32::from_rgb(0x8f, 0xd3, 0xea);
/// Hardware routes and "on" states.
pub const COLOR_ACTIVE: Color32 = Color32::from_rgb(0x5d, 0xf2, 0x7a);
/// Virtual (stream / call) routes and loaded pads.
pub const COLOR_VIRTUAL: Color32 = Color32::from_rgb(0x5a, 0xb8, 0xff);
pub const COLOR_SOLO: Color32 = Color32::from_rgb(0xff, 0xd8, 0x4a);
pub const COLOR_MUTE: Color32 = COLOR_ACCENT;
pub const COLOR_CLIP: Color32 = Color32::from_rgb(0xff, 0x4f, 0x4f);
pub const COLOR_ASSIGNED: Color32 = COLOR_VIRTUAL;
/// The PLAYER toggle in the top bar.
pub const COLOR_PLAYER: Color32 = COLOR_ACCENT;
/// "LIVE" in a panel header: red, like a tally light.
pub const COLOR_LIVE: Color32 = COLOR_CLIP;
/// Panel titles: the accent, so they stand apart from the green and blue bus chips beside them.
pub const COLOR_PANEL_TITLE: Color32 = COLOR_ACCENT;
/// The dim grey line on the colour pad above which echo starts.
pub const COLOR_THRESHOLD: Color32 = Color32::from_rgb(0x5c, 0x62, 0x6c);
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

/// One strip or bus: a console panel whose outer size is exactly `width` by `height`. Returns its
/// rect. Content that does not fit is clipped; the panel never grows or shrinks with it.
pub fn panel(ui: &mut Ui, width: f32, height: f32, add: impl FnOnce(&mut Ui)) -> Rect {
    let frame = framed(COLOR_STRIP, COLOR_STRIP_STROKE);
    let panel_min = ui.next_widget_position();
    let inner_width = (width - 2.0 * PANEL_PADDING).max(0.0);
    let inner_min = panel_min + Vec2::splat(PANEL_PADDING);
    let inner_max = egui::pos2(inner_min.x + inner_width, panel_min.y + height - PANEL_PADDING);
    let background = ui.painter().add(Shape::Noop);

    // A normal `Frame::show` grows its parent when a child asks for more width. Mixer controls
    // intentionally keep a readable minimum, so use an independently clipped child and allocate
    // only the declared outer rectangle in the row. Content may clip, but neighbouring columns
    // never drift out of alignment.
    let mut content_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(Rect::from_min_max(inner_min, inner_max))
            .layout(egui::Layout::top_down(Align::Min)),
    );
    content_ui.set_width(inner_width);
    // The row that holds the panels spaces them with COLUMN_GAP; controls inside use the theme's
    // gap, which every Geometry width is budgeted for.
    content_ui.spacing_mut().item_spacing = Vec2::splat(ITEM_SPACING);
    content_ui.set_clip_rect(content_ui.clip_rect().intersect(Rect::from_min_max(panel_min, egui::pos2(panel_min.x + width, panel_min.y + height))));
    add(&mut content_ui);

    let rect = Rect::from_min_size(panel_min, Vec2::new(width, height));
    ui.painter().set(background, frame.paint(rect));
    ui.allocate_rect(rect, Sense::hover());
    rect
}

/// The wordmark's lettering, and the room between it and the controls.
pub const LOGO_FONT_SIZE: f32 = 22.0;
pub const LOGO_GAP: f32 = COLUMN_GAP;

/// The brand: the DHMIX wordmark, orange like the panel titles and the icon.
pub fn logo(ui: &mut Ui) {
    ui.label(egui::RichText::new("DHMIX").size(LOGO_FONT_SIZE).strong().color(COLOR_PANEL_TITLE));
    ui.add_space(LOGO_GAP);
}

/// Record / stop as a glyph alone, painted rather than typed: the default font has no record or
/// stop symbol, and a missing glyph renders as a box. A red dot to start; while recording the
/// button fills with the accent and shows a stop square.
pub fn record_button(ui: &mut Ui, recording: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(RECORD_BUTTON_SIZE, Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let (rest_fill, rest_stroke) = if recording { (COLOR_ACCENT, COLOR_ACCENT.gamma_multiply(0.7)) } else { (COLOR_INSET, COLOR_STRIP_STROKE) };
    let (fill, stroke) = button_face(&response, rest_fill, rest_stroke);
    let painter = ui.painter();
    painter.rect(rect, Rounding::same(CONTROL_RADIUS), fill, Stroke::new(1.0_f32, stroke));
    if recording {
        painter.rect_filled(Rect::from_center_size(rect.center(), Vec2::splat(2.0 * RECORD_GLYPH_RADIUS)), Rounding::same(1.0), on_color(COLOR_ACCENT));
    } else {
        painter.circle_filled(rect.center(), RECORD_GLYPH_RADIUS, COLOR_CLIP);
    }
    response.on_hover_text(if recording { "Stop and save the WAV file" } else { "Record the chosen bus to a WAV file" })
}

/// Console heading in cyan capitals, an optional coloured tag after it (a bus's name in its
/// route colour), and an optional status word on the right.
pub fn panel_header(ui: &mut Ui, title: &str, tag: Option<(&str, Color32)>, status: Option<(&str, Color32)>) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title.to_uppercase()).size(12.0).strong().color(COLOR_PANEL_TITLE));
        if let Some((text, color)) = tag {
            header_tag(ui, text, color);
        }
        if let Some((text, color)) = status {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(text).size(10.0).strong().color(color));
            });
        }
    });
}

/// A small filled chip in the header: the same colour the strips' A / B buttons light up in.
fn header_tag(ui: &mut Ui, text: &str, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(HEADER_TAG_SIZE, Sense::hover());
    ui.painter().rect_filled(rect, Rounding::same(CONTROL_RADIUS), color);
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, text, FontId::proportional(10.0), on_color(color));
}

/// Hardware buses (A) light green, virtual buses (B) light blue, wherever a bus is named.
pub fn bus_color(bus: usize) -> Color32 {
    if bus < crate::NUM_HW_BUSES {
        COLOR_ACTIVE
    } else {
        COLOR_VIRTUAL
    }
}

/// Small uppercase caption naming the group of controls beneath it. Not for use inside an
/// `egui::Grid`: it adds vertical space first, which a grid forbids. Use `caption` there.
pub fn section(ui: &mut Ui, label: &str) {
    ui.add_space(SECTION_GAP);
    caption(ui, label);
}

/// The same caption inline, naming the group of controls beside it (top bar).
pub fn caption(ui: &mut Ui, label: &str) {
    ui.label(egui::RichText::new(label.to_uppercase()).size(9.5).strong().color(COLOR_TEXT_MUTED));
}

/// A hint that ends a row of buttons, set off by the same gap the app list keeps from the
/// level block, so every "buttons, then words" row on a strip reads alike.
pub fn row_hint(ui: &mut Ui, text: impl Into<String>) {
    ui.add_space(LEVEL_BLOCK_GAP - ITEM_SPACING);
    hint(ui, text);
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

/// Fill and border for a painted button at rest, hovered or pressed. Hovering brightens an unlit
/// border to the same grey the stock controls use; a lit border keeps its colour. Pressing fills
/// it like a stock button.
fn button_face(response: &Response, rest_fill: Color32, rest_stroke: Color32) -> (Color32, Color32) {
    // `led` passes the resting stroke for an off LED; anything else is a lit colour to keep.
    let unlit = rest_stroke == COLOR_STRIP_STROKE;
    if response.is_pointer_button_down_on() {
        (COLOR_STRIP_STROKE, if unlit { COLOR_TEXT_MUTED } else { rest_stroke })
    } else if response.hovered() && unlit {
        (rest_fill, COLOR_TEXT_MUTED)
    } else {
        (rest_fill, rest_stroke)
    }
}

/// The recessed console button every LED and action shares: exactly `size`, label centred,
/// painted by hand, with `button_face` for hover and press and a pointer on hover.
fn recessed_button(ui: &mut Ui, label: &str, text: Color32, stroke: Color32, size: Vec2, tip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let (fill, stroke) = button_face(&response, COLOR_INSET, stroke);
    let painter = ui.painter();
    painter.rect(rect, Rounding::same(CONTROL_RADIUS), fill, Stroke::new(1.0_f32, stroke));
    painter.text(rect.center(), Align2::CENTER_CENTER, label, FontId::proportional(10.0), text);
    if tip.is_empty() {
        response
    } else {
        response.on_hover_text(tip)
    }
}

/// The soft ring around a lit LED.
fn glow(ui: &Ui, rect: Rect, color: Color32) {
    ui.painter().rect_stroke(rect.expand(1.5), Rounding::same(CONTROL_RADIUS + 1.0), Stroke::new(1.0_f32, color.gamma_multiply(0.3)));
}

/// A recessed button whose label lights up in `color` when on, like a console LED.
pub fn led(ui: &mut Ui, on: &mut bool, label: &str, color: Color32, size: Vec2, tip: &str) -> bool {
    let (text, stroke) = if *on { (color, color.gamma_multiply(0.7)) } else { (COLOR_TEXT_MUTED, COLOR_STRIP_STROKE) };
    let response = recessed_button(ui, label, text, stroke, size, tip);
    if *on {
        glow(ui, response.rect, color);
    }
    if response.clicked() {
        *on = !*on;
        return true;
    }
    false
}

/// The one toggle that deserves the eye: the same recessed button as every LED, but its label
/// and border are always in `color`; it gains the LED glow while its window is open.
pub fn primary_toggle(ui: &mut Ui, on: &mut bool, label: &str, color: Color32, size: Vec2, tip: &str) -> bool {
    let response = recessed_button(ui, label, color, color.gamma_multiply(0.7), size, tip);
    if *on {
        glow(ui, response.rect, color);
    }
    if response.clicked() {
        *on = !*on;
        return true;
    }
    false
}

/// A momentary action drawn exactly like an unlit LED, so every button label reads alike.
/// Returns true on click. Inside a disabled `Ui` egui fades the painter, so it greys out.
pub fn button(ui: &mut Ui, label: &str, size: Vec2, tip: &str) -> bool {
    let mut on = false;
    led(ui, &mut on, label, COLOR_TEXT_MUTED, size, tip)
}

/// A filled rectangular button of a given size: the soundboard pads.
pub fn tile_button(ui: &mut Ui, text: egui::RichText, fill: Color32, size: Vec2) -> Response {
    ui.add(egui::Button::new(text).fill(fill).rounding(Rounding::same(CONTROL_RADIUS)).min_size(size)).on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A stock slider with the pointer every other draggable control shows.
pub fn slider(ui: &mut Ui, slider: egui::Slider<'_>) -> Response {
    ui.add(slider).on_hover_cursor(egui::CursorIcon::PointingHand)
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

/// The field under a knob or fader for typing its value; open while its popup id is toggled on,
/// closing on a click anywhere else. Returns true when the value changed.
fn value_popup(ui: &mut Ui, response: &Response, label: &str, value: &mut f32, range: RangeInclusive<f32>) -> bool {
    let (min, max) = (*range.start(), *range.end());
    let mut changed = false;
    egui::popup_below_widget(ui, response.id.with("type-value"), response, egui::PopupCloseBehavior::CloseOnClickOutside, |ui| {
        ui.horizontal(|ui| {
            caption(ui, label);
            ui.add_space(SECTION_GAP);
            // DragValue sizes both its button and its text box from interact_size.x.
            ui.spacing_mut().interact_size.x = VALUE_FIELD_WIDTH;
            if ui.add(egui::DragValue::new(value).range(min..=max).speed((max - min) / 200.0).max_decimals(1)).changed() {
                changed = true;
            }
        });
    });
    changed
}

/// The dial's value as the console prints it: whole numbers without a decimal, else one place.
pub fn knob_value_text(value: f32) -> String {
    if (value - value.round()).abs() < 0.05 {
        format!("{:.0}", value.round())
    } else {
        format!("{value:.1}")
    }
}

/// A rotary knob that shows its value in the centre. Drag up or down to change, double-click to
/// reset to `reset`, click to type a value.
pub fn knob(ui: &mut Ui, value: &mut f32, range: RangeInclusive<f32>, reset: f32, label: &str, tip: &str) -> bool {
    let (min, max) = (*range.start(), *range.end());
    debug_assert!(max > min, "knob range must not be empty");
    let size = Vec2::new(KNOB_WIDTH, KNOB_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
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
    // A plain click (no drag) opens a small field to type the value. A double-click toggles it
    // twice, so a reset leaves it as it was.
    if response.clicked() {
        ui.memory_mut(|m| m.toggle_popup(response.id.with("type-value")));
    }
    changed |= value_popup(ui, &response, label, value, min..=max);
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
    // A dot at the rim marks the position; the value sits in the middle of the face.
    let pointer = Vec2::angled(start + sweep * t);
    let pointer_color = if t > 0.0 { COLOR_TEXT } else { COLOR_TEXT_MUTED };
    painter.circle_filled(center + pointer * (radius * KNOB_DOT_RADIUS_FRACTION), KNOB_DOT_RADIUS, pointer_color);
    painter.text(center, Align2::CENTER_CENTER, knob_value_text(*value), FontId::monospace(KNOB_VALUE_FONT_SIZE), COLOR_TEXT);
    painter.text(Pos2::new(center.x, rect.bottom() - 5.0), Align2::CENTER_CENTER, label, FontId::proportional(10.0), COLOR_TEXT_MUTED);
    response.on_hover_text(format!("{tip}\n{label}: {}. Drag up or down; click to type a value; double-click resets.", knob_value_text(*value)));
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

/// Vertical fader with a dB ruler and a rectangular cap. Double-click resets to 0 dB; a click on
/// the readout beneath the track opens a field to type the level.
pub fn fader(ui: &mut Ui, value: &mut f32, height: f32) -> bool {
    let size = Vec2::new(FADER_WIDTH, height + 2.0 * FADER_MARGIN);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let track_top = rect.top() + FADER_MARGIN;
    let track_bottom = rect.top() + height - FADER_MARGIN;
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
    let on_readout = response.interact_pointer_pos().is_some_and(|pos| pos.y > rect.bottom() - 2.0 * FADER_MARGIN);
    if response.clicked() && on_readout {
        ui.memory_mut(|m| m.toggle_popup(response.id.with("type-value")));
    }
    changed |= value_popup(ui, &response, "dB", value, FADER_MIN_DB..=FADER_MAX_DB);

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
    response.on_hover_text("Drag to set level in dB. Double-click resets to 0 dB; click the number to type a level.");
    changed
}

const PAN_CAP: Vec2 = Vec2::new(10.0, 14.0);
/// Room for the "L" and "R" letters at either end of the pan track.
const PAN_END_LABEL: f32 = 12.0;

/// Horizontal pan: a track with a centre notch and a cap. Drag left or right; double-click
/// centres it. `value` is -1 (left) .. +1 (right).
pub fn pan(ui: &mut Ui, value: &mut f32, width: f32) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, PAN_HEIGHT), Sense::click_and_drag());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let track_left = rect.left() + PAN_END_LABEL + PAN_CAP.x / 2.0;
    let track_right = rect.right() - PAN_END_LABEL - PAN_CAP.x / 2.0;
    let half = ((track_right - track_left) / 2.0).max(1.0);
    let mut changed = false;
    if response.dragged() {
        *value = (*value + response.drag_delta().x / half).clamp(-1.0, 1.0);
        changed = true;
    }
    if response.double_clicked() {
        *value = 0.0;
        changed = true;
    }

    let painter = ui.painter();
    let centre_x = (track_left + track_right) / 2.0;
    let y = rect.center().y;
    let font = FontId::proportional(9.0);
    painter.text(Pos2::new(rect.left() + 2.0, y), Align2::LEFT_CENTER, "L", font.clone(), COLOR_TEXT_MUTED);
    painter.text(Pos2::new(rect.right() - 2.0, y), Align2::RIGHT_CENTER, "R", font, COLOR_TEXT_MUTED);
    let track = Rect::from_min_max(Pos2::new(track_left, y - 3.0), Pos2::new(track_right, y + 3.0));
    painter.rect_filled(track, Rounding::same(2.0), COLOR_INSET);
    painter.rect_stroke(track, Rounding::same(2.0), Stroke::new(1.0_f32, COLOR_STRIP_STROKE));
    // Centre notch, like the fader's unity mark: taller than the track so it reads at a glance.
    painter.line_segment([Pos2::new(centre_x, rect.top() + 1.0), Pos2::new(centre_x, rect.bottom() - 1.0)], Stroke::new(1.0_f32, COLOR_TEXT_MUTED));
    let cap = Rect::from_center_size(Pos2::new(centre_x + *value * half, y), PAN_CAP);
    painter.rect_filled(cap, Rounding::same(2.0), COLOR_HANDLE);
    painter.line_segment([Pos2::new(cap.center().x, cap.top() + 3.0), Pos2::new(cap.center().x, cap.bottom() - 3.0)], Stroke::new(1.5_f32, COLOR_BG));
    response.on_hover_text(format!("Pan {}. Drag left or right; double-click centres.", pan_text(*value)));
    changed
}

/// "centre", "L 40" or "R 100": the pan position as the console prints it.
pub fn pan_text(value: f32) -> String {
    let percent = (value.abs() * 100.0).round() as u32;
    if percent == 0 {
        "centre".to_string()
    } else if value < 0.0 {
        format!("L {percent}")
    } else {
        format!("R {percent}")
    }
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

/// Two columns of LED segments, aligned with the fader track beside them. `width` is at least
/// METER_WIDTH; a wider meter just gets wider segments.
pub fn meter(ui: &mut Ui, peaks: [f32; 2], height: f32, width: f32) {
    let width = width.max(METER_WIDTH);
    let size = Vec2::new(width, height + 2.0 * FADER_MARGIN);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let top = rect.top() + FADER_MARGIN;
    let bottom = rect.top() + height - FADER_MARGIN;
    let well = Rect::from_min_max(Pos2::new(rect.left(), top - METER_WELL_OVERHANG), Pos2::new(rect.right(), bottom + METER_WELL_OVERHANG));
    let painter = ui.painter();
    painter.rect_filled(well, Rounding::same(2.0), COLOR_INSET);
    let seg_h = (bottom - top) / METER_SEGMENTS as f32;
    let col_w = (width - 6.0) / 2.0;
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
/// Drag anywhere; double-click resets both to zero. `threshold_y`, a share of the height, draws
/// the line above which the vertical axis starts to act, so a nudge near the bottom does nothing.
pub fn xy_pad(ui: &mut Ui, x: &mut f32, y: &mut f32, size: Vec2, labels: [&str; 3], threshold_y: Option<f32>, tip: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
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
    if let Some(threshold) = threshold_y {
        let line_y = rect.bottom() - 6.0 - threshold.clamp(0.0, 1.0) * (rect.height() - 12.0);
        painter.line_segment([Pos2::new(rect.left() + 4.0, line_y), Pos2::new(rect.right() - 4.0, line_y)], Stroke::new(1.0_f32, COLOR_THRESHOLD));
    }
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

/// One frame of a themed, font-loaded `Ui` at Retina density, for tests that measure how tall the
/// console's controls really render. egui's own test harness loads no fonts, so text has no
/// height there.
#[cfg(test)]
pub fn run_themed_test_ui(add_contents: impl Fn(&mut Ui)) {
    let ctx = egui::Context::default();
    apply_theme(&ctx);
    let input = egui::RawInput { viewport_id: egui::ViewportId::ROOT, ..Default::default() };
    ctx.set_pixels_per_point(2.0);
    let _ = ctx.run(input, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| add_contents(ui));
    });
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

    /// Fader, meter, the button column and (when present) the app list add up to the width.
    fn strip_bottom_fills(g: &Geometry) -> bool {
        let to_column = FADER_WIDTH + ITEM_SPACING + METER_WIDTH + LEVEL_BLOCK_GAP + g.side_button.x;
        (to_column + LEVEL_BLOCK_GAP + g.strip_apps - g.inner).abs() < 1e-3 || (g.strip_apps == 0.0 && to_column <= g.inner + 1e-3)
    }

    fn bus_row_fills(g: &Geometry) -> bool {
        let to_meter = g.bus_left + ITEM_SPACING + FADER_WIDTH + ITEM_SPACING + METER_WIDTH;
        (to_meter + LEVEL_BLOCK_GAP + g.bus_apps - g.inner).abs() < 1e-3 || (g.bus_apps == 0.0 && to_meter <= g.inner + 1e-3)
    }

    #[test]
    fn geometry_fills_its_column_exactly() {
        let g = Geometry::for_column(230.0);
        assert_eq!(g.inner, 210.0);
        assert!((g.xy_pad.x + ITEM_SPACING + KNOB_BLOCK_WIDTH - g.inner).abs() < 1e-4, "the colour pad and knob block span the column");
        assert_eq!(g.xy_pad.y, XY_PAD_HEIGHT);
        assert!((4.0 * g.fx_led.x + 3.0 * ITEM_SPACING + ROUNDING_SLACK - g.inner).abs() < 1e-4);
        assert!(strip_bottom_fills(&g), "strip bottom row must fill the column");
        assert!(bus_row_fills(&g), "bus row must fill the column");
        assert!(KNOB_WIDTH + ITEM_SPACING + g.bus_button.x <= g.bus_left + 1e-4, "LIMIT knob and MUTE fit the bus column");
        assert!((3.0 * g.pad.x + 2.0 * PAD_SPACING - g.left).abs() < 1e-4);
    }

    /// The send-to rows use fixed small LEDs; with their hints they must fit the narrowest column.
    #[test]
    fn route_rows_fit_the_minimum_column() {
        let widest_row = crate::NUM_HW_BUSES.max(crate::NUM_VIRT_BUSES) as f32;
        let row = widest_row * ROUTE_LED_WIDTH + (widest_row - 1.0) * ITEM_SPACING;
        assert!(row <= Geometry::for_column(MIN_COLUMN_WIDTH).inner);
        let player = Geometry::for_column(PLAYER_WINDOW_WIDTH + 2.0 * PANEL_PADDING);
        assert!(row <= player.left, "and beside the Player window's fader");
    }

    #[test]
    fn geometry_never_shrinks_below_the_minimum_column() {
        let g = Geometry::for_column(120.0);
        assert_eq!(g, Geometry::for_column(MIN_COLUMN_WIDTH));
        assert!(g.xy_pad.x >= XY_PAD_MIN_WIDTH - 1e-4, "the colour pad stays draggable: {}", g.xy_pad.x);
        assert!(g.pad.x >= 30.0);
    }

    #[test]
    fn the_fixed_window_fits_every_column_and_shows_the_app_lists() {
        let column = column_width(WINDOW_SIZE.x - 2.0 * SECTION_GAP, mixer_column_count());
        assert!(column >= MIN_COLUMN_WIDTH, "column {column} is narrower than the minimum {MIN_COLUMN_WIDTH}");
        let g = Geometry::for_column(column);
        assert!(g.strip_apps >= MIN_APP_LIST_WIDTH, "strips list their apps at the fixed size: {}", g.strip_apps);
        assert!(g.bus_apps >= MIN_APP_LIST_WIDTH, "buses list their apps at the fixed size: {}", g.bus_apps);
    }

    /// The Player window passes its own width plus the panel's own padding back into
    /// `for_column`, since `for_column` always subtracts padding once for the panel it draws.
    /// The strip inside the floating window must end up exactly `PLAYER_WINDOW_WIDTH` wide,
    /// matching every other column's panel.
    #[test]
    fn player_window_geometry_yields_its_declared_width() {
        let g = Geometry::for_column(PLAYER_WINDOW_WIDTH + 2.0 * PANEL_PADDING);
        assert_eq!(g.inner, PLAYER_WINDOW_WIDTH);
    }

    #[test]
    fn columns_share_the_width_with_gaps_between() {
        let w = column_width(1000.0, 7);
        let used = 7.0 * w + 6.0 * COLUMN_GAP + COLUMN_GAP + GROUP_DIVIDER_WIDTH + ITEM_SPACING;
        assert!((used - 1000.0).abs() < 1e-4);
    }

    #[test]
    fn column_width_for_a_single_column_leaves_one_spacing_spare() {
        let w = column_width(500.0, 1);
        assert_eq!(w, 500.0 - COLUMN_GAP - GROUP_DIVIDER_WIDTH - ITEM_SPACING);
    }

    /// Five columns is the Player row's own count of buses/strips grouping; the gap budget
    /// (one COLUMN_GAP per pair of columns, plus one more, the divider and the spare spacing)
    /// must exactly equal available minus the width handed to every column.
    #[test]
    fn column_width_budget_for_five_columns_equals_available_minus_gaps() {
        let available = 900.0;
        let w = column_width(available, 5);
        let gap_budget = 4.0 * COLUMN_GAP + COLUMN_GAP + GROUP_DIVIDER_WIDTH + ITEM_SPACING;
        assert_eq!(w, (available - gap_budget) / 5.0);
        let used = 5.0 * w + gap_budget;
        assert!((used - available).abs() < 1e-4);
    }

    #[test]
    fn panel_keeps_its_declared_width_when_content_requests_more_room() {
        egui::__run_test_ui(|ui| {
            let declared = 200.0;
            let rect = panel(ui, declared, 120.0, |ui| {
                ui.allocate_exact_size(Vec2::new(declared + 40.0, 300.0), Sense::hover());
            });
            assert!((rect.width() - declared).abs() < 1e-4, "panel expanded from {declared} to {} px", rect.width());
            assert!((rect.height() - 120.0).abs() < 1e-4, "panel grew to {} px tall", rect.height());
        });
    }

    /// The narrowest window the app claims to support: MIN_WINDOW.x px split across every mixer
    /// column. Every size the column derives from that width must still be usable, and a routing
    /// LED must be wide enough to read its label.
    #[test]
    fn geometry_at_the_minimum_window_width_keeps_every_derived_size_usable() {
        let column = column_width(MIN_WINDOW.x - 2.0 * SECTION_GAP, mixer_column_count());
        let g = Geometry::for_column(column);
        assert!(g.inner > 0.0, "inner width must be positive, got {}", g.inner);
        assert!(g.left > 0.0, "left width must be positive, got {}", g.left);
        assert!(g.led_triple.x > 0.0);
        assert!(g.fx_led.x > 0.0);
        assert!(g.xy_pad.x > 0.0);
        assert!(g.pad.x > 0.0);
    }

    /// At the narrowest column the app supports, both button columns and both meters must stay
    /// wide enough to click and read: no button collapses below 40 px, no meter below
    /// `METER_WIDTH`.
    #[test]
    fn geometry_at_min_column_width_keeps_buttons_and_meters_usable() {
        let g = Geometry::for_column(MIN_COLUMN_WIDTH);
        assert!(g.side_button.x >= 40.0, "side_button.x too narrow: {}", g.side_button.x);
        assert!(g.bus_button.x >= 40.0, "bus_button.x too narrow: {}", g.bus_button.x);
        assert!(strip_bottom_fills(&g) && bus_row_fills(&g));
        let wide = Geometry::for_column(320.0);
        assert!(wide.strip_apps >= MIN_APP_LIST_WIDTH && wide.bus_apps >= MIN_APP_LIST_WIDTH, "wide columns show the app list");
    }

    /// `strip_apps` and `bus_apps` are dropped-or-shown, never a size in between: as the column
    /// widens from the minimum, each field must read exactly 0.0 until it clears
    /// MIN_APP_LIST_WIDTH, then never fall back below it.
    #[test]
    fn app_list_width_is_never_between_zero_and_the_minimum() {
        let mut column = MIN_COLUMN_WIDTH;
        while column <= MIN_COLUMN_WIDTH + 400.0 {
            let g = Geometry::for_column(column);
            assert!(
                g.strip_apps == 0.0 || g.strip_apps >= MIN_APP_LIST_WIDTH,
                "strip_apps landed between zero and the minimum at column {column}: {}",
                g.strip_apps
            );
            assert!(
                g.bus_apps == 0.0 || g.bus_apps >= MIN_APP_LIST_WIDTH,
                "bus_apps landed between zero and the minimum at column {column}: {}",
                g.bus_apps
            );
            column += 1.0;
        }
    }

    /// A very wide window must not let the side buttons balloon past their cap; the meter should
    /// absorb the spare width instead.
    #[test]
    fn side_button_never_exceeds_its_max_width_at_very_wide_columns() {
        let g = Geometry::for_column(4000.0);
        assert!(g.side_button.x <= SIDE_BUTTON_MAX_WIDTH + 1e-4, "side_button.x grew past its cap: {}", g.side_button.x);
        assert!(g.bus_button.x <= SIDE_BUTTON_MAX_WIDTH + 1e-4, "bus_button.x grew past its cap: {}", g.bus_button.x);
    }

    /// `MIN_WINDOW.y` must stay exactly the sum the doc comment claims: chrome, both section
    /// captions and gaps, both fixed column heights, and both faders at their floor.
    #[test]
    fn min_window_y_matches_its_derived_formula() {
        let expected =
            CHROME_HEIGHT + 2.0 * (SECTION_GAP + SECTION_CAPTION_HEIGHT) + INPUT_FIXED_HEIGHT + OUTPUT_FIXED_HEIGHT + 2.0 * FADER_MIN_HEIGHT;
        assert_eq!(MIN_WINDOW.y, expected);
    }

    #[test]
    fn fader_height_fills_the_row_and_stays_within_its_limits() {
        assert_eq!(fader_height_in(INPUT_FIXED_HEIGHT + 150.0, INPUT_FIXED_HEIGHT), 150.0);
        assert_eq!(fader_height_in(OUTPUT_FIXED_HEIGHT + 150.0, OUTPUT_FIXED_HEIGHT), 150.0);
        assert_eq!(fader_height_in(50.0, INPUT_FIXED_HEIGHT), FADER_MIN_HEIGHT);
        assert_eq!(fader_height_in(5000.0, INPUT_FIXED_HEIGHT), FADER_MAX_HEIGHT);
    }

    /// A taller row must never hand back a shorter fader: the whole point of deriving the fader
    /// height from the row is that growing the window never shrinks a panel's content.
    #[test]
    fn fader_height_in_is_monotonic_in_row_height() {
        let fixed = INPUT_FIXED_HEIGHT;
        let mut previous = fader_height_in(0.0, fixed);
        let mut row = 20.0;
        while row <= 5000.0 {
            let height = fader_height_in(row, fixed);
            assert!(height >= previous, "fader_height_in({row}, {fixed}) = {height} is shorter than the previous row's {previous}");
            previous = height;
            row += 20.0;
        }
    }

    /// A panel whose content is shorter than its declared height must not shrink to fit the
    /// content: neighbouring columns rely on every panel in a row being exactly `row_height` tall.
    #[test]
    fn panel_keeps_its_declared_height_when_content_is_shorter() {
        egui::__run_test_ui(|ui| {
            let declared_height = 200.0;
            let rect = panel(ui, 100.0, declared_height, |ui| {
                ui.allocate_exact_size(Vec2::new(50.0, 20.0), Sense::hover());
            });
            assert!(
                (rect.height() - declared_height).abs() < 1e-4,
                "panel shrank from {declared_height} to {} px",
                rect.height()
            );
        });
    }

    /// Runs `pan` for one frame with the given pointer events and returns its rect and value.
    fn pan_frame(ctx: &egui::Context, value: &std::cell::Cell<f32>, events: Vec<egui::Event>, time: f64) -> Rect {
        let rect = std::cell::Cell::new(Rect::NOTHING);
        let input = egui::RawInput { events, time: Some(time), ..Default::default() };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                // A panel's min_rect already spans the screen, so take the control's own rect.
                let origin = ui.cursor().min;
                let mut v = value.get();
                pan(ui, &mut v, 120.0);
                value.set(v);
                rect.set(Rect::from_min_size(origin, Vec2::new(120.0, PAN_HEIGHT)));
            });
        });
        rect.get()
    }

    /// The whole reason `pan` is hand-drawn: egui's slider did not reset on double-click.
    #[test]
    fn pan_double_click_centres_it() {
        use egui::{Event, Modifiers, PointerButton};
        let ctx = egui::Context::default();
        apply_theme(&ctx);
        let value = std::cell::Cell::new(0.6);
        let rect = pan_frame(&ctx, &value, vec![], 0.0);
        let pos = rect.center();
        pan_frame(&ctx, &value, vec![Event::PointerMoved(pos)], 0.05);
        let click = |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        pan_frame(&ctx, &value, vec![click(true), click(false)], 0.10);
        assert_eq!(value.get(), 0.6, "a single click leaves the pan alone");
        // egui only counts a second click once its press and release land in separate frames.
        pan_frame(&ctx, &value, vec![click(true)], 0.15);
        pan_frame(&ctx, &value, vec![click(false)], 0.18);
        assert_eq!(value.get(), 0.0, "the second click of a double-click centres the pan");
    }

    #[test]
    fn knob_value_text_drops_the_decimal_for_whole_numbers() {
        assert_eq!(knob_value_text(0.0), "0");
        assert_eq!(knob_value_text(-12.0), "-12");
        assert_eq!(knob_value_text(9.96), "10");
        assert_eq!(knob_value_text(-0.5), "-0.5");
        assert_eq!(knob_value_text(3.25), "3.2");
    }

    /// Runs `knob` for one frame with the given pointer events and returns its rect.
    fn knob_frame(ctx: &egui::Context, value: &std::cell::Cell<f32>, events: Vec<egui::Event>, time: f64) -> Rect {
        let rect = std::cell::Cell::new(Rect::NOTHING);
        let input = egui::RawInput { events, time: Some(time), ..Default::default() };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let origin = ui.cursor().min;
                let mut v = value.get();
                knob(ui, &mut v, -12.0..=12.0, 0.0, "BASS", "");
                value.set(v);
                rect.set(Rect::from_min_size(origin, Vec2::new(KNOB_WIDTH, KNOB_HEIGHT)));
            });
        });
        rect.get()
    }

    /// A click opens the type-a-value field; a double-click (two clicks) leaves it closed and
    /// resets the value, so the two gestures never fight.
    #[test]
    fn knob_click_opens_the_value_field_and_double_click_resets() {
        use egui::{Event, Modifiers, PointerButton};
        let ctx = egui::Context::default();
        apply_theme(&ctx);
        let value = std::cell::Cell::new(4.0);
        let rect = knob_frame(&ctx, &value, vec![], 0.0);
        let pos = rect.center();
        knob_frame(&ctx, &value, vec![Event::PointerMoved(pos)], 0.05);
        let click = |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        knob_frame(&ctx, &value, vec![click(true), click(false)], 0.10);
        assert!(ctx.memory(|m| m.any_popup_open()), "a single click opens the value field");
        assert_eq!(value.get(), 4.0);
        knob_frame(&ctx, &value, vec![click(true)], 0.15);
        knob_frame(&ctx, &value, vec![click(false)], 0.18);
        assert_eq!(value.get(), 0.0, "the second click of a double-click resets the knob");
        assert!(!ctx.memory(|m| m.any_popup_open()), "and the field is closed again");
    }

    /// Typing an out-of-range value into the knob's popup DragValue clamps it to the knob's
    /// range, the same as dragging does. Finds the DragValue by scanning a grid of points below
    /// the knob for one that takes keyboard focus on click, since the popup's internal layout
    /// (caption width, then the DragValue) isn't exposed to callers.
    #[test]
    fn knob_popup_dragvalue_clamps_typed_values_to_the_range() {
        use egui::{Event, Modifiers, PointerButton};
        let ctx = egui::Context::default();
        apply_theme(&ctx);
        let value = std::cell::Cell::new(4.0);
        let rect = knob_frame(&ctx, &value, vec![], 0.0);
        let center = rect.center();
        knob_frame(&ctx, &value, vec![Event::PointerMoved(center)], 0.05);
        let click_at = |pos: Pos2, pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        knob_frame(&ctx, &value, vec![click_at(center, true), click_at(center, false)], 0.10);
        assert!(ctx.memory(|m| m.any_popup_open()), "a single click opens the value field");

        // The popup opens just below the knob, inset by the theme's menu margin (6px); scan a
        // generous area for the DragValue, which sits somewhere in that row after the caption.
        let popup_top_left = rect.left_bottom() + Vec2::splat(6.0);
        let mut t = 0.11;
        let mut found = false;
        'scan: for dy in (0..40).step_by(4) {
            for dx in (0..160).step_by(4) {
                let pos = popup_top_left + Vec2::new(dx as f32, dy as f32);
                t += 0.001;
                knob_frame(&ctx, &value, vec![Event::PointerMoved(pos), click_at(pos, true), click_at(pos, false)], t);
                if ctx.memory(|m| m.focused()).is_some() {
                    found = true;
                    break 'scan;
                }
            }
        }
        assert!(found, "the DragValue should take keyboard focus somewhere in the scanned popup area");

        // With focus on the DragValue, typing a value past the knob's range (-12..=12) must
        // clamp it, exactly like dragging does.
        t += 0.01;
        knob_frame(&ctx, &value, vec![Event::Text("99".to_string())], t);
        assert_eq!(value.get(), 12.0, "typing 99 into a -12..=12 knob must clamp to the max, not overshoot it");

        // And a large negative value clamps to the minimum.
        t += 0.01;
        // Select-all then retype, the way a user would clear the field first.
        knob_frame(&ctx, &value, vec![Event::Key { key: egui::Key::A, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND }], t);
        t += 0.01;
        knob_frame(&ctx, &value, vec![Event::Text("-99".to_string())], t);
        assert_eq!(value.get(), -12.0, "typing -99 into a -12..=12 knob must clamp to the min, not undershoot it");
    }

    /// Runs `fader` for one frame with the given pointer events and returns its rect.
    fn fader_frame(ctx: &egui::Context, value: &std::cell::Cell<f32>, height: f32, events: Vec<egui::Event>, time: f64) -> Rect {
        let rect = std::cell::Cell::new(Rect::NOTHING);
        let input = egui::RawInput { events, time: Some(time), ..Default::default() };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let origin = ui.cursor().min;
                let mut v = value.get();
                fader(ui, &mut v, height);
                value.set(v);
                rect.set(Rect::from_min_size(origin, Vec2::new(FADER_WIDTH, height + 2.0 * FADER_MARGIN)));
            });
        });
        rect.get()
    }

    /// A click on the readout strip beneath the track opens the type-a-value field; a click on
    /// the track itself (dragging the level) must not, or every drag would pop the field open.
    #[test]
    fn fader_click_on_the_readout_opens_the_popup_and_a_click_on_the_track_does_not() {
        use egui::{Event, Modifiers, PointerButton};
        let click_at = |pos: Pos2, pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };

        // The track: a click here changes the level but must leave the popup closed.
        let ctx = egui::Context::default();
        apply_theme(&ctx);
        let value = std::cell::Cell::new(-6.0);
        let height = 200.0;
        let rect = fader_frame(&ctx, &value, height, vec![], 0.0);
        let track_pos = Pos2::new(rect.center().x, rect.top() + FADER_MARGIN + 5.0);
        fader_frame(&ctx, &value, height, vec![Event::PointerMoved(track_pos)], 0.05);
        fader_frame(&ctx, &value, height, vec![click_at(track_pos, true), click_at(track_pos, false)], 0.10);
        assert!(!ctx.memory(|m| m.any_popup_open()), "a click on the track must not open the value field");

        // The readout: a click there must open it.
        let ctx = egui::Context::default();
        apply_theme(&ctx);
        let value = std::cell::Cell::new(-6.0);
        let rect = fader_frame(&ctx, &value, height, vec![], 0.0);
        let readout_pos = Pos2::new(rect.center().x, rect.bottom() - FADER_MARGIN);
        fader_frame(&ctx, &value, height, vec![Event::PointerMoved(readout_pos)], 0.05);
        fader_frame(&ctx, &value, height, vec![click_at(readout_pos, true), click_at(readout_pos, false)], 0.10);
        assert!(ctx.memory(|m| m.any_popup_open()), "a click on the readout must open the value field");
    }

    #[test]
    fn pan_text_reads_like_the_console() {
        assert_eq!(pan_text(0.0), "centre");
        assert_eq!(pan_text(0.004), "centre");
        assert_eq!(pan_text(-0.4), "L 40");
        assert_eq!(pan_text(1.0), "R 100");
    }

    #[test]
    fn labelled_slider_width_never_drops_below_its_floor() {
        for available in [-100.0, 0.0, 40.0, SLIDER_LABEL_WIDTH, SLIDER_LABEL_WIDTH + 39.0, 1000.0] {
            assert!(labelled_slider_width(available) >= 40.0);
        }
        assert_eq!(labelled_slider_width(SLIDER_LABEL_WIDTH + 100.0), 100.0);
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

    /// Every hardware bus (A1..) lights green; every virtual bus (B1..) lights blue, on either
    /// side of the boundary.
    #[test]
    fn bus_color_matches_hardware_and_virtual_buses() {
        for b in 0..crate::NUM_HW_BUSES {
            assert_eq!(bus_color(b), COLOR_ACTIVE, "hardware bus {b} should be COLOR_ACTIVE");
        }
        for b in crate::NUM_HW_BUSES..crate::NUM_BUSES {
            assert_eq!(bus_color(b), COLOR_VIRTUAL, "virtual bus {b} should be COLOR_VIRTUAL");
        }
    }

    /// `led`'s centred layout must still allocate exactly `size`, whether it sits in a row (as
    /// the send-to rows use it) or a column (as `state_column` and `routing_column` do).
    #[test]
    fn led_allocates_exactly_its_declared_size() {
        let size = Vec2::new(48.0, LED_HEIGHT);
        let used_size = std::cell::Cell::new(Vec2::ZERO);
        run_themed_test_ui(|ui| {
            ui.vertical(|ui| {
                let mut on = false;
                led(ui, &mut on, "MUTE", COLOR_MUTE, size, "");
                used_size.set(ui.min_rect().size());
            });
        });
        let got = used_size.get();
        assert!((got.x - size.x).abs() < 1e-2, "led allocated {} px wide, expected {}", got.x, size.x);
        assert!((got.y - size.y).abs() < 1e-2, "led allocated {} px tall, expected {}", got.y, size.y);
    }

    /// `record_button` allocates exactly `RECORD_BUTTON_SIZE`, whichever state (idle or
    /// recording) it paints its glyph in, since it draws the glyph itself rather than relying on
    /// button content to size the rect.
    #[test]
    fn record_button_allocates_its_declared_size_idle_and_recording() {
        for recording in [false, true] {
            let used_size = std::cell::Cell::new(Vec2::ZERO);
            run_themed_test_ui(|ui| {
                ui.vertical(|ui| {
                    record_button(ui, recording);
                    used_size.set(ui.min_rect().size());
                });
            });
            let got = used_size.get();
            assert!((got.x - RECORD_BUTTON_SIZE.x).abs() < 1e-2, "record_button (recording={recording}) allocated {} px wide, expected {}", got.x, RECORD_BUTTON_SIZE.x);
            assert!((got.y - RECORD_BUTTON_SIZE.y).abs() < 1e-2, "record_button (recording={recording}) allocated {} px tall, expected {}", got.y, RECORD_BUTTON_SIZE.y);
        }
    }

    /// `primary_toggle` shares `led`'s centred, fixed-size layout; it must allocate exactly its
    /// declared size on or off.
    #[test]
    fn primary_toggle_allocates_exactly_its_declared_size() {
        let size = Vec2::new(92.0, 22.0);
        let used_size = std::cell::Cell::new(Vec2::ZERO);
        run_themed_test_ui(|ui| {
            ui.vertical(|ui| {
                let mut on = true;
                primary_toggle(ui, &mut on, "PLAYER", COLOR_PLAYER, size, "");
                used_size.set(ui.min_rect().size());
            });
        });
        let got = used_size.get();
        assert!((got.x - size.x).abs() < 1e-2, "primary_toggle allocated {} px wide, expected {}", got.x, size.x);
        assert!((got.y - size.y).abs() < 1e-2, "primary_toggle allocated {} px tall, expected {}", got.y, size.y);
    }

    /// A hint after a row of buttons (the send-to rows) must sit the same `LEVEL_BLOCK_GAP` off
    /// the last button that the app list keeps off the level block, so both reads alike.
    #[test]
    fn row_hint_after_a_button_row_keeps_the_level_block_gap() {
        let led_size = Vec2::new(48.0, LED_HEIGHT);
        let text = "test hint";
        // The hint's own rendered width, measured with nothing before it.
        let hint_only_width = std::cell::Cell::new(0.0);
        run_themed_test_ui(|ui| {
            ui.horizontal(|ui| {
                hint(ui, text);
                hint_only_width.set(ui.min_rect().width());
            });
        });
        let led_right = std::cell::Cell::new(0.0);
        let hint_left = std::cell::Cell::new(0.0);
        run_themed_test_ui(|ui| {
            ui.horizontal(|ui| {
                let mut on = false;
                led(ui, &mut on, "MUTE", COLOR_MUTE, led_size, "");
                led_right.set(ui.min_rect().right());
                row_hint(ui, text);
                // The hint is the last thing drawn, so its left edge is the row's right edge
                // less its own width.
                hint_left.set(ui.min_rect().right() - hint_only_width.get());
            });
        });
        let gap = hint_left.get() - led_right.get();
        assert!(
            (gap - LEVEL_BLOCK_GAP).abs() < 1e-2,
            "row_hint left a {gap} px gap after the led's right edge ({}), expected LEVEL_BLOCK_GAP ({LEVEL_BLOCK_GAP})",
            led_right.get()
        );
    }

    /// `led` and `button` (a `led` with an always-off state) allocate exactly their declared
    /// size, and keep doing so once the pointer has hovered and the control has repainted with
    /// `recessed_button`'s hover branch active — hovering must change colour, never the rect.
    #[test]
    fn led_and_button_allocate_their_declared_size_after_a_repaint() {
        let size = Vec2::new(48.0, LED_HEIGHT);
        let ctx = egui::Context::default();
        apply_theme(&ctx);
        ctx.set_pixels_per_point(2.0);

        let measure = |ctx: &egui::Context, events: Vec<egui::Event>| -> (Rect, Vec2, Vec2) {
            let led_rect = std::cell::Cell::new(Rect::NOTHING);
            let led_size = std::cell::Cell::new(Vec2::ZERO);
            let button_size = std::cell::Cell::new(Vec2::ZERO);
            let input = egui::RawInput { events, ..Default::default() };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.vertical(|ui| {
                        let origin = ui.cursor().min;
                        let mut on = false;
                        led(ui, &mut on, "MUTE", COLOR_MUTE, size, "");
                        led_size.set(ui.min_rect().size());
                        led_rect.set(Rect::from_min_size(origin, size));
                    });
                    ui.vertical(|ui| {
                        button(ui, "REFRESH", size, "");
                        button_size.set(ui.min_rect().size());
                    });
                });
            });
            (led_rect.get(), led_size.get(), button_size.get())
        };

        let (rect, led_first, button_first) = measure(&ctx, vec![]);
        // A second (and third) frame with the pointer already resting on the led, as egui
        // repaints every frame the pointer doesn't move; this exercises recessed_button's
        // hover-fill branch inside a repaint rather than the widget's first frame.
        let (_, led_hovered, button_hovered) = measure(&ctx, vec![egui::Event::PointerMoved(rect.center())]);
        let (_, led_hovered_again, button_hovered_again) = measure(&ctx, vec![egui::Event::PointerMoved(rect.center())]);

        for (label, got) in [
            ("led first frame", led_first),
            ("led hovered", led_hovered),
            ("led hovered again", led_hovered_again),
        ] {
            assert!((got.x - size.x).abs() < 1e-2 && (got.y - size.y).abs() < 1e-2, "{label} allocated {got:?}, expected {size:?}");
        }
        for (label, got) in [
            ("button first frame", button_first),
            ("button hovered", button_hovered),
            ("button hovered again", button_hovered_again),
        ] {
            assert!((got.x - size.x).abs() < 1e-2 && (got.y - size.y).abs() < 1e-2, "{label} allocated {got:?}, expected {size:?}");
        }
    }

    /// `recessed_button` wraps its response with `on_hover_cursor`, which must return the same
    /// response, not a new one: a `led` that has already been hovered (so the hover-fill branch
    /// above has run and repainted) still toggles on a plain click.
    #[test]
    fn hovered_led_still_toggles_on_click() {
        use egui::{Event, Modifiers, PointerButton};
        let ctx = egui::Context::default();
        apply_theme(&ctx);
        let size = Vec2::new(48.0, LED_HEIGHT);
        let on = std::cell::Cell::new(false);
        let rect = std::cell::Cell::new(Rect::NOTHING);
        let toggled = std::cell::Cell::new(false);

        let frame = |events: Vec<egui::Event>| {
            let input = egui::RawInput { events, ..Default::default() };
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let origin = ui.cursor().min;
                    let mut v = on.get();
                    let t = led(ui, &mut v, "MUTE", COLOR_MUTE, size, "");
                    on.set(v);
                    toggled.set(t);
                    rect.set(Rect::from_min_size(origin, size));
                });
            });
        };

        frame(vec![]);
        let pos = rect.get().center();
        // Hover first, and repaint once more with the pointer still resting on it, so
        // recessed_button's hover-fill branch is active before the click lands.
        frame(vec![Event::PointerMoved(pos)]);
        frame(vec![Event::PointerMoved(pos)]);
        let click = |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        frame(vec![click(true), click(false)]);
        assert!(toggled.get(), "a hovered led should still report clicked()");
        assert!(on.get(), "a hovered led should still toggle on click");
    }

    /// Same guarantee as above for `knob`: hovering (and so painting through
    /// `on_hover_cursor(PointingHand)`) must not eat the double-click that resets it.
    #[test]
    fn hovered_knob_still_resets_on_double_click() {
        use egui::{Event, Modifiers, PointerButton};
        let ctx = egui::Context::default();
        apply_theme(&ctx);
        let value = std::cell::Cell::new(4.0);
        let rect = knob_frame(&ctx, &value, vec![], 0.0);
        let pos = rect.center();
        // Hover first, and repaint once more with the pointer still resting on it.
        knob_frame(&ctx, &value, vec![Event::PointerMoved(pos)], 0.05);
        knob_frame(&ctx, &value, vec![Event::PointerMoved(pos)], 0.06);
        let click = |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
        knob_frame(&ctx, &value, vec![click(true), click(false)], 0.10);
        assert_eq!(value.get(), 4.0, "a single click leaves the knob's value alone");
        // egui only counts a second click once its press and release land in separate frames.
        knob_frame(&ctx, &value, vec![click(true)], 0.15);
        knob_frame(&ctx, &value, vec![click(false)], 0.18);
        assert_eq!(value.get(), 0.0, "a hovered knob should still reset on the second click of a double-click");
    }
}
