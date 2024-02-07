use itertools::Itertools;

use crate::{
    effects::{EffectBehaviors, EffectBundle, Options, SelectedStack, SelectionResult},
    in_play::{CardId, Database},
    log::{Log, LogId},
    protogen::effects::SelectForEachPlayer,
    stack::{Selected, TargetType},
};

impl EffectBehaviors for SelectForEachPlayer {
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
        let list = self
            .valid_targets(db, already_selected, source)
            .map(|card| card.name(db).clone())
            .enumerate()
            .collect_vec();

        if self.optional {
            Options::OptionalList(list)
        } else {
            Options::MandatoryList(list)
        }
    }

    fn select(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        option: Option<usize>,
        selected: &mut SelectedStack,
    ) -> SelectionResult {
        if let Some(option) = option {
            let card = self
                .valid_targets(db, selected, source)
                .nth(option)
                .unwrap();

            selected.push(Selected {
                location: card.location(db),
                target_type: TargetType::Card(card),
                targeted: self.targeted,
                restrictions: self.restrictions.clone(),
            });

            if selected.len() == db.all_players.all_players().len() {
                SelectionResult::Complete
            } else {
                SelectionResult::PendingChoice
            }
        } else if self.optional
            || !db.cards.keys().copied().any(|card| {
                !selected
                    .iter()
                    .any(|selected| db[selected.id(db).unwrap()].controller == db[card].controller)
                    && card.passes_restrictions(
                        db,
                        LogId::current(db),
                        source.unwrap(),
                        &self.restrictions,
                    )
                    && (!self.targeted || card.can_be_targeted(db, db[source.unwrap()].controller))
            })
        {
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
            Log::card_chosen(db, target.id(db).unwrap());
        }

        vec![]
    }
}

impl SelectForEachPlayer {
    fn valid_targets<'db>(
        &'db self,
        db: &'db Database,
        already_selected: &'db [Selected],
        source: Option<CardId>,
    ) -> impl Iterator<Item = CardId> + 'db {
        db.cards.keys().copied().filter(move |card| {
            !already_selected
                .iter()
                .any(|selected| db[selected.id(db).unwrap()].controller == db[*card].controller)
                && card.passes_restrictions(
                    db,
                    LogId::current(db),
                    source.unwrap(),
                    &self.restrictions,
                )
                && (!self.targeted || card.can_be_targeted(db, db[source.unwrap()].controller))
        })
    }
}
