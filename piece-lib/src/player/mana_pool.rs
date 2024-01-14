use std::collections::BTreeMap;

use convert_case::{Case, Casing};
use strum::IntoEnumIterator;

use crate::{
    in_play::{CardId, Database},
    protogen::{
        cost::ManaCost,
        mana::ManaSource,
        mana::{Mana, ManaRestriction},
        types::Type,
    },
    types::TypeSet,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpendReason {
    Casting(CardId),
    Activating(CardId),
    Other,
}

impl SpendReason {
    fn card(&self) -> Option<CardId> {
        match self {
            SpendReason::Casting(card) => Some(*card),
            SpendReason::Activating(source) => Some(*source),
            SpendReason::Other => None,
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
                .entry(ManaSource::ANY)
                .or_default()
                .entry(ManaRestriction::NONE)
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
                .entry(ManaSource::ANY)
                .or_default()
                .entry(ManaRestriction::NONE)
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
            sourced = if let ManaSource::ANY = source {
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
            let card = reason.card();

            if card.is_none() {
                let restricted = sourced.get_mut(&ManaRestriction::NONE);
                if let Some(restricted) = restricted {
                    let Some(mana) = restricted.checked_sub(1) else {
                        return (false, ManaSource::ANY);
                    };

                    *restricted = mana;
                    return (true, ultimate_source);
                } else {
                    return (false, ManaSource::ANY);
                }
            }

            let card = card.unwrap();
            if card.types_intersect(db, &TypeSet::from([Type::ARTIFACT])) {
                let restricted = if let Some(restricted) =
                    sourced.get_mut(&ManaRestriction::ARTIFACT_SPELL_OR_ABILITY)
                {
                    Some(restricted)
                } else if matches!(reason, SpendReason::Activating(_)) {
                    if let Some(restricted) = sourced.get_mut(&ManaRestriction::ACTIVATE_ABILITY) {
                        Some(restricted)
                    } else {
                        sourced.get_mut(&ManaRestriction::NONE)
                    }
                } else {
                    sourced.get_mut(&ManaRestriction::NONE)
                };

                if let Some(restricted) = restricted {
                    let Some(mana) = restricted.checked_sub(1) else {
                        return (false, ManaSource::ANY);
                    };

                    *restricted = mana;
                    (true, ultimate_source)
                } else {
                    (false, ManaSource::ANY)
                }
            } else if matches!(reason, SpendReason::Activating(_)) {
                let restricted =
                    if let Some(restricted) = sourced.get_mut(&ManaRestriction::ACTIVATE_ABILITY) {
                        Some(restricted)
                    } else {
                        sourced.get_mut(&ManaRestriction::NONE)
                    };

                if let Some(restricted) = restricted {
                    let Some(mana) = restricted.checked_sub(1) else {
                        return (false, ManaSource::ANY);
                    };

                    *restricted = mana;
                    (true, ultimate_source)
                } else {
                    (false, ManaSource::ANY)
                }
            } else {
                let restricted = sourced.get_mut(&ManaRestriction::NONE);
                if let Some(restricted) = restricted {
                    let Some(mana) = restricted.checked_sub(1) else {
                        return (false, ManaSource::ANY);
                    };

                    *restricted = mana;
                    (true, ultimate_source)
                } else {
                    (false, ManaSource::ANY)
                }
            }
        } else {
            (false, ManaSource::ANY)
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
            ManaCost::WHITE => {
                if let (false, _) = mana_pool.spend(db, Mana::WHITE, source, reason) {
                    return false;
                }
            }
            ManaCost::BLUE => {
                if let (false, _) = mana_pool.spend(db, Mana::BLUE, source, reason) {
                    return false;
                }
            }
            ManaCost::BLACK => {
                if let (false, _) = mana_pool.spend(db, Mana::BLACK, source, reason) {
                    return false;
                }
            }
            ManaCost::RED => {
                if let (false, _) = mana_pool.spend(db, Mana::RED, source, reason) {
                    return false;
                }
            }
            ManaCost::GREEN => {
                if let (false, _) = mana_pool.spend(db, Mana::GREEN, source, reason) {
                    return false;
                }
            }
            ManaCost::COLORLESS => {
                if let (false, _) = mana_pool.spend(db, Mana::COLORLESS, source, reason) {
                    return false;
                }
            }
            ManaCost::GENERIC => {
                if let Some(max) = mana_pool.max(db, reason) {
                    if let (false, _) = mana_pool.spend(db, max, source, reason) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            ManaCost::X => {}
            ManaCost::TWO_X => {}
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
                if *restriction == ManaRestriction::NONE {
                    return true;
                }

                if let Some(card) = reason.card() {
                    card.types_intersect(db, &TypeSet::from([Type::ARTIFACT]))
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
            if *restriction == ManaRestriction::NONE {
                Some(count)
            } else if let Some(card) = reason.card() {
                if card.types_intersect(db, &TypeSet::from([Type::ARTIFACT])) {
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
        if let ManaRestriction::NONE = restriction {
            if source != ManaSource::ANY {
                result.push_str(&format!(
                    " ({}): {}",
                    source.as_ref().to_case(Case::Title),
                    amount
                ));
            } else {
                result.push_str(&format!(": {}", amount));
            }
        } else if source != ManaSource::ANY {
            result.push_str(&format!(
                " ({}) ({}): {}",
                source.as_ref().to_case(Case::Title),
                restriction.as_ref().to_case(Case::Title),
                amount
            ));
        } else {
            result.push_str(&format!("({}): {}", restriction.as_ref(), amount));
        }

        results.push(result)
    }

    results
}
