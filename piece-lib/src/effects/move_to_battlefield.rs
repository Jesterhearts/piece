use crate::{
    battlefield::Battlefields,
    effects::{handle_replacements, ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database},
    log::LogId,
    protogen::{
        effects::{
            replacement_effect::Replacing,
            static_ability::{self, ForceEtbTapped},
            MoveToBattlefield,
        },
        triggers::TriggerSource,
    },
    stack::Stack,
};

impl EffectBehaviors for MoveToBattlefield {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        if skip_replacement {
            let mut pending = vec![];
            let adding_to_battlefield = selected.restore();
            for (add_to_battlefield, aura_target) in adding_to_battlefield
                .into_iter()
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
                    if let Some(aura_target) = aura_target {
                        aura_target.id(db).unwrap().apply_aura(db, target_card);
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

            pending
        } else {
            for target in selected.iter() {
                let card = target.id(db).unwrap();
                db[card].replacements_active = true;
            }

            handle_replacements(
                db,
                selected.clone(),
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
