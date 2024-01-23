use crate::{
    action_result::{create_token_copy_with_replacements, Action},
    in_play::{CardId, Database},
    pending_results::PendingResults,
    protogen::effects::{replacement_effect::Replacing, ModifyBattlefield},
};

#[derive(Debug, Clone)]
pub(crate) struct CreateTokenCopyOf {
    pub(crate) source: CardId,
    pub(crate) target: CardId,
    pub(crate) modifiers: Vec<ModifyBattlefield>,
}

impl Action for CreateTokenCopyOf {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            source,
            target,
            modifiers,
        } = self;
        let mut results = PendingResults::default();

        let mut replacements = db
            .replacement_abilities_watching(Replacing::TOKEN_CREATION)
            .into_iter();

        create_token_copy_with_replacements(
            db,
            *source,
            *target,
            modifiers,
            &mut replacements,
            &mut results,
        );

        results
    }
}
