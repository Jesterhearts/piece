use convert_case::{Case, Casing};
use egui::{
    vec2, Color32, Frame, Label, Layout, PointerButton, RichText, ScrollArea, Sense, Stroke, Widget,
};
use indexmap::IndexMap;
use itertools::Itertools;

use piece_lib::{
    effects::PendingEffects,
    in_play::{CardId, Database},
    player::Owner,
    protogen::{keywords::Keyword, targets::Location},
    stack::{Selected, StackEntry, StackId},
    turns::Turn,
};
use protobuf::Enum;

pub struct Card<'db> {
    pub db: &'db Database,
    pub card: CardId,
    pub highlight: bool,
}

impl Widget for Card<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        if self.card.tapped(self.db) {
            ui.style_mut().visuals.widgets.active = ui.style().visuals.widgets.noninteractive;
            ui.style_mut().visuals.widgets.hovered = ui.style().visuals.widgets.noninteractive;
            ui.style_mut().visuals.widgets.inactive = ui.style().visuals.widgets.noninteractive;
        }

        let name = if self.db[self.card].manifested {
            "Manifested".to_string()
        } else if self.db[self.card].cloning.is_some() {
            format!(
                "({}) {}",
                self.card.faceup_face(self.db).name,
                self.card.name(self.db),
            )
        } else {
            self.card.name(self.db).clone()
        };

        let cost = &self.db[self.card].modified_cost;

        let title = if cost.mana_cost.is_empty() || self.db[self.card].manifested {
            name
        } else {
            format!("{} - {}", name, cost.text())
        };

        let source = &self.db[self.card];
        let typeline = std::iter::once(
            source
                .modified_types
                .iter()
                .map(|ty| ty.as_ref().to_case(Case::Title))
                .join(" "),
        )
        .chain(
            std::iter::once(
                source
                    .modified_subtypes
                    .iter()
                    .map(|ty| ty.as_ref().to_case(Case::Title))
                    .join(" "),
            )
            .filter(|s| !s.is_empty()),
        )
        .join(" - ");

        let oracle_text = self.card.faceup_face(self.db).oracle_text.clone();
        let has_oracle_text = !oracle_text.is_empty();

        let etb_text = source
            .modified_etb_abilities
            .iter()
            .map(|ability| &ability.oracle_text)
            .filter(|text| !text.is_empty())
            .cloned()
            .collect_vec();
        let has_etb_text = !etb_text.is_empty();

        let effects_text = self
            .card
            .faceup_face(self.db)
            .effects
            .iter()
            .map(|effect| &effect.oracle_text)
            .filter(|text| !text.is_empty())
            .cloned()
            .collect_vec();
        let has_effects_text = !effects_text.is_empty();

        let triggers = source
            .modified_triggers
            .values()
            .flat_map(|triggers| triggers.iter())
            .map(|trigger| &trigger.oracle_text)
            .filter(|text| !text.is_empty())
            .cloned()
            .collect_vec();
        let has_triggers = !triggers.is_empty();

        let abilities = source
            .abilities(self.db)
            .iter()
            .map(|(_, ability)| ability.text(self.db))
            .filter(|text| !text.is_empty())
            .collect_vec();
        let has_abilities = !abilities.is_empty();

        let keywords = source
            .modified_keywords
            .keys()
            .map(|k| Keyword::from_i32(*k).unwrap().as_ref().to_case(Case::Title))
            .join(", ");
        let has_keywords = !keywords.is_empty();

        let modified_by = self.card.modified_by_text(self.db);
        let is_modified = !modified_by.is_empty();

        let counters = source.counter_text_on();
        let has_counters = !counters.is_empty();

        let paragraph = std::iter::once(oracle_text)
            .chain(std::iter::once(String::default()).filter(|_| has_oracle_text))
            .chain(etb_text)
            .chain(std::iter::once(String::default()).filter(|_| has_etb_text))
            .chain(effects_text)
            .chain(std::iter::once(String::default()).filter(|_| has_effects_text))
            .chain(triggers)
            .chain(std::iter::once(String::default()).filter(|_| has_triggers))
            .chain(std::iter::once(keywords).filter(|_| has_keywords))
            .chain(std::iter::once(String::default()).filter(|_| has_keywords))
            .chain(abilities)
            .chain(std::iter::once(String::default()).filter(|_| has_abilities))
            .chain(std::iter::once("Modified by:".to_string()).filter(|_| is_modified))
            .chain(modified_by)
            .chain(std::iter::once(String::default()).filter(|_| is_modified))
            .chain(std::iter::once("Counters:".to_string()).filter(|_| has_counters))
            .chain(counters.into_iter().map(|counter| format!("  {}", counter)))
            .join("\n");

        Frame::none()
            .fill(Color32::from_hex("#141414").unwrap())
            .rounding(10.0)
            .stroke(Stroke::new(
                2.0,
                if self.highlight {
                    Color32::DARK_BLUE
                } else if self.card.summoning_sick(self.db) {
                    Color32::LIGHT_BLUE
                } else {
                    Color32::DARK_GRAY
                },
            ))
            .inner_margin(5.0)
            .outer_margin(2.0)
            .show(ui, |ui| {
                ui.expand_to_include_rect(ui.max_rect());
                ui.vertical(|ui| {
                    ui.add(Label::new(RichText::new(title).heading()));
                    ui.separator();

                    ScrollArea::vertical().id_source(self.card).show(ui, |ui| {
                        ui.add(Label::new(paragraph));
                    });

                    ui.separator();
                    ui.add(Label::new(typeline));

                    if let Some(pt) = self.card.pt_text(self.db) {
                        ui.with_layout(Layout::right_to_left(egui::Align::Min), |ui| {
                            ui.add(Label::new(pt));
                        });
                    }
                });
            })
            .response
    }
}

