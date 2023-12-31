use indexmap::IndexMap;
use itertools::Itertools;

use crate::{
    battlefield::{ActionResult, PendingResult, PendingResults},
    effects::Destination,
    in_play::{CardId, Database},
    player::AllPlayers,
};

#[derive(Debug)]
pub(crate) struct ExamineTopCards {
    cards: Vec<CardId>,
    cards_to_location: IndexMap<Destination, Vec<CardId>>,
    destinations: IndexMap<Destination, usize>,
    placing: usize,
}

impl ExamineTopCards {
    pub(crate) fn new(cards: Vec<CardId>, destinations: IndexMap<Destination, usize>) -> Self {
        Self {
            cards,
            cards_to_location: Default::default(),
            destinations,
            placing: 0,
        }
    }

    fn choose(&mut self, choice: Option<usize>) -> bool {
        debug!(
            "Choosing to place to {:?}",
            self.destinations
                .get_index(self.placing)
                .map(|t| t.0.as_ref())
        );

        if choice.is_none() && self.placing < self.destinations.len() - 1 {
            self.placing += 1;
            return false;
        } else if choice.is_none() {
            let (destination, _) = self.destinations.get_index(self.placing).unwrap();
            for card in self.cards.drain(..) {
                self.cards_to_location
                    .entry(*destination)
                    .or_default()
                    .push(card);
            }
            return true;
        }

        let (destination, max) = self.destinations.get_index(self.placing).unwrap();
        let card = self.cards.remove(choice.unwrap());
        self.cards_to_location
            .entry(*destination)
            .or_default()
            .push(card);

        if self.cards_to_location[destination].len() == *max {
            self.placing += 1;
        }

        self.cards.is_empty()
    }
}

impl PendingResult for ExamineTopCards {
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

    #[instrument(skip(_db))]
    fn description(&self, _db: &Database) -> String {
        let (destination, _) = self.destinations.get_index(self.placing).unwrap();
        match destination {
            Destination::Hand => "moving to your hand".to_string(),
            Destination::TopOfLibrary => "placing on top of your library".to_string(),
            Destination::BottomOfLibrary => "placing on the bottom of your library".to_string(),
            Destination::Graveyard => "placing in your graveyard".to_string(),
            Destination::Battlefield { .. } => "placing on the battlefield".to_string(),
        }
    }

    fn is_empty(&self) -> bool {
        self.cards.is_empty() && self.cards_to_location.is_empty()
    }

    fn make_choice(
        &mut self,
        _db: &mut Database,
        _all_players: &mut AllPlayers,
        choice: Option<usize>,
        results: &mut PendingResults,
    ) -> bool {
        if self.choose(choice) {
            for (destination, cards) in self.cards_to_location.drain(..) {
                match destination {
                    Destination::Hand => {
                        for card in cards {
                            results.push_settled(ActionResult::MoveToHandFromLibrary(card));
                        }
                    }
                    Destination::TopOfLibrary => {
                        for card in cards {
                            results.push_settled(ActionResult::MoveFromLibraryToTopOfLibrary(card));
                        }
                    }
                    Destination::BottomOfLibrary => {
                        for card in cards {
                            results
                                .push_settled(ActionResult::MoveFromLibraryToBottomOfLibrary(card));
                        }
                    }
                    Destination::Graveyard => {
                        for card in cards {
                            results.push_settled(ActionResult::MoveFromLibraryToGraveyard(card));
                        }
                    }
                    Destination::Battlefield { enters_tapped } => {
                        for card in cards {
                            results.push_settled(ActionResult::AddToBattlefieldFromLibrary {
                                card,
                                enters_tapped,
                            });
                        }
                    }
                }
            }

            true
        } else {
            false
        }
    }
}
