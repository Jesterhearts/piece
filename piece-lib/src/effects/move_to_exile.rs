use crate::{
    battlefield::Battlefields,
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database, ExileReason},
    log::LogId,
    protogen::{
        effects::{Duration, MoveToExile},
        targets::Location,
        triggers::TriggerSource,
    },
    stack::Stack,
};

impl EffectBehaviors for MoveToExile {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let mut pending = vec![];

        for target in selected.iter() {
            if !target.targeted
                || target.id(db).unwrap().passes_restrictions(
                    db,
                    LogId::current(db),
                    source.unwrap(),
                    &target.restrictions,
                )
            {
                if self.duration.enum_value().unwrap() == Duration::UNTIL_SOURCE_LEAVES_BATTLEFIELD
                    && !source.unwrap().is_in_location(db, Location::ON_BATTLEFIELD)
                {
                    return vec![];
                }

                let card = target.id(db).unwrap();
                if selected.crafting {
                    for (listener, trigger) in
                        db.active_triggers_of_source(TriggerSource::EXILED_DURING_CRAFT)
                    {
                        if (target.location.is_some()
                            && target.location.unwrap()
                                == trigger.trigger.from.enum_value().unwrap())
                            && card.passes_restrictions(
                                db,
                                LogId::current(db),
                                listener,
                                &trigger.trigger.restrictions,
                            )
                        {
                            pending.push(Stack::move_trigger_to_stack(db, listener, trigger));
                        }
                    }
                }

                pending.extend(Battlefields::maybe_leave_battlefield(db, card));
                card.move_to_exile(
                    db,
                    source.unwrap(),
                    if selected.crafting {
                        Some(ExileReason::Craft)
                    } else {
                        None
                    },
                    self.duration.enum_value().unwrap(),
                );
            }
        }

        pending
    }
}
