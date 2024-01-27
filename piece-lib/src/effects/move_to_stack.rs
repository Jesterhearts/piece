use itertools::Itertools;

use crate::{
    abilities::Ability,
    effects::{EffectBehaviors, PendingEffects, SelectedStack},
    in_play::{CardId, CastFrom, Database},
    log::Log,
    protogen::{effects::MoveToStack, targets::Location},
    stack::{Stack, TargetType},
};

impl EffectBehaviors for MoveToStack {
    fn apply(
        &mut self,
        db: &mut Database,
        pending: &mut PendingEffects,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        modes: &[usize],
        _skip_replacement: bool,
    ) {
        let (casting, targets) = selected.as_slice().split_first().unwrap();
        let cast_from = match casting.location.unwrap() {
            Location::IN_HAND => CastFrom::Hand,
            Location::IN_EXILE => CastFrom::Exile,
            Location::IN_GRAVEYARD => CastFrom::Graveyard,
            _ => unreachable!(),
        };

        match &casting.target_type {
            TargetType::Card(card) => {
                Log::cast(db, *card);
                pending.extend(card.move_to_stack(
                    db,
                    targets.iter().cloned().collect_vec(),
                    cast_from,
                    modes.to_vec(),
                ));
            }
            TargetType::Ability { source, ability } => {
                match ability {
                    Ability::Activated(activated) => {
                        Log::activated(db, *source, *activated);
                    }
                    Ability::EtbOrTriggered(_) => {
                        Log::etb_or_triggered(db, *source);
                    }
                    _ => {}
                }

                pending.extend(Stack::push_ability(
                    db,
                    *source,
                    ability.clone(),
                    targets.to_vec(),
                ))
            }
            _ => unreachable!(),
        }
    }
}
