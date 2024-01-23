use itertools::Itertools;

use crate::{
    action_result::{self, ActionResult},
    effects::EffectBehaviors,
    protogen::{
        cost::XIs,
        effects::{discover::Count, Discover},
    },
};

impl EffectBehaviors for Discover {
    fn needs_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn wants_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::from(action_result::discover::Discover {
            source,
            count: match self.count.as_ref().unwrap() {
                Count::X(x_is) => match x_is.x_is.enum_value().unwrap() {
                    XIs::MANA_VALUE => db[source].modified_cost.cmc() as u32,
                    XIs::MANA_VALUE_OF_TARGET => unreachable!(),
                },
                Count::Fixed(fixed) => fixed.count,
            },
            player: controller,
        }))
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        results.push_settled(ActionResult::from(action_result::discover::Discover {
            source,
            count: match self.count.as_ref().unwrap() {
                Count::X(x_is) => match x_is.x_is.enum_value().unwrap() {
                    XIs::MANA_VALUE => db[source].modified_cost.cmc() as u32,
                    XIs::MANA_VALUE_OF_TARGET => {
                        let card = &db[targets.into_iter().exactly_one().unwrap().id(db).unwrap()];
                        (card.modified_cost.cmc() + card.x_is) as u32
                    }
                },
                Count::Fixed(fixed) => fixed.count,
            },
            player: controller,
        }))
    }
}
