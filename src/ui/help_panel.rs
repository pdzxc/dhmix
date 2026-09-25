//! A plain-language guide to inputs, outputs and the A/B routing buttons, in a floating window.

use super::widgets::{self, COLOR_ACTIVE, COLOR_VIRTUAL};

const HELP_WIDTH: f32 = 560.0;

/// (heading, paragraphs) for each section of the guide.
const SECTIONS: &[(&str, &[&str])] = &[
    (
        "The idea",
        &["Think of the mixer as a room with doors. Sound comes in through the inputs, you shape it, and the buttons on each input decide which doors it leaves through."],
    ),
    (
        "Inputs: where sound comes in",
        &[
            "Hardware input 1, 2, 3 are real things plugged into your PC: your microphone, a second mic, a guitar or a console line. Pick the device under the name.",
            "Virtual input 1 and 2 are programs on your PC: the game, Discord, Spotify. They cannot plug in with a wire, so they play into a virtual cable, and the cable's other end shows up here.",
            "Player is music and sound clips you load into DHMIX itself.",
        ],
    ),
    (
        "Outputs: where the mix goes out",
        &[
            "A1 to A3 (hardware out) go to things you hear with your ears: headphones, speakers, a second pair of headphones for a guest. A1 is usually your headphones.",
            "B1 and B2 (virtual out) go to programs, not ears. B1 is what OBS or Discord hears as your microphone. Nothing physical is connected; it is a cable into another program.",
        ],
    ),
    (
        "The A / B buttons are the doors",
        &[
            "A1 on for the mic means \"I hear my own mic in my headphones\". B1 on for the mic means \"the stream hears my mic\". Several doors can be open at once, or none.",
            "Rule of thumb: A means to my ears, B means to the stream or call.",
        ],
    ),
    (
        "A typical streaming setup",
        &[
            "Mic: A1 and B1. You hear yourself, viewers hear you.",
            "Game: A1 only. You hear it; OBS captures game sound by itself.",
            "Discord friends: A1 so you hear them. Add B1 only if viewers should hear them too.",
            "Music: A1 and B1 so everyone hears it. Turn B1 off during a call where music should stay private.",
        ],
    ),
    (
        "Everything else on a strip",
        &[
            "The fader is how loud. The knobs and the colour pad are how it sounds. MUTE silences it everywhere, SOLO lets you hear only that one, MONO puts a single mic in the centre.",
            "Applications, in the top bar, shows which programs play or record audio and lets you move each one to a different device.",
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
                ui.label(egui::RichText::new("A = to my ears").strong().color(COLOR_ACTIVE));
                ui.label(egui::RichText::new("B = to the stream or call").strong().color(COLOR_VIRTUAL));
            });
        });
        self.open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_covers_inputs_outputs_and_the_routing_rule() {
        let text: String = SECTIONS.iter().flat_map(|(h, ps)| std::iter::once(*h).chain(ps.iter().copied())).collect();
        for needed in ["Hardware input", "Virtual input", "Player", "A1 to A3", "B1 and B2", "A means to my ears"] {
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

    #[test]
    fn section_headings_are_unique() {
        let mut headings: Vec<&str> = SECTIONS.iter().map(|(h, _)| *h).collect();
        let original_len = headings.len();
        headings.sort_unstable();
        headings.dedup();
        assert_eq!(headings.len(), original_len, "section headings should be unique");
    }
}