#[derive(Debug)]
pub struct ManaDisplay {
    pub player: Owner,
    pub items: Vec<String>,
}

impl Widget for ManaDisplay {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        Frame::none()
            .stroke(Stroke::new(2.0, Color32::DARK_GRAY))
            .inner_margin(5.0)
            .outer_margin(2.0)
            .show(ui, |ui| {
                ui.expand_to_include_rect(ui.max_rect());
                ScrollArea::vertical()
                    .id_source(self.player)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            for item in self.items {
                                ui.heading(item);
                            }
                        });
                    });
            })
            .response
    }
}

pub struct Stack<'stack, 'db, 'clicked> {
    pub items: &'stack IndexMap<StackId, StackEntry>,
    pub db: &'db Database,
    pub left_clicked: &'clicked mut Option<usize>,
    pub target: Option<Selected>,
}

impl Widget for Stack<'_, '_, '_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        Frame::none()
            .stroke(Stroke::new(2.0, Color32::DARK_GRAY))
            .inner_margin(5.0)
            .outer_margin(2.0)
            .show(ui, |ui| {
                ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
                    ui.heading("Stack");
                    ui.separator();
                    ui.expand_to_include_rect(ui.max_rect());
                    ScrollArea::vertical()
                        .id_source("Stack")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
                                for (idx, (stack_id, entry)) in self.items.iter().rev().enumerate()
                                {
                                    let highlight =
                                        if let Some(Selected::Stack { id }) = self.target {
                                            id == *stack_id
                                        } else {
                                            false
                                        };

                                    let text = RichText::new(entry.display(self.db));
                                    let text = if highlight {
                                        text.color(Color32::DARK_BLUE)
                                    } else {
                                        text
                                    };

                                    if ui.add(Label::new(text).sense(Sense::click())).clicked() {
                                        *self.left_clicked = Some(idx);
                                    }
                                }
                            })
                        });
                });
            })
            .response
    }
}

pub struct Exile<'clicked> {
    pub player: Owner,
    pub cards: Vec<String>,
    pub right_clicked: &'clicked mut Option<usize>,
}

