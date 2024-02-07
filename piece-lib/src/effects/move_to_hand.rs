use crate::{
    battlefield::Battlefields,
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    log::LogId,
    protogen::effects::MoveToHand,
};

impl EffectBehaviors for MoveToHand {
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
                target.move_to_hand(db);
            }
        }

        pending
    }
}
