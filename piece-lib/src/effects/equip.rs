use itertools::Itertools;

use crate::{
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database, ModifierId},
    protogen::{
        effects::{BattlefieldModifier, Duration, Equip},
        targets::Location,
    },
};

impl EffectBehaviors for Equip {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let source = source.unwrap();
        let Some(target) = selected
            .first()
            .filter(|target| matches!(target.location, Some(Location::ON_BATTLEFIELD)))
        else {
            return vec![];
        };

        let target = target.id(db).unwrap();
        if !target.can_be_targeted(db, db[source].controller) {
            return vec![];
        }

        for modifier in db
            .modifiers
            .iter()
            .filter_map(|(id, modifier)| {
                if modifier.source == source {
                    Some(id)
                } else {
                    None
                }
            })
            .copied()
            .collect_vec()
        {
            db.modifiers.get_mut(&modifier).unwrap().modifying.clear();
        }

        for modifier in self.modifiers.iter() {
            let modifier = ModifierId::upload_temporary_modifier(
                db,
                source,
                BattlefieldModifier {
                    modifier: protobuf::MessageField::some(modifier.clone()),
                    duration: protobuf::EnumOrUnknown::new(
                        Duration::UNTIL_SOURCE_LEAVES_BATTLEFIELD,
                    ),
                    ..Default::default()
                },
            );
            target.apply_modifier(db, modifier);
        }

        vec![]
    }
}