impl Widget for Exile<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        Frame::none()
            .stroke(Stroke::new(2.0, Color32::DARK_GRAY))
            .inner_margin(5.0)
            .outer_margin(2.0)
            .show(ui, |ui| {
                ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
                    ui.heading("Exile");
                    ui.separator();
                    ui.expand_to_include_rect(ui.max_rect());
                    ScrollArea::vertical()
                        .id_source(format!("exile {:?}", self.player))
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
                                for (idx, item) in self.cards.into_iter().enumerate() {
                                    if ui.add(Label::new(item).sense(Sense::click())).clicked() {
                                        *self.right_clicked = Some(idx);
                                    }
                                }
                            })
                        });
                });
            })
            .response
    }
}

pub struct Graveyard<'clicked> {
    pub player: Owner,
    pub cards: Vec<String>,
    pub right_clicked: &'clicked mut Option<usize>,
}

impl Widget for Graveyard<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        Frame::none()
            .stroke(Stroke::new(2.0, Color32::DARK_GRAY))
            .inner_margin(5.0)
            .outer_margin(2.0)
            .show(ui, |ui| {
                ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
                    ui.heading("Graveyard");
                    ui.separator();
                    ui.expand_to_include_rect(ui.max_rect());
                    ScrollArea::vertical()
                        .id_source(format!("graveyard {:?}", self.player))
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
                                for (idx, item) in self.cards.into_iter().enumerate() {
                                    if ui.add(Label::new(item).sense(Sense::click())).clicked() {
                                        *self.right_clicked = Some(idx);
                                    }
                                }
                            })
                        });
                });
            })
            .response
    }
}

pub struct Hand<'db, 'clicked> {
    pub db: &'db Database,
    pub owner: Owner,
    pub cards: Vec<CardId>,
    pub hovered: &'clicked mut Option<usize>,
    pub left_clicked: &'clicked mut Option<usize>,
    pub right_clicked: &'clicked mut Option<usize>,
}

impl Widget for Hand<'_, '_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        Frame::none()
            .stroke(Stroke::new(2.0, Color32::DARK_GRAY))
            .inner_margin(5.0)
            .outer_margin(2.0)
            .show(ui, |ui| {
                ui.expand_to_include_rect(ui.max_rect());
                ui.horizontal(|ui| {
                    ScrollArea::horizontal().id_source("Hand").show(ui, |ui| {
                        const MIN_WIDTH: f32 = 200.0;
                        const MIN_HEIGHT: f32 = 300.0;
                        let mut rects = vec![];

                        let mut hovered = false;
                        for index in 0..self.cards.len() {
                            let (rect, sense) =
                                ui.allocate_exact_size(vec2(MIN_WIDTH, MIN_HEIGHT), Sense::click());

                            rects.push(rect);
                            if sense.hovered() {
                                hovered = true;
                                *self.hovered = Some(index);
                            };

                            if sense.clicked_by(PointerButton::Primary) {
                                *self.left_clicked = Some(index);
                            } else if sense.clicked_by(PointerButton::Secondary) {
                                *self.right_clicked = Some(index);
                            }
                        }

                        for (index, (mut rect, card)) in
                            rects.into_iter().zip(self.cards).enumerate()
                        {
                            if Some(index) == *self.hovered {
                                rect = rect.translate(vec2(0.0, -MIN_HEIGHT));
                                let sense = ui.allocate_rect(rect, Sense::click());
                                if sense.hovered() {
                                    hovered = true;
                                }

                                if sense.clicked_by(PointerButton::Primary) {
                                    *self.left_clicked = Some(index);
                                } else if sense.clicked_by(PointerButton::Secondary) {
                                    *self.right_clicked = Some(index);
                                }
                            }
                            ui.put(
                                rect,
                                Card {
                                    db: self.db,
                                    card,
                                    highlight: false,
                                },
                            );
                        }
                        if !hovered {
                            *self.hovered = None;
                        }
                    });
                });
            })
            .response
    }
}

