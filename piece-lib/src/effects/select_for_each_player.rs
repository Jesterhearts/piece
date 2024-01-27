use itertools::Itertools;

use crate::{
    effects::{EffectBehaviors, Options, PendingEffects, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
    log::{Log, LogId},
    protogen::effects::SelectForEachPlayer,
    stack::{Selected, TargetType},
};

impl EffectBehaviors for SelectForEachPlayer {
    fn options(
        &self,
        db: &Database,
        source: Option<CardId>,
        already_selected: &[Selected],
        _modes: &[usize],
    ) -> Options {
        Options::MandatoryList(
            db.cards
                .keys()
                .copied()
                .filter(|card| {
                    !already_selected.iter().any(|selected| {
                        db[selected.id(db).unwrap()].controller == db[*card].controller
                    }) && card.passes_restrictions(
                        db,
                        LogId::current(db),
                        source.unwrap(),
                        &self.restrictions,
                    )
                })
                .map(|card| card.name(db).clone())
                .enumerate()
                .collect_vec(),
        )
    }

    fn select(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        option: Option<usize>,
        selected: &mut SelectedStack,
        _modes: &mut Vec<usize>,
    ) -> SelectionResult {
        if let Some(option) = option {
            let card = db
                .cards
                .keys()
                .copied()
                .filter(|card| {
                    !selected.iter().any(|selected| {
                        db[selected.id(db).unwrap()].controller == db[*card].controller
                    }) && card.passes_restrictions(
                        db,
                        LogId::current(db),
                        source.unwrap(),
                        &self.restrictions,
                    )
                })
                .nth(option)
                .unwrap();

            selected.push(Selected {
                location: card.location(db),
                target_type: TargetType::Card(card),
                targeted: false,
                restrictions: self.restrictions.clone(),
            });

            if selected.len() == db.all_players.all_players().len() {
                SelectionResult::Complete
            } else {
                SelectionResult::PendingChoice
            }
        } else if !db.cards.keys().copied().any(|card| {
            !selected
                .iter()
                .any(|selected| db[selected.id(db).unwrap()].controller == db[card].controller)
                && card.passes_restrictions(
                    db,
                    LogId::current(db),
                    source.unwrap(),
                    &self.restrictions,
                )
        }) {
            SelectionResult::Complete
        } else {
            SelectionResult::PendingChoice
        }
    }

    fn apply(
        &mut self,
        db: &mut Database,
        _pending: &mut PendingEffects,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        for target in selected.iter() {
            Log::card_chosen(db, target.id(db).unwrap());
        }
    }
}
