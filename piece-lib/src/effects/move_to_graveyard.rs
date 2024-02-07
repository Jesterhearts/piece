use itertools::Itertools;

use crate::{
    battlefield::Battlefields,
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    log::LogId,
    protogen::{effects::MoveToGraveyard, triggers::TriggerSource},
    stack::Stack,
};

impl EffectBehaviors for MoveToGraveyard {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        let results = move_card_to_graveyard(db, selected, source);
        for card in db.cards.keys().copied().collect_vec() {
            card.apply_modifiers_layered(db);
        }

        results
    }
}

pub(crate) fn move_card_to_graveyard(
    db: &mut Database,
    selected: &mut SelectedStack,
    source: Option<CardId>,
) -> Vec<EffectBundle> {
    let mut pending = vec![];

    for target in selected.iter() {
        let Some(card) = target.id(db) else {
            continue;
        };

        if !target.targeted
            || card.passes_restrictions(
                db,
                LogId::current(db),
                source.unwrap(),
                &target.restrictions,
            )
        {
            for (listener, trigger) in
                db.active_triggers_of_source(TriggerSource::PUT_INTO_GRAVEYARD)
            {
                if (target.location.is_some()
                    && target.location.unwrap() == trigger.trigger.from.enum_value().unwrap())
                    && card.passes_restrictions(
                        db,
                        LogId::current(db),
                        listener,
                        &trigger.trigger.restrictions,
                    )
                {
                    pending.push(Stack::move_trigger_to_stack(db, listener, trigger));
                }
            }

            pending.extend(Battlefields::maybe_leave_battlefield(db, card));
            card.move_to_graveyard(db);
        }
    }

    pending
}
