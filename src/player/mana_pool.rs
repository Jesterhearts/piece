use std::collections::{BTreeMap, HashMap};

use anyhow::anyhow;
use bevy_ecs::component::Component;
use derive_more::{Deref, DerefMut};
use indexmap::IndexSet;
use strum::IntoEnumIterator;

use crate::{
    in_play::{AbilityId, CardId, Database},
    mana::{Mana, ManaCost, ManaRestriction},
    protogen,
    types::Type,
};

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, Component)]
pub(crate) struct SourcedMana(pub(crate) HashMap<ManaSource, usize>);

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    bevy_ecs::component::Component,
    strum::AsRefStr,
    strum::EnumIter,
)]
pub(crate) enum ManaSource {
    Any,
    BarracksOfTheThousand,
    Treasure,
    Cave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpendReason {
    Casting(CardId),
    Activating(AbilityId),
    Other,
}

impl SpendReason {
    fn card(&self, db: &Database) -> Option<CardId> {
        match self {
            SpendReason::Casting(card) => Some(*card),
            SpendReason::Activating(ability) => Some(ability.source(db)),
            SpendReason::Other => None,
        }
    }
}

impl TryFrom<&protogen::targets::ManaSource> for ManaSource {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::targets::ManaSource) -> Result<Self, Self::Error> {
        value
            .source
            .as_ref()
            .ok_or_else(|| anyhow!("Expected source to have a source set"))
            .map(Self::from)
    }
}

