use std::collections::HashMap;

use crate::{
    battlefield::ActionResult,
    effects::{BattlefieldModifier, EffectBehaviors, EffectDuration, ModifyBattlefield},
    in_play::ModifierId,
    protogen::{empty::Empty, types::type_::TypeDiscriminants},
    targets::{ControllerRestriction, Restriction},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BattleCry;

impl EffectBehaviors for BattleCry {
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
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        let modifier = ModifierId::upload_temporary_modifier(
            db,
            source,
            BattlefieldModifier {
                modifier: ModifyBattlefield {
                    add_power: Some(1),
                    entire_battlefield: true,
                    ..Default::default()
                },
                duration: EffectDuration::UntilEndOfTurn,
                restrictions: vec![
                    Restriction::Controller(ControllerRestriction::Self_),
                    Restriction::Attacking,
                    Restriction::NotSelf,
                    Restriction::OfType {
                        types: HashMap::from([(
                            TypeDiscriminants::Creature.as_ref().to_string(),
                            Empty::default(),
                        )]),
                        subtypes: Default::default(),
                    },
                ],
            },
        );

        results.push_settled(ActionResult::AddModifier { modifier });
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        self.push_pending_behavior(db, source, controller, results);
    }
}
