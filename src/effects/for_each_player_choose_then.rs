use std::collections::HashSet;

use itertools::Itertools;

use crate::{
    battlefield::choose_for_each_player::ChooseForEachPlayer,
    effects::{AnyEffect, Effect, EffectBehaviors},
    in_play::{all_cards, target_from_location, Database},
    player::AllPlayers,
    protogen,
    targets::Restriction,
};

#[derive(Debug)]
pub(crate) struct ForEachPlayerChooseThen {
    restrictions: Vec<Restriction>,
    effects: Vec<AnyEffect>,
}

impl TryFrom<&protogen::effects::ForEachPlayerChooseThen> for ForEachPlayerChooseThen {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::ForEachPlayerChooseThen) -> Result<Self, Self::Error> {
        Ok(Self {
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
            effects: value
                .effects
                .iter()
                .map(AnyEffect::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for ForEachPlayerChooseThen {
    fn needs_targets(&'static self, db: &mut Database) -> usize {
        AllPlayers::all_players_in_db(db).len()
    }

    fn wants_targets(&'static self, db: &mut Database) -> usize {
        AllPlayers::all_players_in_db(db).len()
    }

    fn valid_targets(
        &'static self,
        db: &mut Database,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let already_chosen = already_chosen
            .iter()
            .map(|target| target.id().unwrap().controller(db))
            .collect::<HashSet<_>>();

        all_cards(db)
            .into_iter()
            .filter_map(|card| {
                if card.passes_restrictions(db, source, &source.restrictions(db))
                    && card.passes_restrictions(db, source, &self.restrictions)
                    && !already_chosen.contains(&card.controller(db))
                {
                    Some(target_from_location(db, card))
                } else {
                    None
                }
            })
            .collect_vec()
    }

    fn push_pending_behavior(
        &'static self,
        db: &mut Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        let valid_targets =
            self.valid_targets(db, source, controller, results.all_currently_targeted());
        results.push_choose_for_each(ChooseForEachPlayer::new(
            Effect(self),
            valid_targets,
            source,
        ));
    }

    fn push_behavior_with_targets(
        &'static self,
        db: &mut Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        for target in targets {
            target.id().unwrap().choose(db);
        }

        for effect in self.effects.iter() {
            effect
                .effect(db, controller)
                .push_pending_behavior(db, source, controller, results);
        }
    }
}