impl From<&protogen::targets::mana_source::Source> for ManaSource {
    fn from(value: &protogen::targets::mana_source::Source) -> Self {
        match value {
            protogen::targets::mana_source::Source::BarracksOfTheThousand(_) => {
                Self::BarracksOfTheThousand
            }
            protogen::targets::mana_source::Source::Cave(_) => Self::Cave,
            protogen::targets::mana_source::Source::Treasure(_) => Self::Treasure,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ManaPool {
    pub(crate) sourced: BTreeMap<Mana, BTreeMap<ManaSource, BTreeMap<ManaRestriction, usize>>>,
}

impl Default for ManaPool {
    fn default() -> Self {
        let mut sourced =
            BTreeMap::<Mana, BTreeMap<ManaSource, BTreeMap<ManaRestriction, usize>>>::default();

        for mana in Mana::iter() {
            *sourced
                .entry(mana)
                .or_default()
                .entry(ManaSource::Any)
                .or_default()
                .entry(ManaRestriction::None)
                .or_default() = 0;
        }

        Self { sourced }
    }
}

impl ManaPool {
    pub(crate) fn drain(&mut self) {
        self.sourced.clear();
        for mana in Mana::iter() {
            *self
                .sourced
                .entry(mana)
                .or_default()
                .entry(ManaSource::Any)
                .or_default()
                .entry(ManaRestriction::None)
                .or_default() = 0;
        }
    }

    pub(crate) fn apply(&mut self, mana: Mana, source: ManaSource, restriction: ManaRestriction) {
        let sourced = self
            .sourced
            .entry(mana)
            .or_default()
            .entry(source)
            .or_default()
            .entry(restriction)
            .or_default();
        *sourced = sourced.saturating_add(1);
    }

    pub(crate) fn spend(
        &mut self,
        db: &Database,
        mana: Mana,
        source: ManaSource,
        reason: SpendReason,
    ) -> (bool, ManaSource) {
        let mana = self.sourced.entry(mana).or_default();
        let mut ultimate_source = source;
        let mut sourced = mana
            .get_mut(&source)
            .filter(|sourced| has_available_mana(sourced, reason, db));
        if sourced.is_none() {
            sourced = if let ManaSource::Any = source {
                // I know it never loops, I just want my break value.
                #[allow(clippy::never_loop)]
                let alt = 'outer: loop {
                    for source in ManaSource::iter() {
                        if let Some(alt) = mana
                            .get_mut(&source)
                            .filter(|sourced| has_available_mana(sourced, reason, db))
                        {
                            ultimate_source = source;
                            break 'outer Some(alt);
                        }
                    }
                    break None;
                };

                alt
            } else {
                None
            }
        }

        if let Some(sourced) = sourced {
            let card = reason.card(db);

            if card.is_none() {
                let restricted = sourced.get_mut(&ManaRestriction::None);
                if let Some(restricted) = restricted {
                    let Some(mana) = restricted.checked_sub(1) else {
                        return (false, ManaSource::Any);
                    };

                    *restricted = mana;
                    return (true, ultimate_source);
                } else {
                    return (false, ManaSource::Any);
                }
            }

            let card = card.unwrap();
            if card.types_intersect(db, &IndexSet::from([Type::Artifact])) {
                let restricted = if let Some(restricted) =
                    sourced.get_mut(&ManaRestriction::ArtifactSpellOrAbility)
                {
                    Some(restricted)
                } else {
                    sourced.get_mut(&ManaRestriction::None)
                };

                if let Some(restricted) = restricted {
                    let Some(mana) = restricted.checked_sub(1) else {
                        return (false, ManaSource::Any);
                    };

                    *restricted = mana;
                    (true, ultimate_source)
                } else {
                    (false, ManaSource::Any)
                }
            } else {
                let restricted = sourced.get_mut(&ManaRestriction::None);
                if let Some(restricted) = restricted {
                    let Some(mana) = restricted.checked_sub(1) else {
                        return (false, ManaSource::Any);
                    };

                    *restricted = mana;
                    (true, ultimate_source)
                } else {
                    (false, ManaSource::Any)
                }
            }
        } else {
            (false, ManaSource::Any)
        }
    }

    pub(crate) fn can_spend(
        &self,
        db: &Database,
        cost: ManaCost,
        source: ManaSource,
        reason: SpendReason,
    ) -> bool {
        let mut mana_pool = self.clone();
        match cost {
            ManaCost::White => {
                if let (false, _) = mana_pool.spend(db, Mana::White, source, reason) {
                    return false;
                }
            }
            ManaCost::Blue => {
                if let (false, _) = mana_pool.spend(db, Mana::Blue, source, reason) {
                    return false;
                }
            }
            ManaCost::Black => {
                if let (false, _) = mana_pool.spend(db, Mana::Black, source, reason) {
                    return false;
                }
            }
            ManaCost::Red => {
                if let (false, _) = mana_pool.spend(db, Mana::Red, source, reason) {
                    return false;
                }
            }
            ManaCost::Green => {
                if let (false, _) = mana_pool.spend(db, Mana::Green, source, reason) {
                    return false;
                }
            }
            ManaCost::Colorless => {
                if let (false, _) = mana_pool.spend(db, Mana::Colorless, source, reason) {
                    return false;
                }
            }
            ManaCost::Generic(count) => {
                for _ in 0..count {
                    if let Some(max) = mana_pool.max(db, reason) {
                        if let (false, _) = mana_pool.spend(db, max, source, reason) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
            }
            ManaCost::X => {}
            ManaCost::TwoX => {}
        }

        true
    }

    pub(crate) fn all_mana(
        &self,
    ) -> impl Iterator<Item = (usize, Mana, ManaSource, ManaRestriction)> + std::fmt::Debug + '_
    {
        self.sourced.iter().flat_map(|(mana, sourced)| {
            sourced.iter().flat_map(|(source, restricted)| {
                restricted
                    .iter()
                    .map(|(restriction, count)| (*count, *mana, *source, *restriction))
            })
        })
    }

    pub(crate) fn available_mana(
        &self,
    ) -> impl Iterator<Item = (usize, Mana, ManaSource, ManaRestriction)> + '_ {
        self.all_mana().filter(|(count, _, _, _)| *count > 0)
    }

    pub(crate) fn max(&self, db: &Database, reason: SpendReason) -> Option<Mana> {
        self.available_mana()
            .filter(|(_, _, _, restriction)| {
                if *restriction == ManaRestriction::None {
                    return true;
                }

                if let Some(card) = reason.card(db) {
                    card.types_intersect(db, &IndexSet::from([Type::Artifact]))
                } else {
                    false
                }
            })
            .max_by_key(|(count, _, _, _)| *count)
            .map(|(_, mana, _, _)| mana)
    }

    pub fn available_pool_display(&self) -> Vec<String> {
        let available = self.available_mana();

        display(available)
    }

    pub fn pools_display(&self) -> Vec<String> {
        let available = self.all_mana();

        display(available)
    }
}

fn has_available_mana(
    sourced: &BTreeMap<ManaRestriction, usize>,
    reason: SpendReason,
    db: &Database,
) -> bool {
    sourced
        .iter()
        .filter_map(|(restriction, count)| {
            if *restriction == ManaRestriction::None {
                Some(count)
            } else if let Some(card) = reason.card(db) {
                if card.types_intersect(db, &IndexSet::from([Type::Artifact])) {
                    Some(count)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .sum::<usize>()
        > 0
}

fn display(
    available: impl Iterator<Item = (usize, Mana, ManaSource, ManaRestriction)>,
) -> Vec<String> {
    let mut results = vec![];
    for (amount, symbol, source, restriction) in available {
        let mut result = String::default();
        symbol.push_mana_symbol(&mut result);
        if let ManaRestriction::None = restriction {
            if source != ManaSource::Any {
                result.push_str(&format!(" ({}): {}", source.as_ref(), amount));
            } else {
                result.push_str(&format!(": {}", amount));
            }
        } else if source != ManaSource::Any {
            result.push_str(&format!(
                " ({}) ({}): {}",
                source.as_ref(),
                restriction.as_ref(),
                amount
            ));
        } else {
            result.push_str(&format!("({}): {}", restriction.as_ref(), amount));
        }

        results.push(result)
    }

    results
}
