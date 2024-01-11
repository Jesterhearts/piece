use std::{collections::HashMap, sync::OnceLock};

use aho_corasick::AhoCorasick;
use convert_case::{Case, Casing};
use itertools::Itertools;
use protobuf::Enum;

use crate::{
    abilities::{ActivatedAbility, Enchant, GainManaAbility, StaticAbility, TriggeredAbility},
    cost::{AbilityCost, AdditionalCost, CastingCost, CostReducer},
    effects::{
        target_creature_explores::TargetCreatureExplores, AnyEffect, DynamicPowerToughness, Effect,
        Mode, ReplacementAbility, Token, TokenCreature,
    },
    protogen::targets::Restriction,
    protogen::{
        self,
        color::Color,
        cost::ManaCost,
        keywords::Keyword,
        types::{Subtype, Type},
    },
};

#[derive(Debug, Clone)]
pub(crate) enum BasePowerType {
    Static(i32),
    Dynamic(DynamicPowerToughness),
}

#[derive(Debug, Clone)]
pub(crate) enum BaseToughnessType {
    Static(i32),
    Dynamic(DynamicPowerToughness),
}

#[derive(Debug, Clone, Default)]
pub struct Card {
    pub name: String,
    pub types: Vec<protobuf::EnumOrUnknown<Type>>,
    pub subtypes: Vec<protobuf::EnumOrUnknown<Subtype>>,

    pub cost: CastingCost,
    pub(crate) reducer: Option<CostReducer>,
    pub(crate) cannot_be_countered: bool,

    pub(crate) colors: Vec<::protobuf::EnumOrUnknown<Color>>,

    pub oracle_text: String,

    pub(crate) enchant: Option<Enchant>,

    pub effects: Vec<AnyEffect>,
    pub(crate) modes: Vec<Mode>,

    pub(crate) etb_abilities: Vec<AnyEffect>,
    pub(crate) apply_individually: bool,

    pub(crate) static_abilities: Vec<StaticAbility>,

    pub(crate) activated_abilities: Vec<ActivatedAbility>,

    pub(crate) triggered_abilities: Vec<TriggeredAbility>,

    pub(crate) replacement_abilities: Vec<ReplacementAbility>,

    pub(crate) mana_abilities: Vec<GainManaAbility>,

    pub(crate) dynamic_power_toughness: Option<DynamicPowerToughness>,
    pub(crate) power: Option<usize>,
    pub(crate) toughness: Option<usize>,

    pub(crate) etb_tapped: bool,

    pub keywords: HashMap<i32, u32>,

    pub(crate) restrictions: Vec<Restriction>,

    pub(crate) back_face: Option<Box<Card>>,
}

impl Card {
    pub fn document(&self) -> String {
        let cost_text = self.cost.text();
        let keywords = self
            .keywords
            .keys()
            .map(|k| Keyword::from_i32(*k).unwrap())
            .collect_vec();

        [
            std::iter::once(self.name.as_str())
                .chain(std::iter::once(cost_text.as_str()))
                .chain(keywords.iter().map(|kw| kw.as_ref()))
                .chain(std::iter::once(self.oracle_text.as_str()))
                .chain(self.effects.iter().map(|e| e.oracle_text.as_str()))
                .chain(
                    self.modes
                        .iter()
                        .flat_map(|m| m.effects.iter().map(|e| e.oracle_text.as_str())),
                )
                .chain(self.etb_abilities.iter().map(|e| e.oracle_text.as_str()))
                .chain(
                    self.activated_abilities
                        .iter()
                        .map(|a| a.oracle_text.as_str()),
                )
                .filter(|t| !t.is_empty())
                .join("\n"),
            self.types
                .iter()
                .map(|t| t.enum_value().unwrap().as_ref().to_case(Case::Title))
                .join(" "),
            self.subtypes
                .iter()
                .map(|t| t.enum_value().unwrap().as_ref().to_case(Case::Title))
                .join(" "),
        ]
        .join("\n")
    }
}

