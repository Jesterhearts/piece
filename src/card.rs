use std::{collections::HashSet, str::FromStr};

use anyhow::{anyhow, Context};
use bevy_ecs::component::Component;
use counter::Counter;
use derive_more::{Deref, DerefMut};
use indexmap::IndexSet;
use itertools::Itertools;
use strum::IntoEnumIterator;

use crate::{
    abilities::{ActivatedAbility, Enchant, GainManaAbility, StaticAbility, TriggeredAbility},
    cost::{AbilityCost, AdditionalCost, CastingCost, CostReducer, Ward},
    effects::{
        target_creature_explores::TargetCreatureExplores, AnyEffect, DynamicPowerToughness, Effect,
        Mode, ReplacementEffect, Token, TokenCreature,
    },
    in_play::{AbilityId, CardId, TriggerId},
    mana::ManaCost,
    newtype_enum::newtype_enum,
    protogen,
    targets::Restriction,
    types::{parse_typeline, Subtype, Type},
};

#[derive(Debug, Clone, Component)]
pub(crate) struct BackFace(pub(crate) CardId);

#[derive(Debug, Clone, Component)]
pub(crate) struct FrontFace(pub(crate) CardId);

#[derive(Debug, Clone, Deref, DerefMut, Component)]
pub(crate) struct Keywords(pub(crate) Counter<Keyword>);

#[derive(Debug, Clone, Deref, DerefMut, Component)]
pub(crate) struct ModifiedKeywords(pub(crate) Counter<Keyword>);

#[rustfmt::skip]
newtype_enum!{
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bevy_ecs::component::Component)]
#[derive(strum::EnumIter, strum::EnumString, strum::AsRefStr)]
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
}

impl Keyword {
    pub(crate) fn all() -> HashSet<Keyword> {
        Keyword::iter().collect()
    }
}

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct PaidX(pub(crate) usize);

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct ApplyIndividually;

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct Revealed;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub(crate) struct CannotBeCountered;

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut)]
pub(crate) struct Colors(pub(crate) HashSet<Color>);

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut)]
pub(crate) struct ModifiedColors(pub(crate) HashSet<Color>);

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut)]
pub(crate) struct AddColors(pub(crate) HashSet<Color>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub(crate) struct RemoveAllColors;

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

#[derive(Debug, Clone, Component)]
pub(crate) enum StaticAbilityModifier {
    RemoveAll,
    AddAll(Vec<StaticAbility>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub(crate) enum ActivatedAbilityModifier {
    RemoveAll,
    Add(AbilityId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
#[allow(unused)]
pub(crate) enum TriggeredAbilityModifier {
    RemoveAll,
    Add(TriggerId),
}

#[derive(Debug, Clone, PartialEq, Eq, Component)]
#[allow(unused)]
pub(crate) enum EtbAbilityModifier {
    RemoveAll,
    Add(AbilityId),
}

#[derive(Debug, Clone, Component)]
pub(crate) enum BasePowerType {
    Static(i32),
    Dynamic(DynamicPowerToughness),
}

#[derive(Debug, Clone, Component)]
pub(crate) enum BaseToughnessType {
    Static(i32),
    Dynamic(DynamicPowerToughness),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut)]
pub(crate) struct Name(pub(crate) String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut)]
pub(crate) struct OracleText(pub(crate) String);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut, Default,
)]
pub(crate) struct MarkedDamage(pub(crate) i32);

#[derive(Debug, Clone, Component, Deref, DerefMut)]
pub(crate) struct BasePower(pub(crate) BasePowerType);

#[derive(Debug, Clone, Component, Deref, DerefMut)]
pub(crate) struct ModifiedBasePower(pub(crate) BasePowerType);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut)]
pub(crate) struct BasePowerModifier(pub(crate) i32);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut, Default,
)]
pub(crate) struct AddPower(pub(crate) i32);

#[derive(Debug, Clone, Component, Deref, DerefMut)]
pub(crate) struct BaseToughness(pub(crate) BaseToughnessType);

#[derive(Debug, Clone, Component, Deref, DerefMut)]
pub(crate) struct ModifiedBaseToughness(pub(crate) BaseToughnessType);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut)]
pub(crate) struct BaseToughnessModifier(pub(crate) i32);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut, Default,
)]
pub(crate) struct AddToughness(pub(crate) i32);

#[derive(Debug, Clone, Copy, Component)]
pub(crate) struct EtbTapped;

#[derive(Debug, Clone, Component)]
pub(crate) enum ModifyKeywords {
    Remove(HashSet<Keyword>),
    Add(Counter<Keyword>),
}

#[derive(Debug, Clone, Default, Component)]
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

    pub(crate) ward: Option<Ward>,

    pub(crate) static_abilities: Vec<StaticAbility>,

    pub(crate) activated_abilities: Vec<ActivatedAbility>,

    pub(crate) triggered_abilities: Vec<TriggeredAbility>,

    pub(crate) replacement_effects: Vec<ReplacementEffect>,

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
            ward: value
                .ward
                .as_ref()
                .map_or(Ok(None), |ward| ward.try_into().map(Some))?,
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
            replacement_effects: value
                .replacement_effects
                .iter()
                .map(ReplacementEffect::try_from)
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

pub(crate) fn replace_symbols(result: &str) -> String {
    let result = result.replace("{W}", "\u{e600}");
    let result = result.replace("{U}", "\u{e601}");
    let result = result.replace("{B}", "\u{e602}");
    let result = result.replace("{R}", "\u{e603}");
    let result = result.replace("{G}", "\u{e604}");
    let result = result.replace("{C}", "\u{e904}");
    let result = result.replace("{0}", "\u{e605}");
    let result = result.replace("{1}", "\u{e606}");
    let result = result.replace("{2}", "\u{e607}");
    let result = result.replace("{3}", "\u{e608}");
    let result = result.replace("{4}", "\u{e609}");
    let result = result.replace("{5}", "\u{e60a}");
    let result = result.replace("{6}", "\u{e60b}");
    let result = result.replace("{7}", "\u{e60c}");
    let result = result.replace("{8}", "\u{e60d}");
    let result = result.replace("{9}", "\u{e60e}");
    let result = result.replace("{10}", "\u{e60f}");
    let result = result.replace("{11}", "\u{e610}");
    let result = result.replace("{12}", "\u{e611}");
    let result = result.replace("{13}", "\u{e612}");
    let result = result.replace("{14}", "\u{e613}");
    let result = result.replace("{15}", "\u{e614}");
    let result = result.replace("{16}", "\u{e62a}");
    let result = result.replace("{17}", "\u{e62b}");
    let result = result.replace("{18}", "\u{e62c}");
    let result = result.replace("{19}", "\u{e62d}");
    let result = result.replace("{20}", "\u{e62e}");
    let result = result.replace("{X}", "\u{e615}");
    // Tap
    let result = result.replace("{T}", "\u{e61a}");
    // Untap
    result.replace("{Q}", "\u{e61b}")
}
