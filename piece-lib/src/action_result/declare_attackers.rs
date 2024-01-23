use crate::{
    action_result::Action,
    in_play::{CardId, Database},
    log::LogId,
    pending_results::PendingResults,
    player::Owner,
    protogen::{
        abilities::TriggeredAbility,
        effects::{effect, BattleCry, Effect},
        targets::{restriction, Restriction},
        triggers::{self, Trigger, TriggerSource},
    },
    stack::Stack,
};

#[derive(Debug, Clone)]
pub(crate) struct DeclareAttackers {
    pub(crate) attackers: Vec<CardId>,
    pub(crate) targets: Vec<Owner>,
}

impl Action for DeclareAttackers {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { attackers, targets } = self;
        let mut results = PendingResults::default();
        for (attacker, target) in attackers.iter().zip(targets.iter()) {
            db[*attacker].attacking = Some(*target);

            let listeners = db.active_triggers_of_source(TriggerSource::ATTACKS);
            debug!("attack listeners {:?}", listeners);
            for (listener, trigger) in listeners {
                if attacker.passes_restrictions(
                    db,
                    LogId::current(db),
                    listener,
                    &trigger.trigger.restrictions,
                ) {
                    results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
                }
            }

            for _ in 0..attacker.battle_cry(db) {
                results.extend(Stack::move_trigger_to_stack(
                    db,
                    *attacker,
                    TriggeredAbility {
                        trigger: protobuf::MessageField::some(Trigger {
                            source: TriggerSource::ATTACKS.into(),
                            from: triggers::Location::ANYWHERE.into(),
                            restrictions: vec![Restriction {
                                restriction: Some(restriction::Restriction::from(
                                    restriction::Controller {
                                        controller: Some(
                                            restriction::controller::Controller::Self_(
                                                Default::default(),
                                            ),
                                        ),
                                        ..Default::default()
                                    },
                                )),
                                ..Default::default()
                            }],
                            ..Default::default()
                        }),
                        effects: vec![Effect {
                            effect: Some(effect::Effect::from(BattleCry::default())),
                            ..Default::default()
                        }],
                        oracle_text: "Battle cry".to_string(),
                        ..Default::default()
                    },
                ));
            }

            if !attacker.vigilance(db) {
                results.extend(attacker.tap(db));
            }
        }
        debug!(
            "Set number of attackers to {} in turn {}",
            attackers.len(),
            db.turn.turn_count
        );
        db.turn.number_of_attackers_this_turn = attackers.len();
        // TODO declare blockers
        results
    }
}
