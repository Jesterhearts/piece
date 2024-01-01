use egui::{
    Color32, Frame, Label, Layout, PointerButton, RichText, ScrollArea, Sense, Stroke, Widget,
};
use itertools::Itertools;

use crate::{
    in_play::{CardId, CounterId, Database, InHand},
    player::{AllPlayers, Owner},
    turns::Turn,
};

pub struct Card<'db> {
    pub db: &'db mut Database,
    pub card: CardId,
    pub title: Option<String>,
}

impl Widget for Card<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let source = self
            .card
            .cloning(self.db)
            .map(CardId::from)
            .unwrap_or(self.card);

        let oracle_text = source.oracle_text(self.db);
        let has_oracle_text = !oracle_text.is_empty();
        let etb_text = source
            .etb_abilities(self.db)
            .into_iter()
            .map(|ability| ability.text(self.db))
            .filter(|text| !text.is_empty())
            .collect_vec();

        let has_etb_text = !etb_text.is_empty();
        let effects_text = source
            .effects(self.db)
            .into_iter()
            .map(|effect| effect.oracle_text)
            .filter(|text| !text.is_empty())
            .collect_vec();
        let has_effects_text = !effects_text.is_empty();
        let triggers = source.triggers_text(self.db);
        let has_triggers = !triggers.is_empty();
        let abilities = source.abilities_text(self.db);
        let has_abilities = !abilities.is_empty();
        let keywords = source
            .keywords(self.db)
            .keys()
            .map(|k| k.as_ref())
            .join(", ");
        let has_keywords = !keywords.is_empty();
        let modified_by = source.modified_by_text(self.db);
        let is_modified = !modified_by.is_empty();
        let counters = CounterId::counter_text_on(self.db, source);
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
            .chain(std::iter::once(abilities))
            .chain(std::iter::once(String::default()).filter(|_| has_abilities))
            .chain(std::iter::once("Modified by:".to_string()).filter(|_| is_modified))
            .chain(modified_by)
            .chain(std::iter::once(String::default()).filter(|_| is_modified))
            .chain(std::iter::once("Counters:".to_string()).filter(|_| has_counters))
            .chain(counters.into_iter().map(|counter| format!("  {}", counter)))
            .join("\n");

        Frame::none()
            .stroke(Stroke::new(2.0, Color32::DARK_GRAY))
            .inner_margin(5.0)
            .outer_margin(2.0)
            .show(ui, |ui| {
                ui.expand_to_include_rect(ui.max_rect());
                ui.vertical(|ui| {
                    let mut sense = ui.allocate_response(egui::vec2(0.0, 0.0), Sense::click());

                    if let Some(title) = self.title.as_ref() {
                        sense =
                            sense.union(ui.add(
                                Label::new(RichText::new(title).heading()).sense(Sense::click()),
                            ));
                        ui.separator();
                    }

                    ScrollArea::vertical().id_source(self.title).show(ui, |ui| {
                        sense = sense.union(ui.add(Label::new(paragraph).sense(Sense::click())));
                    });
                    sense
                })
                .inner
            })
            .inner
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

pub struct Stack<'clicked> {
    pub items: Vec<String>,
    pub left_clicked: &'clicked mut Option<usize>,
}

impl Widget for Stack<'_> {
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
                                for (idx, item) in self.items.into_iter().enumerate() {
                                    if ui.add(Label::new(item).sense(Sense::click())).clicked() {
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
    pub db: &'db mut Database,
    pub owner: Owner,
    pub cards: Vec<CardId>,
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
                        for (index, card) in self.cards.into_iter().enumerate() {
                            let sense =
                                ui.add(Label::new(card.name(self.db)).sense(Sense::click()));
                            ui.separator();
                            if sense.clicked_by(PointerButton::Primary) {
                                *self.left_clicked = Some(index);
                            } else if sense.clicked_by(PointerButton::Secondary) {
                                *self.right_clicked = Some(index);
                            }
                        }
                    });
                });
            })
            .response
    }
}

pub struct Battlefield<'db, 'clicked> {
    pub db: &'db mut Database,
    pub player: Owner,
    pub cards: Vec<(usize, CardId)>,
    pub left_clicked: &'clicked mut Option<usize>,
    pub right_clicked: &'clicked mut Option<usize>,
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
                            let mut types = card.types(self.db).into_iter().collect_vec();
                            types.sort();
                            let mut subtypes = card.subtypes(self.db).into_iter().collect_vec();
                            subtypes.sort();
                            (types, subtypes)
                        });

                        let card_titles = self
                            .cards
                            .iter()
                            .map(|(_, card)| {
                                let manifested = card.manifested(self.db);

                                let index = card.id(self.db);
                                let name = if manifested {
                                    "Manifested".to_string()
                                } else {
                                    card.name(self.db)
                                };

                                let cost = card.cost(self.db);
                                if cost.mana_cost.is_empty() || manifested {
                                    format!("({}) {}", index, name)
                                } else {
                                    format!("({}) {} - {}", index, name, cost.text())
                                }
                            })
                            .collect_vec();

                        const MIN_WIDTH: f32 = 200.0;
                        const MIN_HEIGHT: f32 = 300.0;

                        for ((idx, card), title) in self.cards.into_iter().zip(card_titles) {
                            let sense = ui.add_sized(
                                egui::vec2(MIN_WIDTH, MIN_HEIGHT),
                                Card {
                                    db: self.db,
                                    card,
                                    title: Some(title),
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

pub struct Actions<'db, 'ap, 't, 'clicked> {
    pub db: &'db mut Database,
    pub all_players: &'ap AllPlayers,
    pub turn: &'t Turn,
    pub player: Owner,
    pub card: Option<CardId>,
    pub left_clicked: &'clicked mut Option<usize>,
}

impl Widget for Actions<'_, '_, '_, '_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let abilities = if let Some(card) = self.card {
            if card.is_in_location::<InHand>(self.db) && self.turn.can_cast(self.db, card) {
                [(0, format!("Play {}", card.name(self.db)))]
                    .into_iter()
                    .chain(
                        card.activated_abilities(self.db)
                            .into_iter()
                            .enumerate()
                            .filter_map(|(idx, ability)| {
                                if ability.can_be_activated(
                                    self.db,
                                    self.all_players,
                                    self.turn,
                                    self.player,
                                ) {
                                    Some((idx + 1, ability.text(self.db)))
                                } else {
                                    None
                                }
                            }),
                    )
                    .collect_vec()
            } else {
                card.activated_abilities(self.db)
                    .into_iter()
                    .enumerate()
                    .filter_map(|(idx, ability)| {
                        if ability.can_be_activated(
                            self.db,
                            self.all_players,
                            self.turn,
                            self.player,
                        ) {
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
