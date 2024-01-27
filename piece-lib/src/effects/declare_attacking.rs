use std::collections::HashMap;

use itertools::Itertools;
use protobuf::Enum;

use crate::{
    effects::{EffectBehaviors, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    log::LogId,
    protogen::{
        effects::{
            ApplyModifier, BattlefieldModifier, DeclareAttacking, Duration, Effect,
            ModifyBattlefield, SelectAll, TriggeredAbility,
        },
        empty::Empty,
        targets::{restriction, Restriction},
        triggers::TriggerSource,
        types::Type,
    },
    stack::Stack,
};

impl EffectBehaviors for DeclareAttacking {
    fn apply(
        &mut self,
        db: &mut Database,
        pending: &mut PendingEffects,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        let attackers = selected.restore();

        for (attacker, target) in attackers
            .into_iter()
            .map(|attacker| attacker.id(db).unwrap())
            .zip(selected.iter().map(|target| target.player().unwrap()))
            .collect_vec()
        {
            for (listener, trigger) in db.active_triggers_of_source(TriggerSource::ATTACKS) {
                if attacker.passes_restrictions(
                    db,
                    LogId::current(db),
                    listener,
                    &trigger.trigger.restrictions,
                ) {
                    pending.extend(Stack::move_trigger_to_stack(db, listener, trigger));
                }
            }

            for _ in 0..attacker.battle_cry(db) {
                let restrictions = vec![
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
                ];

                pending.extend(Stack::move_trigger_to_stack(
                    db,
                    attacker,
                    TriggeredAbility {
                        trigger: protobuf::MessageField::none(),
                        effects: vec![
                            Effect {
                                effect: Some(
                                    SelectAll {
                                        restrictions,
                                        ..Default::default()
                                    }
                                    .into(),
                                ),
                                ..Default::default()
                            },
                            Effect {
                                effect: Some(
                                    ApplyModifier {
                                        modifier: protobuf::MessageField::some(
                                            BattlefieldModifier {
                                                modifier: protobuf::MessageField::some(
                                                    ModifyBattlefield {
                                                        add_power: Some(1),
                                                        ..Default::default()
                                                    },
                                                ),
                                                duration: protobuf::EnumOrUnknown::new(
                                                    Duration::UNTIL_END_OF_TURN,
                                                ),
                                                ..Default::default()
                                            },
                                        ),
                                        ..Default::default()
                                    }
                                    .into(),
                                ),
                                ..Default::default()
                            },
                        ],
                        oracle_text: "Battle cry".to_string(),
                        ..Default::default()
                    },
                ))
            }

            db[attacker].attacking = Some(target)
        }
    }
}
