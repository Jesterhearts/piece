use crate::{
    abilities::Ability,
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, CastFrom, Database},
    log::Log,
    protogen::{
        effects::{Cascade, MoveToStack, TriggeredAbility},
        targets::Location,
    },
    stack::{Stack, TargetType},
};

impl EffectBehaviors for MoveToStack {
    fn apply(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let targets = selected.restore();
        let casting = selected.pop().unwrap();

        let mut pending = vec![];
        match &casting.target_type {
            TargetType::Card(card) => {
                let cast_from = match casting.location.unwrap() {
                    Location::IN_HAND => CastFrom::Hand,
                    Location::IN_EXILE => CastFrom::Exile,
                    Location::IN_GRAVEYARD => CastFrom::Graveyard,
                    loc => unreachable!("{}", loc.as_ref()),
                };
                Log::cast(db, *card);

                pending.extend(card.move_to_stack(db, targets, cast_from, selected.modes.clone()));
                card.apply_modifiers_layered(db);

                for _ in 0..card.cascade(db) {
                    pending.extend(Stack::push_ability(
                        db,
                        *card,
                        Ability::TriggeredAbility(TriggeredAbility {
                            effects: vec![Cascade::default().into()],
                            oracle_text: "Cascade".to_string(),
                            ..Default::default()
                        }),
                        vec![],
                    ));
                }
            }
            TargetType::Ability { source, ability } => {
                match ability {
                    Ability::Activated(activated) => {
                        Log::activated(db, *source, *activated);
                    }
                    Ability::Etb(_) | Ability::TriggeredAbility(_) => {
                        Log::etb_or_triggered(db, *source);
                    }
                    _ => {}
                }

                pending.extend(Stack::push_ability(db, *source, ability.clone(), targets))
            }
            tt => unreachable!("{:?}", tt),
        }

        pending
    }
}
