use itertools::Itertools;

use crate::{
    effects::{EffectBehaviors, EffectBundle, SelectedStack},
    in_play::{CardId, Database},
    player::Player,
    protogen::{
        effects::SpendMana,
        mana::spend_reason::{Other, Reason},
    },
};

impl EffectBehaviors for SpendMana {
    fn apply(
        &mut self,
        db: &mut Database,
        source: Option<CardId>,
        _selected: &mut SelectedStack,
        _skip_replacement: bool,
    ) -> Vec<EffectBundle> {
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
            self.reason
                .reason
                .as_ref()
                .unwrap_or(&Reason::Other(Other::default())),
        );

        assert!(
            spent,
            "Should have validated could spend mana before spending."
        );

        vec![]
    }
}