pub struct Battlefield<'db, 'clicked> {
    pub db: &'db Database,
    pub player: Owner,
    pub cards: Vec<(usize, CardId)>,
    pub left_clicked: &'clicked mut Option<usize>,
    pub right_clicked: &'clicked mut Option<usize>,
    pub target: Option<Selected>,
}

impl Widget for Battlefield<'_, '_> {
    fn ui(mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.expand_to_include_rect(ui.max_rect());
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .id_source(self.player)
            .show(ui, |ui| {
                ui.with_layout(
                    Layout::left_to_right(egui::Align::Min).with_main_wrap(true),
                    |ui| {
                        self.cards.sort_by_cached_key(|(_, card)| {
                            let mut types =
                                self.db[*card].modified_types.iter().cloned().collect_vec();
                            types.sort();
                            let mut subtypes = self.db[*card]
                                .modified_subtypes
                                .iter()
                                .cloned()
                                .collect_vec();
                            subtypes.sort();
                            (types, subtypes)
                        });

                        const MIN_WIDTH: f32 = 200.0;
                        const MIN_HEIGHT: f32 = 300.0;

                        for (idx, card) in self.cards {
                            let highlight = if let Some(Selected::Battlefield { id }) = self.target
                            {
                                id == card
                            } else {
                                false
                            };

                            let (rect, sense) =
                                ui.allocate_exact_size(vec2(MIN_WIDTH, MIN_HEIGHT), Sense::click());
                            ui.put(
                                rect,
                                Card {
                                    db: self.db,
                                    card,
                                    highlight,
                                },
                            );

                            if sense.clicked_by(PointerButton::Primary) {
                                *self.left_clicked = Some(idx)
                            } else if sense.clicked_by(PointerButton::Secondary) {
                                *self.right_clicked = Some(idx);
                            }
                        }
                    },
                )
                .response
            })
            .inner
    }
}

pub struct Actions<'db, 'p, 'clicked> {
    pub db: &'db Database,
    pub player: Owner,
    pub card: Option<CardId>,
    pub pending: &'p Option<PendingEffects>,
    pub left_clicked: &'clicked mut Option<usize>,
}

impl Widget for Actions<'_, '_, '_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let abilities = if let Some(card) = self.card {
            if card.is_in_location(self.db, Location::IN_HAND) && Turn::can_cast(self.db, card) {
                [(0, format!("Play {}", card.name(self.db)))]
                    .into_iter()
                    .chain(
                        self.db[card]
                            .abilities(self.db)
                            .into_iter()
                            .enumerate()
                            .filter_map(|(idx, (_, ability))| {
                                if ability.can_be_activated(
                                    self.db,
                                    card,
                                    self.player,
                                    self.pending,
                                ) {
                                    Some((idx + 1, ability.text(self.db)))
                                } else {
                                    None
                                }
                            }),
                    )
                    .collect_vec()
            } else {
                self.db[card]
                    .abilities(self.db)
                    .into_iter()
                    .enumerate()
                    .filter_map(|(idx, (_, ability))| {
                        if ability.can_be_activated(self.db, card, self.player, self.pending) {
                            Some((idx, ability.text(self.db)))
                        } else {
                            None
                        }
                    })
                    .collect_vec()
            }
        } else {
            vec![]
        };

        Frame::none()
            .stroke(Stroke::new(2.0, Color32::DARK_GRAY))
            .inner_margin(5.0)
            .outer_margin(2.0)
            .show(ui, |ui| {
                ui.expand_to_include_rect(ui.max_rect());

                ui.horizontal(|ui| {
                    ScrollArea::horizontal()
                        .id_source("Actions")
                        .show(ui, |ui| {
                            for (index, action) in abilities.into_iter() {
                                if ui.button(action).clicked() {
                                    *self.left_clicked = Some(index);
                                };
                                ui.separator();
                            }
                        });
                });
            })
            .response
    }
}
