use crate::{
    action_result::Action,
    in_play::{CardId, Database},
    pending_results::PendingResults,
    player::{mana_pool::SpendReason, Player},
    protogen::mana::{Mana, ManaSource},
};

#[derive(Debug, Clone)]
pub(crate) struct SpendMana {
    pub(crate) card: CardId,
    pub(crate) mana: Vec<Mana>,
    pub(crate) sources: Vec<ManaSource>,
    pub(crate) reason: SpendReason,
}

impl Action for SpendMana {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            card,
            mana,
            sources,
            reason,
        } = self;

        card.mana_from_source(db, sources);
        let controller = db[*card].controller;
        let spent = Player::spend_mana(db, controller.into(), mana, sources, *reason);
        assert!(
            spent,
            "Should have validated mana could be spent before spending."
        );
        PendingResults::default()
    }
}
