use crate::{
    effects::EffectBehaviors,
    in_play::{CardId, Database},
    pending_results::PendingResults,
    player::Controller,
    protogen::{
        abilities::TriggeredAbility,
        effects::{ChooseCast, Effect, Rebound},
        targets::{restriction::Self_, Restriction},
    },
    stack::ActiveTarget,
    turns::Phase,
};

impl EffectBehaviors for Rebound {
    fn needs_targets(&self, _db: &Database, _source: CardId) -> usize {
        0
    }

    fn wants_targets(&self, _db: &Database, _source: CardId) -> usize {
        0
    }

    fn push_pending_behavior(
        &self,
        db: &mut Database,
        source: CardId,
        controller: Controller,
        _results: &mut PendingResults,
    ) {
        db.delayed_triggers
            .entry(controller.into())
            .or_default()
            .entry(Phase::Upkeep)
            .or_default()
            .push((
                source,
                TriggeredAbility {
                    trigger: protobuf::MessageField::none(),
                    effects: vec![Effect {
                        oracle_text: "At the beginning of your next upkeep, \
                        you may cast the spell from exile without paying its mana cost"
                            .to_string(),
                        effect: Some(
                            ChooseCast {
                                restrictions: vec![Restriction {
                                    restriction: Some(Self_::default().into()),
                                    ..Default::default()
                                }],
                                ..Default::default()
                            }
                            .into(),
                        ),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            ));
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut Database,
        _targets: Vec<ActiveTarget>,
        source: CardId,
        controller: Controller,
        results: &mut PendingResults,
    ) {
        self.push_pending_behavior(db, source, controller, results);
    }
}
