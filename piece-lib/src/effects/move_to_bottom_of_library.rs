use crate::{
    battlefield::Battlefields,
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database},
    library::Library,
    log::LogId,
    protogen::effects::MoveToBottomOfLibrary,
};

impl EffectBehaviors for MoveToBottomOfLibrary {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let mut pending = vec![];
        for target in selected.iter() {
            if !target.targeted
                || target.id(db).unwrap().passes_restrictions(
                    db,
                    LogId::current(db),
                    source.unwrap(),
                    &target.restrictions,
                )
            {
                let target = target.id(db).unwrap();
                pending.extend(Battlefields::maybe_leave_battlefield(db, target));
                target.move_to_limbo(db);
                if !db[target].token {
                    Library::place_on_bottom(db, db[target].owner, target);
                }
            }
        }

        pending
    }
}
