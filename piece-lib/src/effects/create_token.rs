use crate::{
    effects::{handle_replacements, ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database},
    log::LogId,
    protogen::effects::{replacement_effect::Replacing, CreateToken},
    stack::{Selected, TargetType},
};

impl EffectBehaviors for CreateToken {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let owner = selected.first().unwrap().player().unwrap();
        if skip_replacement {
            let card = CardId::upload_token(db, owner, self.token.as_ref().cloned().unwrap());

            selected.clear();
            selected.push(Selected {
                location: None,
                target_type: TargetType::Card(card),
                targeted: false,
                restrictions: vec![],
            });
            vec![]
        } else {
            handle_replacements(
                db,
                selected.clone(),
                source,
                Replacing::TOKEN_CREATION,
                self.clone(),
                |source, restrictions| {
                    owner.passes_restrictions(
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
