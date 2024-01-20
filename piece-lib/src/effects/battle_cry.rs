use std::collections::HashMap;

use protobuf::Enum;

use crate::{
    action_result::ActionResult,
    effects::EffectBehaviors,
    in_play::ModifierId,
    protogen::{
        effects::{BattleCry, BattlefieldModifier, Duration, ModifyBattlefield},
        empty::Empty,
        targets::{restriction, Restriction},
        types::Type,
    },
};

impl EffectBehaviors for BattleCry {
    fn needs_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: &crate::protogen::ids::CardId,
    ) -> usize {
        0
    }

    fn wants_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: &crate::protogen::ids::CardId,
    ) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
        source: &crate::protogen::ids::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        let modifier = ModifierId::upload_temporary_modifier(
            db,
            source.clone(),
            BattlefieldModifier {
                modifier: protobuf::MessageField::some(ModifyBattlefield {
                    add_power: Some(1),
                    entire_battlefield: true,
                    ..Default::default()
                }),
                duration: Duration::UNTIL_END_OF_TURN.into(),
                restrictions: vec![
                    Restriction {
                        restriction: Some(restriction::Restriction::from(
                            restriction::Controller {
                                controller: Some(restriction::controller::Controller::Self_(
                                    Default::default(),
                                )),
                                ..Default::default()
                            },
                        )),
                        ..Default::default()
                    },
                    Restriction {
                        restriction: Some(restriction::Restriction::from(
                            restriction::Attacking::default(),
                        )),
                        ..Default::default()
                    },
                    Restriction {
                        restriction: Some(restriction::Restriction::from(
                            restriction::NotSelf::default(),
                        )),
                        ..Default::default()
                    },
                    Restriction {
                        restriction: Some(restriction::Restriction::from(restriction::OfType {
                            types: HashMap::from([(Type::CREATURE.value(), Empty::default())]),
                            ..Default::default()
                        })),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        );

        results.push_settled(ActionResult::AddModifier { modifier });
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut crate::in_play::Database,
        _targets: Vec<crate::stack::ActiveTarget>,
        source: &crate::protogen::ids::CardId,
        controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        self.push_pending_behavior(db, source, controller, results);
    }
}
