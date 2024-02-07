use itertools::Itertools;

use crate::{
    abilities::Ability,
    battlefield::Battlefields,
    effects::{handle_replacements, EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    log::LogId,
    protogen::{
        effects::{
            replacement_effect::Replacing,
            static_ability::{self, ForceEtbTapped},
            ClearSelected, Effect, MoveToBattlefield, MoveToStack, PopSelected, PushSelected,
        },
        targets::Location,
        triggers::TriggerSource,
    },
    stack::{Selected, Stack, TargetType},
};

impl EffectBehaviors for MoveToBattlefield {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        if skip_replacement {
            let mut pending = vec![];
            let adding_to_battlefield = selected.restore();
            for (add_to_battlefield, aura_target) in adding_to_battlefield
                .iter()
                .zip(selected.iter().map(Some).chain(std::iter::repeat(None)))
            {
                if !add_to_battlefield.targeted
                    || add_to_battlefield.id(db).unwrap().passes_restrictions(
                        db,
                        LogId::current(db),
                        source.unwrap(),
                        &add_to_battlefield.restrictions,
                    )
                {
                    let target_card = add_to_battlefield.id(db).unwrap();
                    if let Some(aura_target) = aura_target.and_then(|target| target.id(db)) {
                        aura_target.apply_aura(db, target_card);
                    }

                    if let Some(etb) = db[target_card].modified_etb_ability.as_ref() {
                        let mut to_trigger = vec![
                            Effect::from(PushSelected::default()),
                            Effect::from(ClearSelected::default()),
                        ];
                        if let Some(targets) = etb.targets.as_ref() {
                            to_trigger.push(targets.clone().into());
                        }
                        if let Some(modes) = etb.modes.as_ref() {
                            to_trigger.push(modes.clone().into());
                        }
                        to_trigger.push(MoveToStack::default().into());
                        to_trigger.push(PopSelected::default().into());

                        pending.push(EffectBundle {
                            push_on_enter: Some(vec![Selected {
                                location: Some(Location::ON_BATTLEFIELD),
                                target_type: TargetType::Ability {
                                    source: target_card,
                                    ability: Ability::Etb(etb.clone()),
                                },
                                targeted: false,
                                restrictions: vec![],
                            }]),
                            source,
                            effects: to_trigger,
                            ..Default::default()
                        });
                    }

                    for (listener, trigger) in
                        db.active_triggers_of_source(TriggerSource::ENTERS_THE_BATTLEFIELD)
                    {
                        if (add_to_battlefield.location.is_some()
                            && add_to_battlefield.location.unwrap()
                                == trigger.trigger.from.enum_value().unwrap())
                            && target_card.passes_restrictions(
                                db,
                                LogId::current(db),
                                listener,
                                &trigger.trigger.restrictions,
                            )
                        {
                            pending.push(Stack::move_trigger_to_stack(db, listener, trigger));
                        }
                    }

                    let must_enter_tapped =
                        Battlefields::static_abilities(db)
                            .iter()
                            .any(|(ability, card)| match ability {
                                static_ability::Ability::ForceEtbTapped(ForceEtbTapped {
                                    restrictions,
                                    ..
                                }) => target_card.passes_restrictions(
                                    db,
                                    LogId::current(db),
                                    *card,
                                    restrictions,
                                ),
                                _ => false,
                            });
                    if must_enter_tapped
                        || target_card.faceup_face(db).etb_tapped
                        || self.enters_tapped
                    {
                        target_card.tap(db);
                    }
                    target_card.move_to_battlefield(db);
                }
            }

            for card in db.cards.keys().copied().collect_vec() {
                card.apply_modifiers_layered(db);
            }

            selected.save();
            selected.clear();
            selected.extend(adding_to_battlefield);

            pending
        } else {
            for target in selected.iter() {
                let card = target.id(db).unwrap();
                db[card].replacements_active = true;
            }

            handle_replacements(
                db,
                source,
                Replacing::ETB,
                self.clone(),
                |source, restrictions| {
                    selected.iter().any(|target| {
                        target.id(db).unwrap().passes_restrictions(
                            db,
                            LogId::current(db),
                            source,
                            restrictions,
                        )
                    })
                },
            )
        }
    }
}
