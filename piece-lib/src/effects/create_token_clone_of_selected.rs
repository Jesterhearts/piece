use crate::{
    effects::{handle_replacements, EffectBehaviors, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    log::LogId,
    protogen::effects::{replacement_effect::Replacing, CreateTokenCloneOfSelected},
    stack::{Selected, TargetType},
};

impl EffectBehaviors for CreateTokenCloneOfSelected {
    fn apply(
        &mut self,
        db: &mut Database,
        pending: &mut PendingEffects,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        skip_replacement: bool,
    ) {
        let controller = selected.last().unwrap().player().unwrap();
        if skip_replacement {
            let copying = selected.first().unwrap().id(db).unwrap();
            let copy = copying.token_copy_of(db, controller.into());

            selected.clear();
            selected.push(Selected {
                location: None,
                target_type: TargetType::Card(copy),
                targeted: false,
                restrictions: vec![],
            });
        } else {
            handle_replacements(
                db,
                pending,
                source,
                Replacing::TOKEN_CREATION,
                self.clone(),
                |source, restrictions| {
                    controller.passes_restrictions(
                        db,
                        LogId::current(db),
                        db[source].controller,
                        restrictions,
                    )
                },
            )
        }
    }
}
