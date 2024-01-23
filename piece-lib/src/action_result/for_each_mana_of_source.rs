use crate::{
    action_result::Action,
    effects::EffectBehaviors,
    in_play::{CardId, Database},
    pending_results::PendingResults,
    protogen::{effects::Effect, mana::ManaSource},
};

#[derive(Debug, Clone)]
pub(crate) struct ForEachManaOfSource {
    pub(crate) card: CardId,
    pub(crate) source: protobuf::EnumOrUnknown<ManaSource>,
    pub(crate) effect: protobuf::MessageField<Effect>,
}

impl Action for ForEachManaOfSource {
    fn apply(&self, db: &mut Database) -> PendingResults {
        let Self {
            card,
            source,
            effect,
        } = self;
        let mut results = PendingResults::default();
        if let Some(from_source) = db[*card].sourced_mana.get(&source.enum_value().unwrap()) {
            for _ in 0..*from_source {
                effect.effect.as_ref().unwrap().push_pending_behavior(
                    db,
                    *card,
                    db[*card].controller,
                    &mut results,
                );
            }
        }

        results
    }
}
