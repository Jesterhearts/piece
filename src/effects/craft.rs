use std::{collections::HashSet, sync::Arc};

use anyhow::anyhow;
use bevy_ecs::component::Component;
use indexmap::IndexSet;
use itertools::Itertools;

use crate::{
    battlefield::{ActionResult, ChooseTargets, TargetSource},
    card::Color,
    effects::{Effect, EffectBehaviors},
    in_play::{self, target_from_location, CardId, Database, InGraveyard, OnBattlefield},
    protogen,
    stack::ActiveTarget,
    types::{Subtype, Type},
};

#[derive(Debug, Clone, Component)]
pub enum CraftTarget {
    One {
        types: IndexSet<Type>,
        subtypes: IndexSet<Subtype>,
    },
    XOrMore {
        minimum: usize,
        types: IndexSet<Type>,
        subtypes: IndexSet<Subtype>,
        colors: HashSet<Color>,
    },
    SharingCardType {
        count: usize,
    },
    OneOfEach {
        subtypes: IndexSet<Subtype>,
    },
}

impl CraftTarget {
    pub(crate) fn needs_targets(&self) -> usize {
        match self {
            CraftTarget::One { .. } => 1,
            CraftTarget::XOrMore { minimum, .. } => *minimum,
            CraftTarget::SharingCardType { count } => *count,
            CraftTarget::OneOfEach { subtypes } => subtypes.len(),
        }
    }

    pub(crate) fn targets(
        &self,
        this: CardId,
        db: &mut Database,
        already_chosen: &HashSet<ActiveTarget>,
    ) -> Vec<ActiveTarget> {
        let candidates = in_play::cards::<OnBattlefield>(db)
            .into_iter()
            .chain(in_play::cards::<InGraveyard>(db))
            .filter(|card| *card != this);
        let mut targets = vec![];

        match self {
            CraftTarget::One { types, subtypes } => {
                for card in candidates
                    .filter(|card| card.types_intersect(db, types))
                    .filter(|card| card.subtypes_intersect(db, subtypes))
                    .collect_vec()
                {
                    targets.push(target_from_location(db, card));
                }
            }
            CraftTarget::XOrMore {
                types,
                subtypes,
                colors,
                ..
            } => {
                for card in candidates
                    .filter(|card| card.types_intersect(db, types))
                    .filter(|card| card.subtypes_intersect(db, subtypes))
                    .filter(|card| !card.colors(db).is_disjoint(colors))
                    .collect_vec()
                {
                    targets.push(target_from_location(db, card));
                }
            }
            CraftTarget::SharingCardType { .. } => {
                let card_types = already_chosen
                    .iter()
                    .map(|chosen| chosen.id().unwrap())
                    .map(|chosen| chosen.types(db))
                    .collect_vec();
                for card in candidates
                    .filter(|candidate| {
                        card_types
                            .iter()
                            .all(|types| candidate.types_intersect(db, types))
                    })
                    .collect_vec()
                {
                    targets.push(target_from_location(db, card));
                }
            }
            CraftTarget::OneOfEach { subtypes } => {
                let already_chosen = already_chosen
                    .iter()
                    .map(|chosen| chosen.id().unwrap())
                    .flat_map(|chosen| chosen.subtypes(db).into_iter())
                    .collect::<HashSet<_>>();

                for card in candidates
                    .filter(|card| {
                        card.subtypes_intersect(db, subtypes)
                            && card
                                .subtypes(db)
                                .intersection(subtypes)
                                .copied()
                                .collect::<HashSet<_>>()
                                .is_disjoint(&already_chosen)
                    })
                    .collect_vec()
                {
                    targets.push(target_from_location(db, card));
                }
            }
        }

        targets
    }
}

impl TryFrom<&protogen::effects::craft::Source> for CraftTarget {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::craft::Source) -> Result<Self, Self::Error> {
        match value {
            protogen::effects::craft::Source::One(one) => Ok(Self::One {
                types: one
                    .types
                    .iter()
                    .map(Type::try_from)
                    .collect::<anyhow::Result<_>>()?,
                subtypes: one
                    .subtypes
                    .iter()
                    .map(Subtype::try_from)
                    .collect::<anyhow::Result<_>>()?,
            }),
            protogen::effects::craft::Source::XOrMore(xormore) => Ok(Self::XOrMore {
                minimum: usize::try_from(xormore.minimum)?,
                types: xormore
                    .types
                    .iter()
                    .map(Type::try_from)
                    .collect::<anyhow::Result<_>>()?,
                subtypes: xormore
                    .subtypes
                    .iter()
                    .map(Subtype::try_from)
                    .collect::<anyhow::Result<_>>()?,
                colors: xormore
                    .colors
                    .iter()
                    .map(Color::try_from)
                    .collect::<anyhow::Result<_>>()?,
            }),
            protogen::effects::craft::Source::SharingCardType(sharing) => {
                Ok(Self::SharingCardType {
                    count: usize::try_from(sharing.count)?,
                })
            }
            protogen::effects::craft::Source::OneOfEach(oneofeach) => Ok(Self::OneOfEach {
                subtypes: oneofeach
                    .subtypes
                    .iter()
                    .map(Subtype::try_from)
                    .collect::<anyhow::Result<_>>()?,
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Craft {
    pub target: CraftTarget,
}

impl TryFrom<&protogen::effects::Craft> for Craft {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::effects::Craft) -> Result<Self, Self::Error> {
        Ok(Self {
            target: value
                .source
                .as_ref()
                .ok_or_else(|| anyhow!("Expected craft to have a target set"))
                .and_then(CraftTarget::try_from)?,
        })
    }
}

impl EffectBehaviors for Craft {
    fn needs_targets(&self) -> usize {
        self.target.needs_targets()
    }

    fn wants_targets(&self) -> usize {
        self.target.needs_targets()
    }

    fn valid_targets(
        &self,
        db: &mut Database,
        source: CardId,
        _controller: crate::player::Controller,
        already_chosen: &HashSet<ActiveTarget>,
    ) -> Vec<ActiveTarget> {
        self.target.targets(source, db, already_chosen)
    }

    fn push_pending_behavior(
        &self,
        db: &mut Database,
        source: CardId,
        controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        let valid_targets =
            self.valid_targets(db, source, controller, results.all_currently_targeted());

        results.push_choose_targets(ChooseTargets::new(
            TargetSource::Effect(Effect(Arc::new(self.clone()) as Arc<_>)),
            valid_targets,
        ));
    }

    fn push_behavior_with_targets(
        &self,
        _db: &mut Database,
        targets: Vec<ActiveTarget>,
        _apply_to_self: bool,
        source: CardId,
        _controller: crate::player::Controller,
        results: &mut crate::battlefield::PendingResults,
    ) {
        results.push_settled(ActionResult::Craft {
            transforming: source,
            targets,
        })
    }
}
