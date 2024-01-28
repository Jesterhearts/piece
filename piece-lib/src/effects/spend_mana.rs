use itertools::Itertools;

use crate::{
    effects::{ApplyResult, EffectBehaviors, SelectedStack},
    in_play::{CardId, Database},
    player::Player,
    protogen::effects::SpendMana,
};

impl EffectBehaviors for SpendMana {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        _selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<ApplyResult> {
        let player = db[source.unwrap()].controller;
        let spent = Player::spend_mana(
            db,
            player.into(),
            &self
                .mana
                .iter()
                .map(|mana| mana.enum_value().unwrap())
                .collect_vec(),
            &self
                .mana_sources
                .iter()
                .map(|source| source.enum_value().unwrap())
                .collect_vec(),
            self.reason.reason.as_ref().unwrap(),
        );

        assert!(
            spent,
            "Should have validated could spend mana before spending."
        );

        vec![]
    }
}
