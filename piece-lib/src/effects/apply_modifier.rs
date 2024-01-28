use crate::{
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database, ModifierId},
    protogen::effects::ApplyModifier,
};

impl EffectBehaviors for ApplyModifier {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let modifier = ModifierId::upload_temporary_modifier(
            db,
            source.unwrap(),
            self.modifier.as_ref().cloned().unwrap(),
        );

        for target in selected.iter() {
            target.id(db).unwrap().apply_modifier(db, modifier)
        }

        vec![]
    }
}
