use itertools::Itertools;

use crate::{
    action_result::Action,
    effects::EffectBehaviors,
    in_play::{CardId, Database},
    library::Library,
    log::LogId,
    pending_results::PendingResults,
    protogen::effects,
};

#[derive(Debug, Clone)]
pub(crate) struct RevealEachTopOfLibrary {
    pub(crate) source: CardId,
    pub(crate) reveal: effects::RevealEachTopOfLibrary,
}

impl Action for RevealEachTopOfLibrary {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { source, reveal } = self;

        let players = db.all_players.all_players();
        let revealed = players
            .into_iter()
            .filter_map(|player| {
                Library::reveal_top(db, player).filter(|card| {
                    card.passes_restrictions(
                        db,
                        LogId::current(db),
                        *source,
                        &reveal.for_each.restrictions,
                    )
                })
            })
            .collect_vec();

        let mut results = PendingResults::default();
        if revealed.is_empty() {
            let controller = db[*source].controller;
            for effect in reveal.for_each.if_none.effects.iter() {
                effect.effect.as_ref().unwrap().push_pending_behavior(
                    db,
                    *source,
                    controller,
                    &mut results,
                );
            }
        } else {
            for target in revealed {
                for effect in reveal.for_each.effects.iter() {
                    effect
                        .effect
                        .as_ref()
                        .unwrap()
                        .push_behavior_from_top_of_library(db, *source, target, &mut results);
                }
            }
        }

        results
    }
}
