use std::collections::{BTreeMap, HashMap};

use anyhow::anyhow;
use bevy_ecs::component::Component;
use derive_more::{Deref, DerefMut};
use itertools::Itertools;

use crate::{
    mana::{Mana, ManaCost},
    protogen,
};

#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, Component)]
pub struct SourcedMana(pub HashMap<ManaSource, usize>);

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
)]
pub enum ManaSource {
    Cave,
    Treasure,
}

impl TryFrom<&protogen::cost::ManaSource> for ManaSource {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::cost::ManaSource) -> Result<Self, Self::Error> {
        value
            .source
            .as_ref()
            .ok_or_else(|| anyhow!("Expected source to have a source set"))
            .map(Self::from)
    }
}

impl From<&protogen::cost::mana_source::Source> for ManaSource {
    fn from(value: &protogen::cost::mana_source::Source) -> Self {
        match value {
            protogen::cost::mana_source::Source::Cave(_) => Self::Cave,
            protogen::cost::mana_source::Source::Treasure(_) => Self::Treasure,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ManaPool {
    pub sourced: BTreeMap<Mana, BTreeMap<ManaSource, usize>>,

    pub white_mana: usize,
    pub blue_mana: usize,
    pub black_mana: usize,
    pub red_mana: usize,
    pub green_mana: usize,
    pub colorless_mana: usize,
}

impl ManaPool {
    pub fn apply(&mut self, mana: Mana, source: Option<ManaSource>) {
        if let Some(source) = source {
            let sourced_mana = self
                .sourced
                .entry(mana)
                .or_default()
                .entry(source)
                .or_default();
            *sourced_mana = sourced_mana.saturating_add(1);
        } else {
            match mana {
                Mana::White => self.white_mana = self.white_mana.saturating_add(1),
                Mana::Blue => self.blue_mana = self.blue_mana.saturating_add(1),
                Mana::Black => self.black_mana = self.black_mana.saturating_add(1),
                Mana::Red => self.red_mana = self.red_mana.saturating_add(1),
                Mana::Green => self.green_mana = self.green_mana.saturating_add(1),
                Mana::Colorless => self.colorless_mana = self.colorless_mana.saturating_add(1),
            }
        }
    }

    fn max_sourced_mana(&mut self, mana: Mana) -> Option<&mut usize> {
        if let Some(sources) = self.sourced.get_mut(&mana) {
            sources
                .iter_mut()
                .max_by_key(|(_, amount)| **amount)
                .map(|(_, amount)| amount)
        } else {
            None
        }
    }

    pub fn spend(&mut self, mana: Mana, source: Option<ManaSource>) -> bool {
        if let Some(source) = source {
            let sourced = self
                .sourced
                .entry(mana)
                .or_default()
                .entry(source)
                .or_default();
            let Some(mana) = sourced.checked_sub(1) else {
                return false;
            };

            *sourced = mana;
        } else {
            match mana {
                Mana::White => {
                    let Some(mana) = self.white_mana.checked_sub(1) else {
                        let mut sourced = self.max_sourced_mana(mana);
                        let Some(mana) = sourced.as_mut().and_then(|s| s.checked_sub(1)) else {
                            return false;
                        };
                        *sourced.unwrap() = mana;
                        return true;
                    };

                    self.white_mana = mana;
                }
                Mana::Blue => {
                    let Some(mana) = self.blue_mana.checked_sub(1) else {
                        let mut sourced = self.max_sourced_mana(mana);
                        let Some(mana) = sourced.as_mut().and_then(|s| s.checked_sub(1)) else {
                            return false;
                        };
                        *sourced.unwrap() = mana;
                        return true;
                    };

                    self.blue_mana = mana;
                }
                Mana::Black => {
                    let Some(mana) = self.black_mana.checked_sub(1) else {
                        let mut sourced = self.max_sourced_mana(mana);
                        let Some(mana) = sourced.as_mut().and_then(|s| s.checked_sub(1)) else {
                            return false;
                        };
                        *sourced.unwrap() = mana;
                        return true;
                    };

                    self.black_mana = mana;
                }
                Mana::Red => {
                    let Some(mana) = self.red_mana.checked_sub(1) else {
                        let mut sourced = self.max_sourced_mana(mana);
                        let Some(mana) = sourced.as_mut().and_then(|s| s.checked_sub(1)) else {
                            return false;
                        };
                        *sourced.unwrap() = mana;
                        return true;
                    };

                    self.red_mana = mana;
                }
                Mana::Green => {
                    let Some(mana) = self.green_mana.checked_sub(1) else {
                        let mut sourced = self.max_sourced_mana(mana);
                        let Some(mana) = sourced.as_mut().and_then(|s| s.checked_sub(1)) else {
                            return false;
                        };
                        *sourced.unwrap() = mana;
                        return true;
                    };

                    self.green_mana = mana;
                }
                Mana::Colorless => {
                    let Some(mana) = self.colorless_mana.checked_sub(1) else {
                        let mut sourced = self.max_sourced_mana(mana);
                        let Some(mana) = sourced.as_mut().and_then(|s| s.checked_sub(1)) else {
                            return false;
                        };
                        *sourced.unwrap() = mana;
                        return true;
                    };

                    self.colorless_mana = mana;
                }
            }
        }

        true
    }

    pub fn can_spend(&self, cost: ManaCost, source: Option<ManaSource>) -> bool {
        let mut mana_pool = self.clone();
        match cost {
            ManaCost::White => {
                if !mana_pool.spend(Mana::White, source) {
                    return false;
                }
            }
            ManaCost::Blue => {
                if !mana_pool.spend(Mana::Blue, source) {
                    return false;
                }
            }
            ManaCost::Black => {
                if !mana_pool.spend(Mana::Black, source) {
                    return false;
                }
            }
            ManaCost::Red => {
                if !mana_pool.spend(Mana::Red, source) {
                    return false;
                }
            }
            ManaCost::Green => {
                if !mana_pool.spend(Mana::Green, source) {
                    return false;
                }
            }
            ManaCost::Colorless => {
                if !mana_pool.spend(Mana::Colorless, source) {
                    return false;
                }
            }
            ManaCost::Generic(count) => {
                for _ in 0..count {
                    if let Some(max) = mana_pool.max() {
                        if !mana_pool.spend(max, source) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
            }
            ManaCost::X => {}
        }

        true
    }

    pub fn available_mana(&self) -> impl Iterator<Item = (usize, Mana, Option<ManaSource>)> + '_ {
        [
            (self.white_mana, Mana::White, None),
            (self.blue_mana, Mana::Blue, None),
            (self.black_mana, Mana::Black, None),
            (self.red_mana, Mana::Red, None),
            (self.green_mana, Mana::Green, None),
            (self.colorless_mana, Mana::Colorless, None),
        ]
        .into_iter()
        .chain(self.sourced.iter().flat_map(|(mana, sources)| {
            sources
                .iter()
                .sorted_by_key(|(_, count)| *count)
                .map(|(source, count)| (*count, *mana, Some(*source)))
        }))
        .filter(|(count, _, _)| *count > 0)
    }

    pub fn max(&self) -> Option<Mana> {
        self.available_mana()
            .max_by_key(|(count, _, _)| *count)
            .map(|(_, mana, _)| mana)
    }

    pub fn pools_display(&self) -> Vec<String> {
        let available = self.available_mana();

        let mut results = vec![];
        for (amount, symbol, source) in available {
            let mut result = String::default();
            symbol.push_mana_symbol(&mut result);
            if let Some(source) = source {
                result.push_str(&format!(" ({}): {}", source.as_ref(), amount));
            } else {
                result.push_str(&format!(": {}", amount));
            }
            results.push(result)
        }

        results
    }
}
