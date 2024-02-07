use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database, ExileReason},
    protogen::{effects::SelectExiledWithCascadeOrDiscover, targets::Location},
    stack::{Selected, TargetType},
};

impl EffectBehaviors for SelectExiledWithCascadeOrDiscover {
    fn apply(
        &mut self,
        db: &mut Database,
        _source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
        selected.extend(
            db.exile
                .exile_zones
                .values()
                .flat_map(|e| e.iter())
                .copied()
                .filter(|card| db[*card].exile_reason == Some(ExileReason::CascadeOrDiscover))
                .map(|card| Selected {
                    location: Some(Location::IN_EXILE),
                    target_type: TargetType::Card(card),
                    targeted: false,
                    restrictions: vec![],
                }),
        );

        vec![]
    }
}
