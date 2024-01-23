use crate::{
    abilities::Ability,
    action_result::Action,
    in_play::{CardId, Database},
    log::Log,
    pending_results::PendingResults,
    protogen::triggers::TriggerSource,
    stack::{ActiveTarget, Stack},
};

#[derive(Debug, Clone)]
pub(crate) struct AddAbilityToStack {
    pub(crate) source: CardId,
    pub(crate) ability: Ability,
    pub(crate) targets: Vec<Vec<ActiveTarget>>,
    pub(crate) x_is: Option<usize>,
}

impl Action for AddAbilityToStack {
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

        let pending_results = PendingResults::default();
        let mut results = pending_results;

        if let Ability::Activated(ability) = ability {
            Log::activated(db, *source, *ability);
            db.turn.activated_abilities.insert(*ability);

            for (listener, trigger) in
                db.active_triggers_of_source(TriggerSource::ABILITY_ACTIVATED)
            {
                results.extend(Stack::move_trigger_to_stack(db, listener, trigger));
            }
        } else {
            Log::triggered(db, *source);
        }
        results.extend(Stack::push_ability(
            db,
            *source,
            ability.clone(),
            targets.clone(),
        ));

        results
    }
}
