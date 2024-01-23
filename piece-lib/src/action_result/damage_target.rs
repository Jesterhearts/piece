use crate::{
    action_result::Action, in_play::Database, pending_results::PendingResults, stack::ActiveTarget,
};

#[derive(Debug, Clone)]
pub(crate) struct DamageTarget {
    pub(crate) quantity: u32,
    pub(crate) target: ActiveTarget,
}

impl Action for DamageTarget {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self { quantity, target } = self;
        match target {
            ActiveTarget::Battlefield { id } => {
                id.mark_damage(db, *quantity);
            }
            ActiveTarget::Player { id } => {
                db.all_players[*id].life_total -= *quantity as i32;
            }
            _ => unreachable!(),
        }
        PendingResults::default()
    }
}
