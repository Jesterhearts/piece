use std::{collections::HashSet, str::FromStr};

use aho_corasick::AhoCorasick;
use anyhow::{anyhow, Context};
use counter::Counter;
use indexmap::IndexSet;
use itertools::Itertools;

use crate::{
    abilities::{ActivatedAbility, Enchant, GainManaAbility, StaticAbility, TriggeredAbility},
    cost::{AbilityCost, AdditionalCost, CastingCost, CostReducer},
    effects::{
        target_creature_explores::TargetCreatureExplores, AnyEffect, DynamicPowerToughness, Effect,
        Mode, ReplacementAbility, Token, TokenCreature,
    },
    mana::ManaCost,
    protogen,
    targets::Restriction,
    types::{parse_typeline, Subtype, Type},
};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter, strum::EnumString, strum::AsRefStr,
)]
#[strum(ascii_case_insensitive)]
pub enum Keyword {
    Absorb,
    Affinity,
    Afflict,
    Afterlife,
    Aftermath,
    Amplify,
    Annihilator,
    Ascend,
    Assist,
    AuraSwap,
    Awaken,
    Backup,
    Banding,
    Bargain,
    BattleCry,
    Bestow,
    Blitz,
    Bloodthirst,
    Boast,
    Bushido,
    Buyback,
    Cascade,
    Casualty,
    Champion,
    Changeling,
    Cipher,
    Cleave,
    Companion,
    Compleated,
    Conspire,
    Convoke,
    Craft,
    Crew,
    CumulativeUpkeep,
    Cycling,
    Dash,
    DayboundandNightbound,
    Deathtouch,
    Decayed,
    Defender,
    Delve,
    Demonstrate,
    Dethrone,
    Devoid,
    Devour,
    Disturb,
    DoubleStrike,
    Dredge,
    Echo,
    Embalm,
    Emerge,
    Enchant,
    Encore,
    Enlist,
    Entwine,
    Epic,
    Equip,
    Escalate,
    Escape,
    Eternalize,
    Evoke,
    Evolve,
    Exalted,
    Exploit,
    Extort,
    Fabricate,
    Fading,
    Fear,
    FirstStrike,
    Flanking,
    Flash,
    Flashback,
    Flying,
    Forecast,
    Foretell,
    ForMirrodin,
    Fortify,
    Frenzy,
    Fuse,
    Graft,
    Gravestorm,
    Haste,
    Haunt,
    Hexproof,
    HiddenAgenda,
    Hideaway,
    Horsemanship,
    Improvise,
    Indestructible,
    Infect,
    Ingest,
    Intimidate,
    JumpStart,
    Kicker,
    Landwalk,
    LevelUp,
    Lifelink,
    LivingMetal,
    LivingWeapon,
    Madness,
    Melee,
    Menace,
    Mentor,
    Miracle,
    Modular,
    MoreThanMeetstheEye,
    Morph,
    Mutate,
    Myriad,
    Ninjutsu,
    Offering,
    Outlast,
    Overload,
    Partner,
    Persist,
    Phasing,
    Poisonous,
    Protection,
    Prototype,
    Provoke,
    Prowess,
    Prowl,
    Rampage,
    Ravenous,
    Reach,
    ReadAhead,
    Rebound,
    Reconfigure,
    Recover,
    Reinforce,
    Renown,
    Replicate,
    Retrace,
    Riot,
    Ripple,
    Scavenge,
    Shadow,
    Shroud,
    Skulk,
    Soulbond,
    Soulshift,
    SpaceSculptor,
    Spectacle,
    Splice,
    SplitSecond,
    Squad,
    Storm,
    Sunburst,
    Surge,
    Suspend,
    TotemArmor,
    Toxic,
    Training,
    Trample,
    Transfigure,
    Transmute,
    Tribute,
    Undaunted,
    Undying,
    Unearth,
    Unleash,
    Vanishing,
    Vigilance,
    Visit,
    Ward,
    Wither,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::AsRefStr)]
pub enum Color {
    White,
    Blue,
    Black,
    Red,
    Green,
    Colorless,
}

impl TryFrom<&protogen::color::Color> for Color {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::color::Color) -> Result<Self, Self::Error> {
        value
            .color
            .as_ref()
            .ok_or_else(|| anyhow!("Expected color to have a color set"))
            .map(Self::from)
    }
}

impl From<&protogen::color::color::Color> for Color {
    fn from(value: &protogen::color::color::Color) -> Self {
        match value {
            protogen::color::color::Color::White(_) => Self::White,
            protogen::color::color::Color::Blue(_) => Self::Blue,
            protogen::color::color::Color::Black(_) => Self::Black,
            protogen::color::color::Color::Red(_) => Self::Red,
            protogen::color::color::Color::Green(_) => Self::Green,
            protogen::color::color::Color::Colorless(_) => Self::Colorless,
        }
    }
}

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
    pub types: IndexSet<Type>,
    pub subtypes: IndexSet<Subtype>,

    pub cost: CastingCost,
    pub(crate) reducer: Option<CostReducer>,
    pub(crate) cannot_be_countered: bool,

    pub(crate) colors: HashSet<Color>,

    pub(crate) oracle_text: String,

    pub full_text: String,

    pub(crate) enchant: Option<Enchant>,

    pub(crate) effects: Vec<AnyEffect>,
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

    pub keywords: Counter<Keyword>,

    pub(crate) restrictions: Vec<Restriction>,

    pub(crate) back_face: Option<Box<Card>>,
}

impl Card {
    fn compute_full_text(&mut self) {
        self.full_text = std::iter::once(self.oracle_text.as_str())
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
            .join("\n");
    }
}

impl TryFrom<&protogen::card::Card> for Card {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::card::Card) -> Result<Self, Self::Error> {
        let (types, subtypes) = parse_typeline(&value.typeline)?;

        let mut this = Self {
            name: value.name.clone(),
            types,
            subtypes,
            cost: value.cost.get_or_default().try_into()?,
            reducer: value
                .cost_reducer
                .as_ref()
                .map_or(Ok(None), |reducer| reducer.try_into().map(Some))?,
            cannot_be_countered: value.cannot_be_countered,
            colors: value
                .colors
                .iter()
                .map(Color::try_from)
                .collect::<anyhow::Result<HashSet<_>>>()?,
            oracle_text: replace_symbols(&value.oracle_text),
            full_text: Default::default(),
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
            keywords: value
                .keywords
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|s| Keyword::from_str(s.trim()).with_context(|| anyhow!("Parsing {}", s)))
                .collect::<anyhow::Result<_>>()?,
            restrictions: value
                .restrictions
                .iter()
                .map(Restriction::try_from)
                .collect::<anyhow::Result<_>>()?,
            back_face: value.back_face.as_ref().map_or(Ok(None), |back| {
                Card::try_from(back).map(|card| Some(Box::new(card)))
            })?,
        };

        this.compute_full_text();

        Ok(this)
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
                types: IndexSet::from([Type::Artifact]),
                activated_abilities: vec![ActivatedAbility {
                    cost: AbilityCost {
                        mana_cost: vec![ManaCost::Generic(1)],
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
    let patterns = &[
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
    let replace_with = &[
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

    let ac = AhoCorasick::new(patterns).unwrap();
    ac.replace_all(result, replace_with)
}
