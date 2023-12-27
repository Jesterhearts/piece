use indexmap::IndexSet;

use crate::{
    battlefield::ActionResult,
    controller::ControllerRestriction,
    effects::{BattlefieldModifier, EffectBehaviors, EffectDuration, ModifyBattlefield},
    in_play::ModifierId,
    targets::Restriction,
    types::Type,
};

#[derive(Debug)]
pub struct BattleCry;

impl EffectBehaviors for BattleCry {
    fn needs_targets(&'static self) -> usize {
        0
    }

    fn wants_targets(&'static self) -> usize {
        0
    }

    fn push_pending_behavior(
        &'static self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::AddModifier {
            modifier: ModifierId::upload_temporary_modifier(
                db,
                source,
                &BattlefieldModifier {
                    modifier: ModifyBattlefield {
                        add_power: Some(1),
                        ..Default::default()
                    },
                    controller: ControllerRestriction::You,
                    duration: EffectDuration::UntilEndOfTurn,
                    restrictions: vec![
                        Restriction::Attacking,
                        Restriction::OfType {
                            types: IndexSet::from([Type::Creature]),
                            subtypes: Default::default(),
                        },
                    ],
                },
            ),
        })
    }

    fn push_behavior_with_targets(
        &'static self,
        db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        self.push_pending_behavior(db, source, controller, results);
    }
}
