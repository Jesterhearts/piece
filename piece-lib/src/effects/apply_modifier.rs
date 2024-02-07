use itertools::Itertools;

use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
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
    ) -> Vec<EffectBundle> {
        let modifier = ModifierId::upload_temporary_modifier(
            db,
            source.unwrap(),
            self.modifier.as_ref().cloned().unwrap(),
        );

        if self.modifier.modifier.entire_battlefield {
            db[modifier].active = true;
            for card in db.battlefield[db[source.unwrap()].controller]
                .iter()
                .copied()
                .collect_vec()
            {
                card.apply_modifiers_layered(db);
            }
        } else {
            for target in selected.iter() {
                target.id(db).unwrap().apply_modifier(db, modifier)
            }
        }

        vec![]
    }
}
