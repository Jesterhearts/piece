use crate::{
    effects::{handle_replacements, EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    log::LogId,
    protogen::effects::{
        replacement_effect::Replacing, CreateTokenCloneOfSelected, MoveToBattlefield,
    },
    stack::{Selected, TargetType},
};

impl EffectBehaviors for CreateTokenCloneOfSelected {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        let controller = selected.last().unwrap().player().unwrap();
        if skip_replacement {
            let copying = selected.first().unwrap().id(db).unwrap();
            let copy = copying.token_copy_of(db, controller.into());

            vec![
                EffectBundle {
                    push_on_enter: Some(vec![Selected {
                        location: None,
                        target_type: TargetType::Card(copy),
                        targeted: false,
                        restrictions: vec![],
                    }]),
                    source,
                    effects: vec![MoveToBattlefield::default().into()],
                    ..Default::default()
                },
                EffectBundle {
                    push_on_enter: Some(vec![]),
                    ..Default::default()
                },
            ]
        } else {
            handle_replacements(
                db,
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
