use crate::{
    battlefield::Battlefields,
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    library::Library,
    log::LogId,
    protogen::effects::MoveToTopOfLibrary,
};

impl EffectBehaviors for MoveToTopOfLibrary {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
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
                    Library::place_under_top(db, db[target].owner, target, self.under as usize);
                }
            }
        }

        pending
    }
}
