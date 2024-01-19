use indexmap::IndexMap;
use itertools::Itertools;

use crate::{
    action_result::ActionResult,
    in_play::{CardId, Database},
    pending_results::{Options, PendingResult, PendingResults},
    protogen::{
        effects::{
            destination::{self, Battlefield},
            examine_top_cards::Dest,
        },
        targets::Location,
    },
};

#[derive(Debug)]
pub(crate) struct ExamineCards {
    location: Location,
    cards: Vec<CardId>,
    cards_to_location: IndexMap<destination::Destination, Vec<CardId>>,
    destinations: Vec<Dest>,
    placing: usize,
}

impl ExamineCards {
    pub(crate) fn new(location: Location, cards: Vec<CardId>, destinations: Vec<Dest>) -> Self {
        Self {
            location,
            cards,
            cards_to_location: Default::default(),
            destinations,
            placing: 0,
        }
    }

    fn choose(&mut self, choice: Option<usize>) -> bool {
        debug!(
            "Choosing to place to {:?}",
            self.destinations.get(self.placing).map(|t| t
                .destination
                .destination
                .as_ref()
                .unwrap())
        );

        if choice.is_none() && self.placing < self.destinations.len() - 1 {
            self.placing += 1;
            return false;
        } else if choice.is_none() {
            let Dest {
                destination, count, ..
            } = &self.destinations[self.placing];

            for card in self.cards.drain(..).take(*count as usize) {
                self.cards_to_location
                    .entry(destination.destination.as_ref().unwrap().clone())
                    .or_default()
                    .push(card);
            }
            return true;
        }

        let Dest {
            destination, count, ..
        } = &self.destinations[self.placing];

        let card = self.cards.remove(choice.unwrap());
        self.cards_to_location
            .entry(destination.destination.as_ref().unwrap().clone())
            .or_default()
            .push(card);

        if self
            .cards_to_location
            .get(destination.destination.as_ref().unwrap())
            .unwrap()
            .len()
            == *count as usize
        {
            self.placing += 1;
        }

        self.cards.is_empty() || self.placing == self.destinations.len()
    }
}

impl PendingResult for ExamineCards {
    fn cancelable(&self, _db: &Database) -> bool {
        (self.placing < self.destinations.len() - 1)
            || (self.destinations[self.placing].count as usize >= self.cards.len())
    }

    fn options(&self, db: &mut Database) -> Options {
        let options = self
            .cards
            .iter()
            .map(|card| card.name(db).clone())
            .enumerate()
            .collect_vec();

        if (self.placing < self.destinations.len() - 1)
            || (self.destinations[self.placing].count as usize >= self.cards.len())
        {
            Options::OptionalList(options)
        } else {
            Options::MandatoryList(options)
        }
    }

    fn target_for_option(
        &self,
        db: &Database,
        option: usize,
    ) -> Option<crate::stack::ActiveTarget> {
        self.cards
            .get(option)
            .and_then(|card| card.target_from_location(db))
    }

    #[instrument(skip(_db))]
    fn description(&self, _db: &Database) -> String {
        let Dest { destination, .. } = &self.destinations[self.placing];
        match destination.destination.as_ref().unwrap() {
            destination::Destination::Hand(_) => "moving to your hand".to_string(),
            destination::Destination::TopOfLibrary(_) => {
                "placing on top of your library".to_string()
            }
            destination::Destination::BottomOfLibrary(_) => {
                "placing on the bottom of your library".to_string()
            }
            destination::Destination::Graveyard(_) => "placing in your graveyard".to_string(),
            destination::Destination::Battlefield(_) => "placing on the battlefield".to_string(),
        }
    }

    fn is_empty(&self) -> bool {
        self.cards.is_empty() && self.cards_to_location.is_empty()
    }

    fn make_choice(
        &mut self,
        _db: &mut Database,
        choice: Option<usize>,
        results: &mut PendingResults,
    ) -> bool {
        if self.choose(choice) {
            for (destination, cards) in self.cards_to_location.drain(..) {
                match destination {
                    destination::Destination::Hand(_) => {
                        for card in cards {
                            match self.location {
                                Location::IN_LIBRARY => {
                                    results.push_settled(ActionResult::MoveToHandFromLibrary(card));
                                }
                                _ => unreachable!(),
                            }
                        }
                    }
                    destination::Destination::TopOfLibrary(_) => {
                        for card in cards {
                            match self.location {
                                Location::IN_LIBRARY => {
                                    results.push_settled(
                                        ActionResult::MoveFromLibraryToTopOfLibrary(card),
                                    );
                                }
                                _ => todo!(),
                            }
                        }
                    }
                    destination::Destination::BottomOfLibrary(_) => {
                        for card in cards {
                            match self.location {
                                Location::IN_LIBRARY => {
                                    results.push_settled(
                                        ActionResult::MoveFromLibraryToBottomOfLibrary(card),
                                    );
                                }
                                _ => todo!(),
                            }
                        }
                    }
                    destination::Destination::Graveyard(_) => {
                        for card in cards {
                            match self.location {
                                Location::IN_HAND => {
                                    results.push_settled(ActionResult::Discard(card));
                                }
                                Location::IN_LIBRARY => {
                                    results.push_settled(ActionResult::MoveFromLibraryToGraveyard(
                                        card,
                                    ));
                                }
                                Location::ON_BATTLEFIELD => {
                                    results.push_settled(ActionResult::PermanentToGraveyard(card));
                                }
                                _ => todo!(),
                            }
                        }
                    }
                    destination::Destination::Battlefield(Battlefield {
                        enters_tapped, ..
                    }) => {
                        for card in cards {
                            match self.location {
                                Location::IN_LIBRARY => {
                                    results.push_settled(
                                        ActionResult::AddToBattlefieldFromLibrary {
                                            card,
                                            enters_tapped,
                                        },
                                    );
                                }
                                _ => todo!(),
                            }
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
