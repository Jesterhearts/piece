use crate::{
    action_result::Action,
    effects::EffectBehaviors,
    in_play::{CardId, CastFrom, Database},
    log::{Log, LogId},
    pending_results::PendingResults,
    protogen::{
        abilities::TriggeredAbility,
        effects::{effect, Cascade, Effect, Rebound},
        targets::{restriction, Restriction},
        triggers::{self, Trigger, TriggerSource},
    },
    stack::{ActiveTarget, Stack},
};

#[derive(Debug, Clone)]
pub(crate) struct CastCard {
    pub(crate) card: CardId,
    pub(crate) targets: Vec<Vec<ActiveTarget>>,
    pub(crate) from: CastFrom,
    pub(crate) x_is: Option<usize>,
    pub(crate) chosen_modes: Vec<usize>,
}

impl Action for CastCard {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            card,
            targets,
            from,
            x_is,
            chosen_modes,
        } = self;
        let mut results = PendingResults::default();

        Log::cast(db, *card);

        results.extend(card.move_to_stack(db, targets.clone(), Some(*from), chosen_modes.clone()));
        if let Some(x_is) = x_is {
            db[*card].x_is = *x_is;
        };
        card.apply_modifiers_layered(db);

        for (listener, trigger) in db.active_triggers_of_source(TriggerSource::CAST) {
            if card.passes_restrictions(
                db,
                LogId::current(db),
                listener,
                &trigger.trigger.restrictions,
            ) {
                results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
            }
        }

        let cascade = card.cascade(db);
        for _ in 0..cascade {
            results.extend(Stack::move_trigger_to_stack(
                db,
                *card,
                TriggeredAbility {
                    trigger: protobuf::MessageField::some(Trigger {
                        source: TriggerSource::CAST.into(),
                        from: triggers::Location::HAND.into(),
                        restrictions: vec![Restriction {
                            restriction: Some(restriction::Restriction::from(
                                restriction::Controller {
                                    controller: Some(restriction::controller::Controller::Self_(
                                        Default::default(),
                                    )),
                                    ..Default::default()
                                },
                            )),
                            ..Default::default()
                        }],
                        ..Default::default()
                    }),
                    effects: vec![Effect {
                        effect: Some(effect::Effect::from(Cascade::default())),
                        ..Default::default()
                    }],
                    oracle_text: "Cascade".to_string(),
                    ..Default::default()
                },
            ));
        }

        if card.rebound(db) {
            Rebound::default().push_pending_behavior(db, *card, db[*card].controller, &mut results);
        }

        results
    }
}
