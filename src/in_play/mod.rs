mod activated_ability_id;
mod card_id;
mod gain_mana_ability_id;
mod modifier_id;
mod static_ability_id;

use std::sync::atomic::AtomicUsize;

use indexmap::{IndexMap, IndexSet};

pub use activated_ability_id::{ActivatedAbilityId, ActivatedAbilityInPlay};
pub(crate) use card_id::target_from_location;
pub use card_id::CardId;
pub(crate) use card_id::CardInPlay;
pub use gain_mana_ability_id::{GainManaAbilityId, GainManaAbilityInPlay};
use itertools::Itertools;
pub(crate) use modifier_id::{ModifierId, ModifierInPlay};
pub(crate) use static_ability_id::{StaticAbilityId, StaticAbilityInPlay};

use crate::{
    abilities::TriggeredAbility,
    battlefield::Battlefields,
    effects::{ReplacementAbility, Replacing},
    exile::Exiles,
    graveyard::Graveyards,
    hand::Hands,
    library::Library,
    log::Log,
    player::{AllPlayers, Controller, Owner},
    stack::Stack,
    triggers::TriggerSource,
    turns::Turn,
};

static NEXT_ABILITY_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_CARD_ID: AtomicUsize = AtomicUsize::new(0);
static NEXT_MODIFIER_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumIter)]
pub(crate) enum CastFrom {
    Hand,
    Exile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExileReason {
    Cascade,
    Craft,
}

#[derive(Debug)]
pub struct Database {
    pub log: Log,

    pub(crate) cards: IndexMap<CardId, CardInPlay>,
    pub(crate) modifiers: IndexMap<ModifierId, ModifierInPlay>,
    pub(crate) activated_abilities: IndexMap<ActivatedAbilityId, ActivatedAbilityInPlay>,
    pub(crate) mana_abilities: IndexMap<GainManaAbilityId, GainManaAbilityInPlay>,
    pub(crate) static_abilities: IndexMap<StaticAbilityId, StaticAbilityInPlay>,

    pub(crate) gc_abilities: Vec<ActivatedAbilityId>,

    pub battlefield: Battlefields,
    pub graveyard: Graveyards,
    pub exile: Exiles,
    pub hand: Hands,

    pub stack: Stack,

    pub turn: Turn,
    pub all_players: AllPlayers,
}

pub struct OwnerViewMut<'db> {
    battlefield: &'db mut IndexSet<CardId>,
    graveyard: &'db mut IndexSet<CardId>,
    exile: &'db mut IndexSet<CardId>,
    hand: &'db mut IndexSet<CardId>,
    library: &'db mut Library,
}

impl std::ops::Index<CardId> for Database {
    type Output = CardInPlay;

    fn index(&self, index: CardId) -> &Self::Output {
        self.cards.get(&index).unwrap()
    }
}

impl std::ops::IndexMut<CardId> for Database {
    fn index_mut(&mut self, index: CardId) -> &mut Self::Output {
        self.cards.get_mut(&index).unwrap()
    }
}

impl std::ops::Index<ModifierId> for Database {
    type Output = ModifierInPlay;

    fn index(&self, index: ModifierId) -> &Self::Output {
        self.modifiers.get(&index).unwrap()
    }
}

impl std::ops::IndexMut<ModifierId> for Database {
    fn index_mut(&mut self, index: ModifierId) -> &mut Self::Output {
        self.modifiers.get_mut(&index).unwrap()
    }
}

impl std::ops::Index<StaticAbilityId> for Database {
    type Output = StaticAbilityInPlay;

    fn index(&self, index: StaticAbilityId) -> &Self::Output {
        self.static_abilities.get(&index).unwrap()
    }
}

impl std::ops::IndexMut<StaticAbilityId> for Database {
    fn index_mut(&mut self, index: StaticAbilityId) -> &mut Self::Output {
        self.static_abilities.get_mut(&index).unwrap()
    }
}

impl std::ops::Index<ActivatedAbilityId> for Database {
    type Output = ActivatedAbilityInPlay;

    fn index(&self, index: ActivatedAbilityId) -> &Self::Output {
        self.activated_abilities.get(&index).unwrap()
    }
}

impl std::ops::IndexMut<ActivatedAbilityId> for Database {
    fn index_mut(&mut self, index: ActivatedAbilityId) -> &mut Self::Output {
        self.activated_abilities.get_mut(&index).unwrap()
    }
}

impl std::ops::Index<GainManaAbilityId> for Database {
    type Output = GainManaAbilityInPlay;

    fn index(&self, index: GainManaAbilityId) -> &Self::Output {
        self.mana_abilities.get(&index).unwrap()
    }
}

impl std::ops::IndexMut<GainManaAbilityId> for Database {
    fn index_mut(&mut self, index: GainManaAbilityId) -> &mut Self::Output {
        self.mana_abilities.get_mut(&index).unwrap()
    }
}

impl Database {
    pub fn new(all_players: AllPlayers) -> Self {
        let mut battlefield = Battlefields::default();
        let mut graveyard = Graveyards::default();
        let mut exile = Exiles::default();
        let mut hand = Hands::default();

        for player in all_players.all_players() {
            battlefield
                .battlefields
                .entry(Controller::from(player))
                .or_default();
            graveyard.graveyards.entry(player).or_default();
            exile.exile_zones.entry(player).or_default();
            hand.hands.entry(player).or_default();
        }

        let turn = Turn::new(&all_players);

        Self {
            all_players,
            log: Default::default(),
            cards: Default::default(),
            modifiers: Default::default(),
            activated_abilities: Default::default(),
            mana_abilities: Default::default(),
            static_abilities: Default::default(),
            gc_abilities: Default::default(),
            battlefield,
            graveyard,
            exile,
            hand,
            stack: Default::default(),
            turn,
        }
    }

    pub(crate) fn owner_view_mut(&mut self, owner: Owner) -> OwnerViewMut {
        OwnerViewMut {
            battlefield: &mut self.battlefield[owner],
            graveyard: &mut self.graveyard[owner],
            exile: &mut self.exile[owner],
            hand: &mut self.hand[owner],
            library: &mut self.all_players[owner].library,
        }
    }

    pub(crate) fn active_triggers_of_source(
        &self,
        source: TriggerSource,
    ) -> Vec<(CardId, TriggeredAbility)> {
        self.battlefield
            .battlefields
            .values()
            .flat_map(|b| b.iter())
            .flat_map(|card| {
                self[*card]
                    .modified_triggers
                    .get(&source)
                    .iter()
                    .flat_map(|triggers| triggers.iter())
                    .map(|ability| (*card, ability.clone()))
                    .collect_vec()
            })
            .collect_vec()
    }

    pub(crate) fn replacement_abilities_watching(
        &self,
        replacement: Replacing,
    ) -> Vec<(CardId, ReplacementAbility)> {
        self.cards
            .keys()
            .copied()
            .filter(|card| self[*card].replacements_active)
            .flat_map(|card| {
                self[card]
                    .modified_replacement_abilities
                    .get(&replacement)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(move |replacing| (card, replacing))
            })
            .collect_vec()
    }
}
