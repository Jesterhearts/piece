use indexmap::IndexSet;

use crate::{
    battlefield::{choose_targets::ChooseTargets, ActionResult, TargetSource},
    controller::ControllerRestriction,
    effects::{Effect, EffectBehaviors},
    in_play::{self, OnBattlefield},
    player::AllPlayers,
    protogen,
    stack::ActiveTarget,
    targets::Restriction,
    types::Type,
};

#[derive(Debug, Clone)]
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
    fn needs_targets(&self) -> usize {
        1
    }

    fn wants_targets(&self) -> usize {
        1
    }

    fn valid_targets(
        &self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        already_chosen: &std::collections::HashSet<crate::stack::ActiveTarget>,
    ) -> Vec<crate::stack::ActiveTarget> {
        let mut targets = vec![];
        for card in in_play::all_cards(db) {
            if card.passes_restrictions(
                db,
                source,
                ControllerRestriction::Any,
                &source.restrictions(db),
            ) && card.is_in_location::<OnBattlefield>(db)
                && card.types_intersect(db, &IndexSet::from([Type::Creature]))
                && card.can_be_targeted(db, controller)
                && card.passes_restrictions(
                    db,
                    source,
                    ControllerRestriction::Any,
                    &self.restrictions,
                )
            {
                let target = ActiveTarget::Battlefield { id: card };
                if !already_chosen.contains(&target) {
                    targets.push(target);
                }
            }
        }

        for player in AllPlayers::all_players_in_db(db) {
            // TODO player hexproof, non-all-target-damage
            if player.passes_restrictions(db, controller, &source.restrictions(db)) {
                targets.push(ActiveTarget::Player { id: player });
            }
        }

        targets
    }

    fn push_pending_behavior(
        &'static self,
        db: &mut crate::in_play::Database,
        source: crate::in_play::CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        let valid_targets =
            self.valid_targets(db, source, controller, results.all_currently_targeted());

        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect(self)),
            valid_targets,
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
        results: &mut crate::battlefield::PendingResults,
    ) {
        for target in targets {
            results.push_settled(ActionResult::DamageTarget {
                quantity: self.quantity,
                target,
            });
        }
    }
}