impl TryFrom<&protogen::card::Card> for Card {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::card::Card) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name.clone(),
            types: value.typeline.types.clone(),
            subtypes: value.typeline.subtypes.clone(),
            cost: value.cost.get_or_default().try_into()?,
            reducer: value
                .cost_reducer
                .as_ref()
                .map_or(Ok(None), |reducer| reducer.try_into().map(Some))?,
            cannot_be_countered: value.cannot_be_countered,
            colors: value.colors.clone(),
            oracle_text: replace_symbols(&value.oracle_text),
            enchant: value
                .enchant
                .as_ref()
                .map_or(Ok(None), |enchant| enchant.try_into().map(Some))?,
            effects: value
                .effects
                .iter()
                .map(AnyEffect::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            modes: value
                .modes
                .iter()
                .map(Mode::try_from)
                .collect::<anyhow::Result<_>>()?,
            etb_abilities: value
                .etb_abilities
                .iter()
                .map(AnyEffect::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            apply_individually: value.apply_individually,
            static_abilities: value
                .static_abilities
                .iter()
                .map(StaticAbility::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            activated_abilities: value
                .activated_abilities
                .iter()
                .map(ActivatedAbility::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            triggered_abilities: value
                .triggered_abilities
                .iter()
                .map(TriggeredAbility::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            replacement_abilities: value
                .replacement_abilities
                .iter()
                .map(ReplacementAbility::try_from)
                .collect::<anyhow::Result<_>>()?,
            mana_abilities: value
                .mana_abilities
                .iter()
                .map(GainManaAbility::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            etb_tapped: value.etb_tapped,
            dynamic_power_toughness: value
                .dynamic_power_toughness
                .as_ref()
                .map_or(Ok(None), |dynamic| dynamic.try_into().map(Some))?,
            power: value
                .power
                .map_or::<anyhow::Result<Option<usize>>, _>(Ok(None), |v| {
                    Ok(usize::try_from(v).map(Some)?)
                })?,
            toughness: value
                .toughness
                .map_or::<anyhow::Result<Option<usize>>, _>(Ok(None), |v| {
                    Ok(usize::try_from(v).map(Some)?)
                })?,
            keywords: value.keywords.clone(),
            restrictions: value.restrictions.clone(),
            back_face: value.back_face.as_ref().map_or(Ok(None), |back| {
                Card::try_from(back).map(|card| Some(Box::new(card)))
            })?,
        })
    }
}

impl From<Token> for Card {
    fn from(value: Token) -> Self {
        match value {
            Token::Creature(token) => {
                let TokenCreature {
                    name,
                    types,
                    subtypes,
                    colors,
                    keywords,
                    dynamic_power_toughness,
                    power,
                    toughness,
                } = *token;

                Self {
                    name,
                    types,
                    subtypes,
                    colors,
                    power: Some(power),
                    toughness: Some(toughness),
                    keywords,
                    dynamic_power_toughness,
                    ..Default::default()
                }
            }
            Token::Map => Self {
                name: "Map".to_string(),
                types: vec![Type::ARTIFACT.into()],
                activated_abilities: vec![ActivatedAbility {
                    cost: AbilityCost {
                        mana_cost: vec![ManaCost::GENERIC.into()],
                        tap: true,
                        additional_cost: vec![AdditionalCost::SacrificeSource],
                        restrictions: vec![],
                    },
                    effects: vec![AnyEffect {
                        effect: Effect::from(TargetCreatureExplores),
                        oracle_text: String::default(),
                    }],
                    apply_to_self: false,
                    oracle_text: "Target creature you control explores. Activate only as sorcery"
                        .to_string(),
                    sorcery_speed: true,
                    craft: false,
                }],
                ..Default::default()
            },
        }
    }
}

pub fn replace_symbols(result: &str) -> String {
    #[rustfmt::skip]
    const PATTERNS: &[&str] = &[
        "{W}",
        "{U}",
        "{B}",
        "{R}",
        "{G}",
        "{C}",
        "{0}",
        "{1}",
        "{2}",
        "{3}",
        "{4}",
        "{5}",
        "{6}",
        "{7}",
        "{8}",
        "{9}",
        "{10}",
        "{11}",
        "{12}",
        "{13}",
        "{14}",
        "{15}",
        "{16}",
        "{17}",
        "{18}",
        "{19}",
        "{20}",
        "{X}",
        "{T}",
        "{Q}",
    ];

    #[rustfmt::skip]
    const REPLACE_WITH: &[&str] = &[
        "\u{e600}",
        "\u{e601}",
        "\u{e602}",
        "\u{e603}",
        "\u{e604}",
        "\u{e904}",
        "\u{e605}",
        "\u{e606}",
        "\u{e607}",
        "\u{e608}",
        "\u{e609}",
        "\u{e60a}",
        "\u{e60b}",
        "\u{e60c}",
        "\u{e60d}",
        "\u{e60e}",
        "\u{e60f}",
        "\u{e610}",
        "\u{e611}",
        "\u{e612}",
        "\u{e613}",
        "\u{e614}",
        "\u{e62a}",
        "\u{e62b}",
        "\u{e62c}",
        "\u{e62d}",
        "\u{e62e}",
        "\u{e615}",
        "\u{e61a}",
        "\u{e61b}",
    ];

    static AC: OnceLock<AhoCorasick> = OnceLock::new();
    AC.get_or_init(|| AhoCorasick::new(PATTERNS).unwrap())
        .replace_all(result, REPLACE_WITH)
}
