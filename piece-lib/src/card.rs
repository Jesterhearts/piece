use std::sync::OnceLock;

use aho_corasick::AhoCorasick;
use itertools::Itertools;

use crate::{
    protogen::effects::Effect,
    protogen::{
        card::Card,
        cost::{additional_cost, AbilityCost, AdditionalCost, ManaCost},
        effects::{
            create_token::{self, Token},
            effect, ActivatedAbility, DynamicPowerToughness, TargetCreatureExplores,
        },
        types::{Subtype, Type, Typeline},
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

impl Card {
    pub fn document(&self) -> String {
        let cost_text = self.cost.text();

        std::iter::once(self.name.as_str())
            .chain(std::iter::once(cost_text.as_str()))
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
            .join("\n")
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

                Self {
                    name,
                    typeline,
                    colors,
                    power: Some(power),
                    toughness: Some(toughness),
                    keywords,
                    dynamic_power_toughness,
                    ..Default::default()
                }
            }
            Token::Map(_) => Self {
                name: "Map".to_string(),
                typeline: protobuf::MessageField::some(Typeline {
                    types: vec![Type::ARTIFACT.into()],
                    subtypes: vec![Subtype::MAP.into()],
                    ..Default::default()
                }),
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
                    oracle_text: "{1}, {T}, Sacrifice this artifact: Target creature you control explores. Activate only as sorcery"
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

#[rustfmt::skip]
const EXPANDED_SYMBOLS: &[&str] = &[
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
const EMOJI_SYMBOLS: &[&str] = &[
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

pub fn replace_expanded_symbols(result: &str) -> String {
    static AC: OnceLock<AhoCorasick> = OnceLock::new();
    AC.get_or_init(|| AhoCorasick::new(EXPANDED_SYMBOLS).unwrap())
        .replace_all(result, EMOJI_SYMBOLS)
}

pub fn replace_emoji_symbols(result: &str) -> String {
    static AC: OnceLock<AhoCorasick> = OnceLock::new();
    AC.get_or_init(|| AhoCorasick::new(EMOJI_SYMBOLS).unwrap())
        .replace_all(result, EXPANDED_SYMBOLS)
}
