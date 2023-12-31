use std::collections::HashSet;

use itertools::Itertools;

use crate::{
    effects::{Effect, EffectBehaviors},
    in_play::{target_from_location, Database},
    log::Log,
    pending_results::choose_for_each_player::ChooseForEachPlayer,
    protogen,
    targets::Restriction,
};

#[derive(Debug, Clone)]
pub(crate) struct ForEachPlayerChooseThen {
    restrictions: Vec<Restriction>,
    effects: Vec<Effect>,
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
                .map(Effect::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for ForEachPlayerChooseThen {
    fn needs_targets(
        &self,
        db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        db.all_players.all_players().len()
    }

    fn wants_targets(
        &self,
        db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        db.all_players.all_players().len()
    }

    fn valid_targets(
        &self,
        db: &Database,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let already_chosen = already_chosen
            .iter()
            .map(|target| db[target.id().unwrap()].controller)
            .collect::<HashSet<_>>();

        db.cards
            .keys()
            .filter_map(|card| {
                if card.passes_restrictions(db, source, &source.faceup_face(db).restrictions)
                    && card.passes_restrictions(db, source, &self.restrictions)
                    && !already_chosen.contains(&db[*card].controller)
                {
                    Some(target_from_location(db, *card))
                } else {
                    None
                }
            })
            .collect_vec()
    }

    fn push_pending_behavior(
        &self,
        db: &mut Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        let valid_targets =
            self.valid_targets(db, source, controller, results.all_currently_targeted());
        results.push_choose_for_each(ChooseForEachPlayer::new(
            Effect::from(self.clone()),
            valid_targets,
            source,
        ));
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        for target in targets {
            Log::card_chosen(db, target.id().unwrap());
        }

        for effect in self.effects.iter() {
            effect.push_pending_behavior(db, source, controller, results);
        }
    }
}
