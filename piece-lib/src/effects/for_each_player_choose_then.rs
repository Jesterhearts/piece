use std::collections::HashSet;

use itertools::Itertools;

use crate::{
    effects::EffectBehaviors,
    in_play::Database,
    log::Log,
    pending_results::choose_for_each_player::ChooseForEachPlayer,
    protogen::effects::{effect::Effect, ForEachPlayerChooseThen},
};

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
        log_session: crate::log::LogId,
        _controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let already_chosen = already_chosen
            .iter()
            .map(|target| db[target.id(db).unwrap()].controller)
            .collect::<HashSet<_>>();

        db.cards
            .keys()
            .filter_map(|card| {
                if card.passes_restrictions(
                    db,
                    log_session,
                    source,
                    &source.faceup_face(db).restrictions,
                ) && card.passes_restrictions(db, log_session, source, &self.restrictions)
                    && !already_chosen.contains(&db[*card].controller)
                {
                    card.target_from_location(db)
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
        let valid_targets = self.valid_targets(
            db,
            source,
            crate::log::LogId::current(db),
            controller,
            results.all_currently_targeted(),
        );
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
            Log::card_chosen(db, target.id(db).unwrap());
        }

        for effect in self.effects.iter() {
            effect
                .effect
                .as_ref()
                .unwrap()
                .push_pending_behavior(db, source, controller, results);
        }
    }
}
