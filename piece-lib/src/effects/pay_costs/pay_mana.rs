use indexmap::IndexMap;
use itertools::Itertools;
use protobuf::Enum;

use crate::{
    effects::{
        EffectBehaviors, EffectBundle, Options, PendingEffects, SelectedStack, SelectionResult,
    },
    in_play::{CardId, Database},
    protogen::{
        cost::ManaCost,
        effects::{pay_cost::PayMana, Effect, SpendMana},
        mana::{Mana, ManaSource},
    },
    stack::Selected,
};

impl EffectBehaviors for PayMana {
    fn options(
        &self,
        db: &Database,
        source: Option<CardId>,
        _already_selected: &[Selected],
        _modes: &[usize],
    ) -> Options {
        let (mana, sources) = self.paying();
        let pool_post_paid = db.all_players[db[source.unwrap()].controller].pool_post_pay(
            db,
            &mana.iter().map(|e| e.enum_value().unwrap()).collect_vec(),
            &sources
                .iter()
                .map(|e| e.enum_value().unwrap())
                .collect_vec(),
            self.reason.reason.as_ref().unwrap(),
        );
        if pool_post_paid.is_none()
            || pool_post_paid
                .as_ref()
                .unwrap()
                .max(db, self.reason.reason.as_ref().unwrap())
                .is_none()
        {
            return Options::OptionalList(vec![]);
        }

        let pool_post_paid = pool_post_paid.unwrap();
        let display = pool_post_paid
            .available_pool_display()
            .into_iter()
            .enumerate()
            .collect_vec();

        match self.first_unpaid_x_always_unpaid() {
            Some(ManaCost::GENERIC | ManaCost::X) => Options::ListWithDefault(display),
            Some(ManaCost::TWO_X) => {
                if self
                    .paid
                    .get(&ManaCost::TWO_X.value())
                    .iter()
                    .flat_map(|m| m.mana_to_source.values())
                    .flat_map(|m| m.source_to_count.values())
                    .sum::<u32>()
                    % 2
                    == 0
                {
                    Options::ListWithDefault(display)
                } else {
                    Options::MandatoryList(display)
                }
            }
            _ => Options::ListWithDefault(vec![]),
        }
    }

    fn select(
        &mut self,
        db: &mut Database,
        source_card: Option<CardId>,
        option: Option<usize>,
        _selected: &mut SelectedStack,
        _modes: &mut Vec<usize>,
    ) -> SelectionResult {
        if option.is_none() {
            if self
                .paid
                .get(&ManaCost::TWO_X.value())
                .iter()
                .flat_map(|m| m.mana_to_source.values())
                .flat_map(|m| m.source_to_count.values())
                .sum::<u32>()
                % 2
                == 0
            {
                return SelectionResult::PendingChoice;
            }

            let (mana, sources) = self.paying();
            let mut pool_post_pay = db.all_players[db[source_card.unwrap()].controller]
                .pool_post_pay(
                    db,
                    &mana.iter().map(|e| e.enum_value().unwrap()).collect_vec(),
                    &sources
                        .iter()
                        .map(|e| e.enum_value().unwrap())
                        .collect_vec(),
                    self.reason.reason.as_ref().unwrap(),
                )
                .unwrap();
            let Some(first_unpaid) = self.first_unpaid() else {
                return SelectionResult::Complete;
            };

            if pool_post_pay.can_spend(
                db,
                first_unpaid,
                ManaSource::ANY,
                self.reason.reason.as_ref().unwrap(),
            ) {
                let mana = match first_unpaid {
                    ManaCost::WHITE => Mana::WHITE,
                    ManaCost::BLUE => Mana::BLUE,
                    ManaCost::BLACK => Mana::BLACK,
                    ManaCost::RED => Mana::RED,
                    ManaCost::GREEN => Mana::GREEN,
                    ManaCost::COLORLESS => Mana::COLORLESS,
                    ManaCost::GENERIC => {
                        while matches!(self.first_unpaid(), Some(ManaCost::GENERIC))
                            && pool_post_pay.can_spend(
                                db,
                                ManaCost::GENERIC,
                                ManaSource::ANY,
                                self.reason.reason.as_ref().unwrap(),
                            )
                        {
                            let max = pool_post_pay
                                .max(db, self.reason.reason.as_ref().unwrap())
                                .unwrap();
                            let (_, source) = pool_post_pay.spend(
                                db,
                                max,
                                ManaSource::ANY,
                                self.reason.reason.as_ref().unwrap(),
                            );
                            *self
                                .paid
                                .entry(first_unpaid.value())
                                .or_default()
                                .mana_to_source
                                .entry(max.value())
                                .or_default()
                                .source_to_count
                                .entry(source.value())
                                .or_default() += 1;
                        }

                        return if matches!(
                            self.first_unpaid_x_always_unpaid(),
                            Some(ManaCost::X | ManaCost::TWO_X)
                        ) {
                            SelectionResult::PendingChoice
                        } else {
                            SelectionResult::Complete
                        };
                    }
                    ManaCost::X => unreachable!(),
                    ManaCost::TWO_X => unreachable!(),
                };

                let (_, source) = pool_post_pay.spend(
                    db,
                    mana,
                    ManaSource::ANY,
                    self.reason.reason.as_ref().unwrap(),
                );
                *self
                    .paid
                    .entry(first_unpaid.value())
                    .or_default()
                    .mana_to_source
                    .entry(mana.value())
                    .or_default()
                    .source_to_count
                    .entry(source.value())
                    .or_default() += 1;

                return if matches!(
                    self.first_unpaid_x_always_unpaid(),
                    Some(ManaCost::X | ManaCost::TWO_X)
                ) {
                    SelectionResult::PendingChoice
                } else {
                    SelectionResult::Complete
                };
            } else {
                return SelectionResult::PendingChoice;
            }
        }

        let (mana, sources) = self.paying();
        if let Some((_, mana, source, _)) = db.all_players[db[source_card.unwrap()].controller]
            .pool_post_pay(
                db,
                &mana.iter().map(|e| e.enum_value().unwrap()).collect_vec(),
                &sources
                    .iter()
                    .map(|e| e.enum_value().unwrap())
                    .collect_vec(),
                self.reason.reason.as_ref().unwrap(),
            )
            .unwrap()
            .available_mana()
            .nth(option.unwrap())
        {
            let cost = self.first_unpaid_x_always_unpaid().unwrap();
            *self
                .paid
                .entry(cost.value())
                .or_default()
                .mana_to_source
                .entry(mana.value())
                .or_default()
                .source_to_count
                .entry(source.value())
                .or_default() += 1;

            let (mana, sources) = self.paying();
            if db.all_players[db[source_card.unwrap()].controller].can_spend_mana(
                db,
                &mana.iter().map(|e| e.enum_value().unwrap()).collect_vec(),
                &sources
                    .iter()
                    .map(|e| e.enum_value().unwrap())
                    .collect_vec(),
                self.reason.reason.as_ref().unwrap(),
            ) {
                if self.first_unpaid_x_always_unpaid().is_none() {
                    SelectionResult::Complete
                } else {
                    SelectionResult::PendingChoice
                }
            } else {
                SelectionResult::PendingChoice
            }
        } else {
            SelectionResult::PendingChoice
        }
    }

