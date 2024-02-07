use itertools::Itertools;

use crate::{
    effects::{EffectBehaviors, EffectBundle, Options, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
    log::{Log, LogId},
    protogen::effects::SelectTargets,
    stack::{Selected, TargetType},
};

impl EffectBehaviors for SelectTargets {
    fn wants_input(
        &self,
        _db: &Database,
        _source: Option<CardId>,
        _already_selected: &[Selected],
        _modes: &[usize],
    ) -> bool {
        true
    }

    fn options(
        &self,
        db: &Database,
        source: Option<CardId>,
        already_selected: &[Selected],
        _modes: &[usize],
    ) -> Options {
        let options = db
            .cards
            .keys()
            .copied()
            .filter(|card| {
                card.passes_restrictions(
                    db,
                    LogId::current(db),
                    source.unwrap(),
                    &self.restrictions,
                ) && !already_selected
                    .iter()
                    .filter_map(|selected| selected.id(db))
                    .any(|selected| selected == *card)
            })
            .map(|card| card.name(db).clone())
            .chain(
                db.all_players
                    .all_players()
                    .into_iter()
                    .filter(|player| {
                        player.passes_restrictions(
                            db,
                            LogId::current(db),
                            db[source.unwrap()].controller,
                            &self.restrictions,
                        )
                    })
                    .map(|player| db.all_players[player].name.clone()),
            )
            .enumerate()
            .collect_vec();

        if self.optional {
            Options::OptionalList(options)
        } else {
            Options::MandatoryList(options)
        }
    }

    fn select(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        option: Option<usize>,
        selected: &mut SelectedStack,
    ) -> SelectionResult {
        let mut targets = db
            .cards
            .keys()
            .copied()
            .filter(|card| {
                {
                    card.passes_restrictions(
                        db,
                        LogId::current(db),
                        source.unwrap(),
                        &self.restrictions,
                    ) && !selected
                        .iter()
                        .filter_map(|target| target.id(db))
                        .any(|target| target == *card)
                }
            })
            .map(|card| Selected {
                location: card.location(db),
                target_type: TargetType::Card(card),
                targeted: true,
                restrictions: self.restrictions.clone(),
            })
            .chain(
                db.all_players
                    .all_players()
                    .into_iter()
                    .filter(|player| {
                        player.passes_restrictions(
                            db,
                            LogId::current(db),
                            db[source.unwrap()].controller,
                            &self.restrictions,
                        )
                    })
                    .map(|player| Selected {
                        location: None,
                        target_type: TargetType::Player(player),
                        targeted: true,
                        restrictions: self.restrictions.clone(),
                    }),
            );

        if let Some(option) = option {
            let target = targets.nth(option).unwrap();

            selected.push(target);

            let count = self.count.count(db, source, selected);
            if selected.len() == (count as usize) {
                SelectionResult::Complete
            } else {
                SelectionResult::PendingChoice
            }
        } else if self.optional || targets.next().is_none() {
            SelectionResult::Complete
        } else {
            SelectionResult::PendingChoice
        }
    }

    fn apply(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        for target in selected.iter() {
            if let Some(card) = target.id(db) {
                Log::card_chosen(db, card);
            }
        }

        vec![]
    }
}
