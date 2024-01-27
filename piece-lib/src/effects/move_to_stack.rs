use crate::{
    abilities::Ability,
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, CastFrom, Database},
    log::Log,
    protogen::{effects::MoveToStack, targets::Location},
    stack::{Stack, TargetType},
};

impl EffectBehaviors for MoveToStack {
    fn apply(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        modes: &[usize],
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let targets = selected.restore();
        let casting = selected.pop().unwrap();
        let cast_from = match casting.location.unwrap() {
            Location::IN_HAND => CastFrom::Hand,
            Location::IN_EXILE => CastFrom::Exile,
            Location::IN_GRAVEYARD => CastFrom::Graveyard,
            _ => unreachable!(),
        };

        let mut pending = vec![];
        match &casting.target_type {
            TargetType::Card(card) => {
                Log::cast(db, *card);
                pending.extend(card.move_to_stack(db, targets, cast_from, modes.to_vec()));
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

                pending.extend(Stack::push_ability(db, *source, ability.clone(), targets))
            }
            _ => unreachable!(),
        }

        pending
    }
}
