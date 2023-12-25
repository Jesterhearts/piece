use itertools::Itertools;

use crate::{
    battlefield::{PendingResult, PendingResults},
    in_play::{CardId, Database},
    player::{AllPlayers, Controller},
};

#[derive(Debug)]
pub struct ChoosingScry {
    cards: Vec<CardId>,
    cards_on_bottom: Vec<CardId>,
    cards_on_top: Vec<CardId>,
    placing_on_top: bool,
    controller: Controller,
}

impl ChoosingScry {
    pub fn new(cards: Vec<CardId>, controller: Controller) -> Self {
        Self {
            cards,
            cards_on_bottom: Default::default(),
            cards_on_top: Default::default(),
            placing_on_top: Default::default(),
            controller,
        }
    }

    fn choose(&mut self, choice: Option<usize>) -> bool {
        debug!("Choosing to scry to top = {}", self.placing_on_top);
        if choice.is_none() && !self.placing_on_top {
            self.placing_on_top = true;
            return false;
        } else if choice.is_none() {
            for card in self.cards.drain(..) {
                self.cards_on_top.push(card);
            }
            return true;
        }

        if self.placing_on_top {
            let card = self.cards.remove(choice.unwrap());
            self.cards_on_top.push(card);
        } else {
            let card = self.cards.remove(choice.unwrap());
            self.cards_on_bottom.push(card);
        }

        self.cards.is_empty()
    }
}

impl PendingResult for ChoosingScry {
    fn optional(&self, _db: &Database, _all_players: &AllPlayers) -> bool {
        true
    }

    fn options(&self, db: &mut Database, _all_players: &AllPlayers) -> Vec<(usize, String)> {
        self.cards
            .iter()
            .map(|card| card.name(db))
            .enumerate()
            .collect_vec()
    }

    fn description(&self, _db: &Database) -> String {
        if self.placing_on_top {
            "placing on top of your library".to_string()
        } else {
            "placing on the bottom of your library".to_string()
        }
    }

    fn is_empty(&self) -> bool {
        self.cards.is_empty() && self.cards_on_bottom.is_empty() && self.cards_on_bottom.is_empty()
    }

    fn make_choice(
        &mut self,
        db: &mut Database,
        all_players: &mut AllPlayers,
        choice: Option<usize>,
        _results: &mut PendingResults,
    ) -> bool {
        if self.choose(choice) {
            for card in self.cards_on_bottom.drain(..) {
                all_players[self.controller].deck.place_on_bottom(db, card);
            }

            for card in self.cards_on_top.drain(..) {
                all_players[self.controller].deck.place_on_top(db, card);
            }
            true
        } else {
            false
        }
    }
}
