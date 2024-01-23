use itertools::Itertools;

use crate::{
    action_result::Action,
    battlefield::Battlefields,
    in_play::{CardId, Database},
    log::LogId,
    pending_results::PendingResults,
    protogen::targets::Restriction,
};

#[derive(Debug, Clone)]
pub(crate) struct DestroyEach {
    pub(crate) source: CardId,
    pub(crate) restrictions: Vec<Restriction>,
}

impl Action for DestroyEach {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            source,
            restrictions,
        } = self;

        let cards = db
            .battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .copied()
            .filter(|card| {
                card.passes_restrictions(db, LogId::current(db), *source, restrictions)
                    && !card.indestructible(db)
            })
            .collect_vec();

        let mut results = PendingResults::default();
        for card in cards {
            results.extend(Battlefields::permanent_to_graveyard(db, card));
        }

        results
    }
}
