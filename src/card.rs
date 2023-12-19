use std::{collections::HashSet, str::FromStr};

use anyhow::{anyhow, Context};
use bevy_ecs::component::Component;
use counter::Counter;
use derive_more::{Deref, DerefMut};
use indexmap::IndexSet;
use strum::IntoEnumIterator;

use crate::{
    abilities::{ActivatedAbility, Enchant, GainManaAbility, StaticAbility, TriggeredAbility},
    cost::CastingCost,
    effects::{AnyEffect, ReplacementEffect, Token, TokenCreature},
    in_play::{AbilityId, TriggerId},
    newtype_enum::newtype_enum,
    protogen,
    types::{Subtype, Type},
};

#[derive(Debug, Clone, Deref, DerefMut, Component)]
pub struct Keywords(pub Counter<Keyword>);

#[derive(Debug, Clone, Deref, DerefMut, Component)]
pub struct ModifiedKeywords(pub Counter<Keyword>);

#[rustfmt::skip]
newtype_enum!{
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bevy_ecs::component::Component)]
#[derive(strum::EnumIter, strum::EnumString)]
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
    pub fn all() -> HashSet<Keyword> {
        Keyword::iter().collect()
    }
}

#[derive(Debug, Clone, Copy, Component)]
pub struct TargetIndividually;

#[derive(Debug, Clone, Copy, Component)]
pub struct Revealed;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct CannotBeCountered;

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut)]
pub struct Colors(pub HashSet<Color>);

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut)]
pub struct ModifiedColors(pub HashSet<Color>);

#[derive(Debug, Clone, PartialEq, Eq, Component, Deref, DerefMut)]
pub struct AddColors(pub HashSet<Color>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct RemoveAllColors;

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

#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub enum StaticAbilityModifier {
    RemoveAll,
    Add(StaticAbility),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub enum ActivatedAbilityModifier {
    RemoveAll,
    Add(AbilityId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub enum TriggeredAbilityModifier {
    RemoveAll,
    Add(TriggerId),
}

#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub enum EtbAbilityModifier {
    RemoveAll,
    Add(AbilityId),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut)]
pub struct Name(pub String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut)]
pub struct OracleText(pub String);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut, Default,
)]
pub struct MarkedDamage(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut)]
pub struct BasePower(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut)]
pub struct ModifiedBasePower(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut)]
pub struct BasePowerModifier(pub i32);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut, Default,
)]
pub struct AddPower(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut)]
pub struct BaseToughness(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut)]
pub struct ModifiedBaseToughness(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut)]
pub struct BaseToughnessModifier(pub i32);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Component, Deref, DerefMut, Default,
)]
pub struct AddToughness(pub i32);

#[derive(Debug, Clone, Copy, Component)]
pub struct EtbTapped;

#[derive(Debug, Clone, Component)]
pub enum ModifyKeywords {
    Remove(HashSet<Keyword>),
    Add(Counter<Keyword>),
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Component)]
pub struct Card {
    pub name: String,
    pub types: IndexSet<Type>,
    pub subtypes: IndexSet<Subtype>,

    pub cost: CastingCost,
    pub cannot_be_countered: bool,

    pub colors: HashSet<Color>,

    pub oracle_text: String,

    pub enchant: Option<Enchant>,

    pub etb_abilities: Vec<AnyEffect>,
    pub effects: Vec<AnyEffect>,
    pub target_individually: bool,

    pub static_abilities: Vec<StaticAbility>,

    pub activated_abilities: Vec<ActivatedAbility>,

    pub triggered_abilities: Vec<TriggeredAbility>,

    pub replacement_effects: Vec<ReplacementEffect>,

    pub mana_abilities: Vec<GainManaAbility>,

    pub power: Option<usize>,
    pub toughness: Option<usize>,

    pub etb_tapped: bool,

    pub keywords: Counter<Keyword>,
}

impl TryFrom<protogen::card::Card> for Card {
    type Error = anyhow::Error;

    fn try_from(value: protogen::card::Card) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            types: value
                .types
                .iter()
                .map(Type::try_from)
                .collect::<anyhow::Result<_>>()?,
            subtypes: value
                .subtypes
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<_>>()?,
            cost: value.cost.get_or_default().try_into()?,
            cannot_be_countered: value.cannot_be_countered,
            colors: value
                .colors
                .iter()
                .map(Color::try_from)
                .collect::<anyhow::Result<HashSet<_>>>()?,
            oracle_text: value.oracle_text,
            enchant: value
                .enchant
                .as_ref()
                .map_or(Ok(None), |enchant| enchant.try_into().map(Some))?,
            etb_abilities: value
                .etb_abilities
                .iter()
                .map(AnyEffect::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            effects: value
                .effects
                .iter()
                .map(AnyEffect::try_from)
                .collect::<anyhow::Result<Vec<_>>>()?,
            target_individually: value.target_individually,
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
        })
    }
}

impl From<Token> for Card {
    fn from(value: Token) -> Self {
        match value {
            Token::Creature(TokenCreature {
                name,
                types,
                subtypes,
                colors,
                power,
                toughness,
                keywords,
            }) => Self {
                name,
                types,
                subtypes,
                colors,
                power: Some(power),
                toughness: Some(toughness),
                keywords,
                ..Default::default()
            },
        }
    }
}
