use std::collections::HashSet;

use itertools::Itertools;

use crate::{
    battlefield::ActionResult,
    effects::{Effect, EffectBehaviors},
    in_play::{CardId, Database},
    pending_results::{choose_targets::ChooseTargets, PendingResults, TargetSource},
    player::Controller,
    protogen,
    stack::{ActiveTarget, Entry},
    targets::Restriction,
};

#[derive(Debug, Clone)]
pub struct CopySpellOrAbility {
    restrictions: Vec<Restriction>,
}

impl TryFrom<&protogen::effects::CopySpellOrAbility> for CopySpellOrAbility {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::CopySpellOrAbility) -> Result<Self, Self::Error> {
        Ok(Self {
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
        })
    }
}

impl EffectBehaviors for CopySpellOrAbility {
    fn needs_targets(&self, _db: &Database, _source: CardId) -> usize {
        1
    }

    fn wants_targets(&self, _db: &Database, _source: CardId) -> usize {
        1
    }

    fn valid_targets(
        &self,
        db: &Database,
        source: CardId,
        _controller: Controller,
        _already_chosen: &HashSet<ActiveTarget>,
    ) -> Vec<ActiveTarget> {
        db.stack
            .entries
            .iter()
            .filter_map(|(id, entry)| {
                if entry.passes_restrictions(db, source, &self.restrictions) {
                    Some(ActiveTarget::Stack { id: *id })
                } else {
                    None
                }
            })
            .collect_vec()
    }

    fn push_pending_behavior(
        &self,
        db: &mut Database,
        source: CardId,
        controller: Controller,
        results: &mut PendingResults,
    ) {
        let valid_targets =
            self.valid_targets(db, source, controller, results.all_currently_targeted());

        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect::from(self.clone())),
            valid_targets,
            source,
        ));
    }

    fn push_behavior_with_targets(
        &self,
        db: &mut Database,
        targets: Vec<ActiveTarget>,
        _apply_to_self: bool,
        _source: CardId,
        controller: Controller,
        results: &mut PendingResults,
    ) {
        for target in targets {
            let ActiveTarget::Stack { id } = target else {
                unreachable!()
            };

            match &db.stack.entries.get(&id).unwrap().ty {
                Entry::Card(card) => {
                    results.push_settled(ActionResult::CopyCardInStack(*card, controller));
                }
                Entry::Ability { source, ability } => {
                    results.add_ability_to_stack(*source, ability.clone());

                    for effect in ability.effects(db) {
                        let effect = effect.effect;
                        let valid_targets = effect.valid_targets(
                            db,
                            *source,
                            controller,
                            results.all_currently_targeted(),
                        );

                        if !valid_targets.is_empty() {
                            results.push_choose_targets(ChooseTargets::new(
                                TargetSource::Effect(effect),
                                valid_targets,
                                *source,
                            ));
                        }
                    }
                }
            }
        }
    }
}
