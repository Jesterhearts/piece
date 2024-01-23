use crate::{
    action_result::{create_token_copy_with_replacements, Action},
    in_play::{CardId, Database},
    pending_results::PendingResults,
    protogen::effects::{create_token::Token, replacement_effect::Replacing},
};

#[derive(Debug, Clone)]
pub(crate) struct CreateToken {
    pub(crate) source: CardId,
    pub(crate) token: Token,
}

impl Action for CreateToken {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { source, token } = self;
        let mut results = PendingResults::default();

        let mut replacements = db
            .replacement_abilities_watching(Replacing::TOKEN_CREATION)
            .into_iter();

        let card = CardId::upload_token(db, db[*source].controller.into(), token.clone());
        create_token_copy_with_replacements(
            db,
            *source,
            card,
            &[],
            &mut replacements,
            &mut results,
        );

        results
    }
}