    fn apply(
        &mut self,
        db: &mut Database,
        effects: &mut PendingEffects,
        source: Option<CardId>,
        _selected: &mut SelectedStack,
        _modes: &[usize],
        _skip_replacement: bool,
    ) {
        db[source.unwrap()].x_is = self.x_paid() as usize;

        let (mana_paid, mana_sources) = self.paying();

        effects.push_back(EffectBundle {
            effects: vec![Effect {
                effect: Some(
                    SpendMana {
                        mana: mana_paid,
                        mana_sources,
                        reason: self.reason.clone(),
                        ..Default::default()
                    }
                    .into(),
                ),
                ..Default::default()
            }],
            source,
            ..Default::default()
        });
    }
}

impl PayMana {
    pub(crate) fn first_unpaid_x_always_unpaid(&self) -> Option<ManaCost> {
        let paying = self
            .paying
            .iter()
            .map(|pay| pay.enum_value().unwrap())
            .fold(IndexMap::<_, u32>::default(), |mut map, e| {
                *map.entry(e).or_default() += 1;
                map
            });

        paying
            .into_iter()
            .find(|(paying, required)| {
                let required = match paying {
                    ManaCost::X => u32::MAX,
                    ManaCost::TWO_X => u32::MAX,
                    _ => *required,
                };

                self.paid
                    .get(&paying.value())
                    .map(|paid| {
                        let paid = paid
                            .mana_to_source
                            .values()
                            .flat_map(|sourced| sourced.source_to_count.values())
                            .sum::<u32>();
                        paid < required
                    })
                    .unwrap_or(true)
            })
            .map(|(paying, _)| paying)
    }

    pub(crate) fn first_unpaid(&self) -> Option<ManaCost> {
        self.first_unpaid_x_always_unpaid()
            .filter(|unpaid| !matches!(unpaid, ManaCost::X | ManaCost::TWO_X))
    }

    fn x_paid(&mut self) -> u32 {
        u32::max(
            self.paid
                .entry(ManaCost::X.value())
                .or_default()
                .mana_to_source
                .values()
                .flat_map(|m| m.source_to_count.values())
                .sum::<u32>(),
            self.paid
                .entry(ManaCost::TWO_X.value())
                .or_default()
                .mana_to_source
                .values()
                .flat_map(|m| m.source_to_count.values())
                .sum::<u32>(),
        )
    }

    fn paying(
        &self,
    ) -> (
        Vec<protobuf::EnumOrUnknown<Mana>>,
        Vec<protobuf::EnumOrUnknown<ManaSource>>,
    ) {
        let mut mana_paid = vec![];
        let mut mana_sources = vec![];

        for paid in self.paid.values() {
            for (mana, source) in paid.mana_to_source.iter() {
                for (source, count) in source.source_to_count.iter() {
                    for _ in 0..*count {
                        mana_paid.push(protobuf::EnumOrUnknown::from_i32(*mana));
                        mana_sources.push(protobuf::EnumOrUnknown::from_i32(*source))
                    }
                }
            }
        }
        (mana_paid, mana_sources)
    }
}