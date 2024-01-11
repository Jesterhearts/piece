use crate::{
    action_result::ActionResult,
    effects::{Effect, EffectBehaviors},
    pending_results::{choose_targets::ChooseTargets, TargetSource},
    protogen::{self, types::Type},
    stack::ActiveTarget,
    targets::Restriction,
    types::TypeSet,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DealDamage {
    pub(crate) quantity: usize,
    pub(crate) restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::DealDamage> for DealDamage {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::DealDamage) -> Result<Self, Self::Error> {
        Ok(Self {
            quantity: usize::try_from(value.quantity)?,
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for DealDamage {
    fn needs_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        1
    }

    fn wants_targets(
        &self,
        _db: &crate::in_play::Database,
        _source: crate::in_play::CardId,
    ) -> usize {
        1
    }

    fn valid_targets(
        &self,
        db: &crate::in_play::Database,
        source: crate::in_play::CardId,
        log_session: crate::log::LogId,
        controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for card in db.battlefield.battlefields.values().flat_map(|b| b.iter()) {
            if card.passes_restrictions(
                db,
                log_session,
                source,
                &source.faceup_face(db).restrictions,
            ) && card.types_intersect(db, &TypeSet::from([Type::CREATURE]))
                && card.can_be_targeted(db, controller)
                && card.passes_restrictions(db, log_session, source, &self.restrictions)
            {
                let target = ActiveTarget::Battlefield { id: *card };
                if !already_chosen.contains(&target) {
                    targets.push(target);
                }
            }
        }

        for player in db.all_players.all_players() {
            // TODO player hexproof, non-all-target-damage
            if player.passes_restrictions(
                db,
                log_session,
                controller,
                &source.faceup_face(db).restrictions,
            ) {
                targets.push(ActiveTarget::Player { id: player });
            }
        }

        targets
    }

    fn push_pending_behavior(
        &self,
        db: &mut crate::in_play::Database,
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

        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect::from(self.clone())),
            valid_targets,
            crate::log::LogId::current(db),
            source,
        ));
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut crate::in_play::Database,
        targets: Vec<crate::stack::ActiveTarget>,
        _apply_to_self: bool,
        _source: crate::in_play::CardId,
        _controller: crate::player::Controller,
        results: &mut crate::pending_results::PendingResults,
    ) {
        for target in targets {
            results.push_settled(ActionResult::DamageTarget {
                quantity: self.quantity,
                target,
            });
        }
    }
}
