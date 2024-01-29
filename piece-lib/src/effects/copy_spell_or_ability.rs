use crate::{
    abilities::Ability,
    effects::{ApplyResult, EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    protogen::{
        effects::{ClearSelected, CopySpellOrAbility, MoveToStack, PopSelected, PushSelected},
        targets::Location,
    },
    stack::{Entry, Selected, TargetType},
};

impl EffectBehaviors for CopySpellOrAbility {
    fn apply(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let target = selected.first().unwrap();

        let TargetType::Stack(target) = target.target_type else {
            unreachable!()
        };

        let mut results = vec![];

        match db.stack.entries[&target].ty.clone() {
            Entry::Card(card) => {
                let controller = db[card].controller;
                let copy = card.token_copy_of(db, controller);
                db[copy].x_is = db[card].x_is;

                results.push(ApplyResult::PushBack(EffectBundle {
                    push_on_enter: Some(vec![Selected {
                        location: Some(Location::IN_STACK),
                        target_type: TargetType::Card(copy),
                        targeted: false,
                        restrictions: vec![],
                    }]),
                    source: Some(copy),
                    effects: vec![
                        PushSelected::default().into(),
                        ClearSelected::default().into(),
                        card.faceup_face(db).targets.get_or_default().clone().into(),
                        MoveToStack::default().into(),
                        PopSelected::default().into(),
                    ],
                    ..Default::default()
                }));
            }
            Entry::Ability { source, ability } => match &ability {
                Ability::Activated(activated) => {
                    results.push(ApplyResult::PushBack(EffectBundle {
                        push_on_enter: Some(vec![Selected {
                            location: Some(Location::IN_STACK),
                            target_type: TargetType::Ability {
                                source,
                                ability: ability.clone(),
                            },
                            targeted: false,
                            restrictions: vec![],
                        }]),
                        source: Some(source),
                        effects: vec![
                            PushSelected::default().into(),
                            ClearSelected::default().into(),
                            db[*activated]
                                .ability
                                .targets
                                .get_or_default()
                                .clone()
                                .into(),
                            MoveToStack::default().into(),
                            PopSelected::default().into(),
                        ],
                        ..Default::default()
                    }));
                }
                Ability::EtbOrTriggered(_) => todo!(),
                _ => unreachable!(),
            },
        }

        vec![]
    }
}
