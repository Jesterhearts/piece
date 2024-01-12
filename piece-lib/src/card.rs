use std::{collections::HashMap, sync::OnceLock};

use aho_corasick::AhoCorasick;
use convert_case::{Case, Casing};
use itertools::Itertools;
use protobuf::Enum;

use crate::{
    abilities::Enchant,
    protogen::{
        self,
        abilities::TriggeredAbility,
        color::Color,
        cost::{additional_cost, AbilityCost, AdditionalCost, CastingCost, CostReducer, ManaCost},
        effects::{
            create_token::{self, Token},
            effect, ActivatedAbility, DynamicPowerToughness, GainManaAbility, StaticAbility,
            TargetCreatureExplores,
        },
        keywords::Keyword,
        types::{Subtype, Type},
    },
    protogen::{
        effects::{Effect, Mode, ReplacementEffect},
        targets::Restriction,
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

    pub effects: Vec<Effect>,
    pub(crate) modes: Vec<Mode>,

    pub(crate) etb_abilities: Vec<Effect>,
    pub(crate) apply_individually: bool,

    pub(crate) static_abilities: Vec<StaticAbility>,

    pub(crate) activated_abilities: Vec<ActivatedAbility>,

    pub(crate) triggered_abilities: Vec<TriggeredAbility>,

    pub(crate) replacement_abilities: Vec<ReplacementEffect>,

    pub(crate) mana_abilities: Vec<GainManaAbility>,

    pub(crate) dynamic_power_toughness: Option<DynamicPowerToughness>,
    pub(crate) power: Option<i32>,
    pub(crate) toughness: Option<i32>,

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
            cost: value.cost.get_or_default().clone(),
            reducer: value.cost_reducer.as_ref().cloned(),
            cannot_be_countered: value.cannot_be_countered,
            colors: value.colors.clone(),
            oracle_text: replace_symbols(&value.oracle_text),
            enchant: value
                .enchant
                .as_ref()
                .map_or(Ok(None), |enchant| enchant.try_into().map(Some))?,
            effects: value.effects.clone(),
            modes: value.modes.clone(),
            etb_abilities: value.etb_abilities.clone(),
            apply_individually: value.apply_individually,
            static_abilities: value.static_abilities.clone(),
            activated_abilities: value.activated_abilities.clone(),
            triggered_abilities: value.triggered_abilities.clone(),
            replacement_abilities: value.replacement_abilities.clone(),
            mana_abilities: value.mana_abilities.clone(),
            etb_tapped: value.etb_tapped,
            dynamic_power_toughness: value.dynamic_power_toughness.as_ref().cloned(),
            power: value.power,
            toughness: value.toughness,
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
                let create_token::Creature {
                    name,
                    typeline,
                    colors,
                    keywords,
                    dynamic_power_toughness,
                    power,
                    toughness,
                    ..
                } = token;

                let typeline = typeline.unwrap();
                Self {
                    name,
                    types: typeline.types,
                    subtypes: typeline.subtypes,
                    colors,
                    power: Some(power),
                    toughness: Some(toughness),
                    keywords,
                    dynamic_power_toughness: dynamic_power_toughness.into_option(),
                    ..Default::default()
                }
            }
            Token::Map(_) => Self {
                name: "Map".to_string(),
                types: vec![Type::ARTIFACT.into()],
                activated_abilities: vec![ActivatedAbility {
                    cost: protobuf::MessageField::some(AbilityCost {
                        mana_cost: vec![ManaCost::GENERIC.into()],
                        tap: true,
                        additional_costs: vec![AdditionalCost {
                            cost: Some(additional_cost::Cost::SacrificeSource(Default::default())),
                            ..Default::default()
                        }],
                        restrictions: vec![],
                        ..Default::default()
                    }),
                    effects: vec![Effect {
                        effect: Some(effect::Effect::from(TargetCreatureExplores::default())),
                        oracle_text: String::default(),
                        ..Default::default()
                    }],
                    apply_to_self: false,
                    oracle_text: "Target creature you control explores. Activate only as sorcery"
                        .to_string(),
                    sorcery_speed: true,
                    craft: false,
                    ..Default::default()
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
