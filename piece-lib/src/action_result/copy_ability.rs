use crate::{
    abilities::Ability,
    action_result::Action,
    in_play::{CardId, Database},
    pending_results::PendingResults,
    stack::{ActiveTarget, Stack},
};

#[derive(Debug, Clone)]
pub(crate) struct CopyAbility {
    pub(crate) source: CardId,
    pub(crate) ability: Ability,
    pub(crate) targets: Vec<Vec<ActiveTarget>>,
    pub(crate) x_is: Option<usize>,
}

impl Action for CopyAbility {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            source,
            ability,
            targets,
            x_is,
        } = self;

        if let Some(x) = x_is {
            db[*source].x_is = *x;
        }
        Stack::push_ability(db, *source, ability.clone(), targets.clone())
    }
}
