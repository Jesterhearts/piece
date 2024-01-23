use crate::{
    action_result::Action,
    battlefield::Battlefields,
    in_play::Database,
    pending_results::PendingResults,
    stack::{Entry, StackId},
};

#[derive(Debug, Clone)]
pub(crate) struct SpellCountered {
    pub(crate) index: StackId,
}

impl Action for SpellCountered {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { index } = self;
        match &db.stack.entries.get(index).unwrap().ty {
            Entry::Card(card) => Battlefields::stack_to_graveyard(db, *card),
            Entry::Ability { .. } => {
                db.stack.entries.shift_remove(index);
                PendingResults::default()
            }
        }
    }
}
