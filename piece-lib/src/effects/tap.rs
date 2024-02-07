use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    log::LogId,
    protogen::{effects::Tap, triggers::TriggerSource},
    stack::Stack,
};

impl EffectBehaviors for Tap {
    fn apply(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        let mut pending = vec![];
        for target in selected.iter() {
            let target = target.id(db).unwrap();
            target.tap(db);

            for (listener, trigger) in db.active_triggers_of_source(TriggerSource::TAPPED) {
                if target.passes_restrictions(
                    db,
                    LogId::current(db),
                    listener,
                    &trigger.trigger.restrictions,
                ) {
                    pending.push(Stack::move_trigger_to_stack(db, listener, trigger));
                }
            }
        }

        pending
    }
}
