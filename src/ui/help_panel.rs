//! A plain-language guide to inputs, outputs and the A/B routing buttons, in a floating window.

use super::widgets::{self, COLOR_ACTIVE, COLOR_VIRTUAL};

const HELP_WIDTH: f32 = 560.0;
/// Shown at the foot of the guide.
const LICENCE_NOTE: &str = "DHMIX is free and open source software under the MIT licence.";

/// (heading, paragraphs) for each section of the guide.
const SECTIONS: &[(&str, &[&str])] = &[
    ("The idea", &["A mixer controls where every sound goes."]),
    (
        "Sound comes in through inputs",
        &[
            "•  Hardware inputs: real devices like your mic, guitar, or console.",
            "•  Virtual inputs: desktop apps like games, Discord, Spotify, or a browser.",
            "•  Player: music and sound clips loaded into DHMIX.",
        ],
    ),
    (
        "Sound leaves through outputs",
        &["•  A1–A3: your headphones or speakers.", "•  B1–B2: other apps, such as OBS, Discord, or a call."],
    ),
    (
        "The buttons choose where each sound goes",
        &["•  A = you hear it.", "•  B = your stream, recording, or call hears it."],
    ),
    (
        "Sending a desktop app to your stream",
        &[
            "For example, if you want Spotify, a game, or any desktop app to reach your stream:",
            "1.  Set that app's audio output to a DHMIX Virtual Input.",
            "2.  Find that Virtual Input strip in DHMIX.",
            "3.  Turn on B1.",
            "4.  In OBS, Discord, or your call app, select the DHMIX B1 virtual output as the microphone / input.",
        ],
    ),
    (
        "Simple streaming setup",
        &[
            "•  Mic: A1 + B1 — you hear yourself; stream hears you.",
            "•  Game: A1 only — you hear it; OBS can capture it separately.",
            "•  Discord friends: A1; enable B1 if viewers should hear them.",
            "•  Music / desktop app: A1 + B1 — you and viewers hear it.",
        ],
    ),
    (
        "Other controls",
        &[
            "•  Fader: volume.",
            "•  Mute: turns that sound off everywhere.",
            "•  Solo: lets you hear only that sound.",
            "•  Mono: centres a single mic.",
            "•  Applications: shows audio apps and lets you move each one to another input / device.",
        ],
    ),
];

pub struct HelpPanel {
    pub open: bool,
}

impl HelpPanel {
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }
        let mut open = self.open;
        egui::Window::new("How DHMIX works").open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
            ui.set_max_width(HELP_WIDTH);
            for (heading, paragraphs) in SECTIONS {
                widgets::section(ui, heading);
                for paragraph in *paragraphs {
                    ui.label(*paragraph);
                }
            }
            ui.add_space(widgets::SECTION_GAP);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("A = you hear it").strong().color(COLOR_ACTIVE));
                ui.label(egui::RichText::new("B = your stream, recording, or call hears it").strong().color(COLOR_VIRTUAL));
            });
            ui.add_space(widgets::SECTION_GAP);
            widgets::hint(ui, LICENCE_NOTE);
        });
        self.open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_window_renders_without_panicking() {
        let ctx = egui::Context::default();
        let mut panel = HelpPanel { open: true };
        for _ in 0..3 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| panel.show(ctx));
        }
    }

    #[test]
    fn guide_covers_inputs_outputs_and_the_routing_rule() {
        let text: String = SECTIONS.iter().flat_map(|(h, ps)| std::iter::once(*h).chain(ps.iter().copied())).collect();
        for needed in ["Hardware inputs", "Virtual inputs", "Player", "A1–A3", "B1–B2", "A = you hear it", "Turn on B1"] {
            assert!(text.contains(needed), "guide should mention {needed}");
        }
    }

    /// "Other controls" is the one section documenting the per-strip buttons; a reader hunting
    /// for what MUTE, SOLO or MONO does, or where Applications lives, must find it there.
    #[test]
    fn guide_documents_mute_solo_mono_and_applications() {
        let text: String = SECTIONS.iter().flat_map(|(h, ps)| std::iter::once(*h).chain(ps.iter().copied())).collect();
        for needed in ["Mute", "Solo", "Mono", "Applications"] {
            assert!(text.contains(needed), "guide should mention {needed}");
        }
    }

    #[test]
    fn every_section_has_a_non_empty_heading_and_at_least_one_paragraph() {
        assert!(!SECTIONS.is_empty(), "guide should have at least one section");
        for (heading, paragraphs) in SECTIONS {
            assert!(!heading.trim().is_empty(), "section heading should not be blank");
            assert!(!paragraphs.is_empty(), "section '{heading}' should have at least one paragraph");
        }
    }

    #[test]
    fn no_paragraph_is_empty_or_whitespace_only() {
        for (heading, paragraphs) in SECTIONS {
            for paragraph in *paragraphs {
                assert!(!paragraph.trim().is_empty(), "section '{heading}' has a blank paragraph");
            }
        }
    }

    /// The licence note at the foot of the guide must actually name the licence, or a reader
    /// has no way to know DHMIX is MIT-licensed without leaving the app.
    #[test]
    fn licence_note_mentions_mit() {
        assert!(LICENCE_NOTE.contains("MIT"), "licence note should mention MIT: {LICENCE_NOTE}");
    }

    #[test]
    fn section_headings_are_unique() {
        let mut headings: Vec<&str> = SECTIONS.iter().map(|(h, _)| *h).collect();
        let original_len = headings.len();
        headings.sort_unstable();
        headings.dedup();
        assert_eq!(headings.len(), original_len, "section headings should be unique");
    }
}
