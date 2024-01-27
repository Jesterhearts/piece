use crate::{
    effects::{EffectBehaviors, PendingEffects, SelectedStack},
    in_play::{CardId, Database},
    log::LogId,
    protogen::{effects::DamageSelected, targets::Location},
    stack::TargetType,
};

impl EffectBehaviors for DamageSelected {
    fn apply(
        &mut self,
        db: &mut Database,
        _pending: &mut PendingEffects,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        let count = self.count.count(db, source, selected);
        for target in selected.iter().filter(|target| {
            (matches!(target.location, Some(Location::ON_BATTLEFIELD)))
                || matches!(target.target_type, TargetType::Player(_))
        }) {
            match &target.target_type {
                TargetType::Card(card) => {
                    if !target.targeted
                        || (card.can_be_targeted(db, db[source.unwrap()].controller)
                            && card.passes_restrictions(
                                db,
                                LogId::current(db),
                                source.unwrap(),
                                &target.restrictions,
                            ))
                    {
                        card.mark_damage(db, count as u32)
                    }
                }
                TargetType::Player(player) => db.all_players[*player].life_total -= count,
                _ => unreachable!(),
            }
        }
    }
}
