use std::collections::{HashMap, HashSet};

use aho_corasick::AhoCorasick;
use anyhow::anyhow;
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
    types::{Subtype, Type},
};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter, strum::EnumString, strum::AsRefStr,
)]
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
    DayboundAndNightbound,
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
    MoreThanMeetsTheEye,
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

impl TryFrom<&protogen::keywords::Keyword> for Keyword {
    type Error = anyhow::Error;

    fn try_from(value: &protogen::keywords::Keyword) -> Result<Self, Self::Error> {
        value
            .keyword
            .as_ref()
            .ok_or_else(|| anyhow!("Expected keyword to have a keyword set."))
            .map(Self::from)
    }
}

impl From<&protogen::keywords::keyword::Keyword> for Keyword {
    fn from(value: &protogen::keywords::keyword::Keyword) -> Self {
        match value {
            protogen::keywords::keyword::Keyword::Absorb(_) => Self::Absorb,
            protogen::keywords::keyword::Keyword::Affinity(_) => Self::Affinity,
            protogen::keywords::keyword::Keyword::Afflict(_) => Self::Afflict,
            protogen::keywords::keyword::Keyword::Afterlife(_) => Self::Afterlife,
            protogen::keywords::keyword::Keyword::Aftermath(_) => Self::Aftermath,
            protogen::keywords::keyword::Keyword::Amplify(_) => Self::Amplify,
            protogen::keywords::keyword::Keyword::Annihilator(_) => Self::Annihilator,
            protogen::keywords::keyword::Keyword::Ascend(_) => Self::Ascend,
            protogen::keywords::keyword::Keyword::Assist(_) => Self::Assist,
            protogen::keywords::keyword::Keyword::AuraSwap(_) => Self::AuraSwap,
            protogen::keywords::keyword::Keyword::Awaken(_) => Self::Awaken,
            protogen::keywords::keyword::Keyword::Backup(_) => Self::Backup,
            protogen::keywords::keyword::Keyword::Banding(_) => Self::Banding,
            protogen::keywords::keyword::Keyword::Bargain(_) => Self::Bargain,
            protogen::keywords::keyword::Keyword::BattleCry(_) => Self::BattleCry,
            protogen::keywords::keyword::Keyword::Bestow(_) => Self::Bestow,
            protogen::keywords::keyword::Keyword::Blitz(_) => Self::Blitz,
            protogen::keywords::keyword::Keyword::Bloodthirst(_) => Self::Bloodthirst,
            protogen::keywords::keyword::Keyword::Boast(_) => Self::Boast,
            protogen::keywords::keyword::Keyword::Bushido(_) => Self::Bushido,
            protogen::keywords::keyword::Keyword::Buyback(_) => Self::Buyback,
            protogen::keywords::keyword::Keyword::Cascade(_) => Self::Cascade,
            protogen::keywords::keyword::Keyword::Casualty(_) => Self::Casualty,
            protogen::keywords::keyword::Keyword::Champion(_) => Self::Champion,
            protogen::keywords::keyword::Keyword::Changeling(_) => Self::Changeling,
            protogen::keywords::keyword::Keyword::Cipher(_) => Self::Cipher,
            protogen::keywords::keyword::Keyword::Cleave(_) => Self::Cleave,
            protogen::keywords::keyword::Keyword::Companion(_) => Self::Companion,
            protogen::keywords::keyword::Keyword::Compleated(_) => Self::Compleated,
            protogen::keywords::keyword::Keyword::Conspire(_) => Self::Conspire,
            protogen::keywords::keyword::Keyword::Convoke(_) => Self::Convoke,
            protogen::keywords::keyword::Keyword::Craft(_) => Self::Craft,
            protogen::keywords::keyword::Keyword::Crew(_) => Self::Crew,
            protogen::keywords::keyword::Keyword::CumulativeUpkeep(_) => Self::CumulativeUpkeep,
            protogen::keywords::keyword::Keyword::Cycling(_) => Self::Cycling,
            protogen::keywords::keyword::Keyword::Dash(_) => Self::Dash,
            protogen::keywords::keyword::Keyword::DayboundAndNightbound(_) => {
                Self::DayboundAndNightbound
            }
            protogen::keywords::keyword::Keyword::Deathtouch(_) => Self::Deathtouch,
            protogen::keywords::keyword::Keyword::Decayed(_) => Self::Decayed,
            protogen::keywords::keyword::Keyword::Defender(_) => Self::Defender,
            protogen::keywords::keyword::Keyword::Delve(_) => Self::Delve,
            protogen::keywords::keyword::Keyword::Demonstrate(_) => Self::Demonstrate,
            protogen::keywords::keyword::Keyword::Dethrone(_) => Self::Dethrone,
            protogen::keywords::keyword::Keyword::Devoid(_) => Self::Devoid,
            protogen::keywords::keyword::Keyword::Devour(_) => Self::Devour,
            protogen::keywords::keyword::Keyword::Disturb(_) => Self::Disturb,
            protogen::keywords::keyword::Keyword::DoubleStrike(_) => Self::DoubleStrike,
            protogen::keywords::keyword::Keyword::Dredge(_) => Self::Dredge,
            protogen::keywords::keyword::Keyword::Echo(_) => Self::Echo,
            protogen::keywords::keyword::Keyword::Embalm(_) => Self::Embalm,
            protogen::keywords::keyword::Keyword::Emerge(_) => Self::Emerge,
            protogen::keywords::keyword::Keyword::Enchant(_) => Self::Enchant,
            protogen::keywords::keyword::Keyword::Encore(_) => Self::Encore,
            protogen::keywords::keyword::Keyword::Enlist(_) => Self::Enlist,
            protogen::keywords::keyword::Keyword::Entwine(_) => Self::Entwine,
            protogen::keywords::keyword::Keyword::Epic(_) => Self::Epic,
            protogen::keywords::keyword::Keyword::Equip(_) => Self::Equip,
            protogen::keywords::keyword::Keyword::Escalate(_) => Self::Escalate,
            protogen::keywords::keyword::Keyword::Escape(_) => Self::Escape,
            protogen::keywords::keyword::Keyword::Eternalize(_) => Self::Eternalize,
            protogen::keywords::keyword::Keyword::Evoke(_) => Self::Evoke,
            protogen::keywords::keyword::Keyword::Evolve(_) => Self::Evolve,
            protogen::keywords::keyword::Keyword::Exalted(_) => Self::Exalted,
            protogen::keywords::keyword::Keyword::Exploit(_) => Self::Exploit,
            protogen::keywords::keyword::Keyword::Extort(_) => Self::Extort,
            protogen::keywords::keyword::Keyword::Fabricate(_) => Self::Fabricate,
            protogen::keywords::keyword::Keyword::Fading(_) => Self::Fading,
            protogen::keywords::keyword::Keyword::Fear(_) => Self::Fear,
            protogen::keywords::keyword::Keyword::FirstStrike(_) => Self::FirstStrike,
            protogen::keywords::keyword::Keyword::Flanking(_) => Self::Flanking,
            protogen::keywords::keyword::Keyword::Flash(_) => Self::Flash,
            protogen::keywords::keyword::Keyword::Flashback(_) => Self::Flashback,
            protogen::keywords::keyword::Keyword::Flying(_) => Self::Flying,
            protogen::keywords::keyword::Keyword::Forecast(_) => Self::Forecast,
            protogen::keywords::keyword::Keyword::Foretell(_) => Self::Foretell,
            protogen::keywords::keyword::Keyword::ForMirrodin(_) => Self::ForMirrodin,
            protogen::keywords::keyword::Keyword::Fortify(_) => Self::Fortify,
            protogen::keywords::keyword::Keyword::Frenzy(_) => Self::Frenzy,
            protogen::keywords::keyword::Keyword::Fuse(_) => Self::Fuse,
            protogen::keywords::keyword::Keyword::Graft(_) => Self::Graft,
            protogen::keywords::keyword::Keyword::Gravestorm(_) => Self::Gravestorm,
            protogen::keywords::keyword::Keyword::Haste(_) => Self::Haste,
            protogen::keywords::keyword::Keyword::Haunt(_) => Self::Haunt,
            protogen::keywords::keyword::Keyword::Hexproof(_) => Self::Hexproof,
            protogen::keywords::keyword::Keyword::HiddenAgenda(_) => Self::HiddenAgenda,
            protogen::keywords::keyword::Keyword::Hideaway(_) => Self::Hideaway,
            protogen::keywords::keyword::Keyword::Horsemanship(_) => Self::Horsemanship,
            protogen::keywords::keyword::Keyword::Improvise(_) => Self::Improvise,
            protogen::keywords::keyword::Keyword::Indestructible(_) => Self::Indestructible,
            protogen::keywords::keyword::Keyword::Infect(_) => Self::Infect,
            protogen::keywords::keyword::Keyword::Ingest(_) => Self::Ingest,
            protogen::keywords::keyword::Keyword::Intimidate(_) => Self::Intimidate,
            protogen::keywords::keyword::Keyword::JumpStart(_) => Self::JumpStart,
            protogen::keywords::keyword::Keyword::Kicker(_) => Self::Kicker,
            protogen::keywords::keyword::Keyword::Landwalk(_) => Self::Landwalk,
            protogen::keywords::keyword::Keyword::LevelUp(_) => Self::LevelUp,
            protogen::keywords::keyword::Keyword::Lifelink(_) => Self::Lifelink,
            protogen::keywords::keyword::Keyword::LivingMetal(_) => Self::LivingMetal,
            protogen::keywords::keyword::Keyword::LivingWeapon(_) => Self::LivingWeapon,
            protogen::keywords::keyword::Keyword::Madness(_) => Self::Madness,
            protogen::keywords::keyword::Keyword::Melee(_) => Self::Melee,
            protogen::keywords::keyword::Keyword::Menace(_) => Self::Menace,
            protogen::keywords::keyword::Keyword::Mentor(_) => Self::Mentor,
            protogen::keywords::keyword::Keyword::Miracle(_) => Self::Miracle,
            protogen::keywords::keyword::Keyword::Modular(_) => Self::Modular,
            protogen::keywords::keyword::Keyword::MoreThanMeetsTheEye(_) => {
                Self::MoreThanMeetsTheEye
            }
            protogen::keywords::keyword::Keyword::Morph(_) => Self::Morph,
            protogen::keywords::keyword::Keyword::Mutate(_) => Self::Mutate,
            protogen::keywords::keyword::Keyword::Myriad(_) => Self::Myriad,
            protogen::keywords::keyword::Keyword::Ninjutsu(_) => Self::Ninjutsu,
            protogen::keywords::keyword::Keyword::Offering(_) => Self::Offering,
            protogen::keywords::keyword::Keyword::Outlast(_) => Self::Outlast,
            protogen::keywords::keyword::Keyword::Overload(_) => Self::Overload,
            protogen::keywords::keyword::Keyword::Partner(_) => Self::Partner,
            protogen::keywords::keyword::Keyword::Persist(_) => Self::Persist,
            protogen::keywords::keyword::Keyword::Phasing(_) => Self::Phasing,
            protogen::keywords::keyword::Keyword::Poisonous(_) => Self::Poisonous,
            protogen::keywords::keyword::Keyword::Protection(_) => Self::Protection,
            protogen::keywords::keyword::Keyword::Prototype(_) => Self::Prototype,
            protogen::keywords::keyword::Keyword::Provoke(_) => Self::Provoke,
            protogen::keywords::keyword::Keyword::Prowess(_) => Self::Prowess,
            protogen::keywords::keyword::Keyword::Prowl(_) => Self::Prowl,
            protogen::keywords::keyword::Keyword::Rampage(_) => Self::Rampage,
            protogen::keywords::keyword::Keyword::Ravenous(_) => Self::Ravenous,
            protogen::keywords::keyword::Keyword::Reach(_) => Self::Reach,
            protogen::keywords::keyword::Keyword::ReadAhead(_) => Self::ReadAhead,
            protogen::keywords::keyword::Keyword::Rebound(_) => Self::Rebound,
            protogen::keywords::keyword::Keyword::Reconfigure(_) => Self::Reconfigure,
            protogen::keywords::keyword::Keyword::Recover(_) => Self::Recover,
            protogen::keywords::keyword::Keyword::Reinforce(_) => Self::Reinforce,
            protogen::keywords::keyword::Keyword::Renown(_) => Self::Renown,
            protogen::keywords::keyword::Keyword::Replicate(_) => Self::Replicate,
            protogen::keywords::keyword::Keyword::Retrace(_) => Self::Retrace,
            protogen::keywords::keyword::Keyword::Riot(_) => Self::Riot,
            protogen::keywords::keyword::Keyword::Ripple(_) => Self::Ripple,
            protogen::keywords::keyword::Keyword::Scavenge(_) => Self::Scavenge,
            protogen::keywords::keyword::Keyword::Shadow(_) => Self::Shadow,
            protogen::keywords::keyword::Keyword::Shroud(_) => Self::Shroud,
            protogen::keywords::keyword::Keyword::Skulk(_) => Self::Skulk,
            protogen::keywords::keyword::Keyword::Soulbond(_) => Self::Soulbond,
            protogen::keywords::keyword::Keyword::Soulshift(_) => Self::Soulshift,
            protogen::keywords::keyword::Keyword::SpaceSculptor(_) => Self::SpaceSculptor,
            protogen::keywords::keyword::Keyword::Spectacle(_) => Self::Spectacle,
            protogen::keywords::keyword::Keyword::Splice(_) => Self::Splice,
            protogen::keywords::keyword::Keyword::SplitSecond(_) => Self::SplitSecond,
            protogen::keywords::keyword::Keyword::Squad(_) => Self::Squad,
            protogen::keywords::keyword::Keyword::Storm(_) => Self::Storm,
            protogen::keywords::keyword::Keyword::Sunburst(_) => Self::Sunburst,
            protogen::keywords::keyword::Keyword::Surge(_) => Self::Surge,
            protogen::keywords::keyword::Keyword::Suspend(_) => Self::Suspend,
            protogen::keywords::keyword::Keyword::TotemArmor(_) => Self::TotemArmor,
            protogen::keywords::keyword::Keyword::Toxic(_) => Self::Toxic,
            protogen::keywords::keyword::Keyword::Training(_) => Self::Training,
            protogen::keywords::keyword::Keyword::Trample(_) => Self::Trample,
            protogen::keywords::keyword::Keyword::Transfigure(_) => Self::Transfigure,
            protogen::keywords::keyword::Keyword::Transmute(_) => Self::Transmute,
            protogen::keywords::keyword::Keyword::Tribute(_) => Self::Tribute,
            protogen::keywords::keyword::Keyword::Undaunted(_) => Self::Undaunted,
            protogen::keywords::keyword::Keyword::Undying(_) => Self::Undying,
            protogen::keywords::keyword::Keyword::Unearth(_) => Self::Unearth,
            protogen::keywords::keyword::Keyword::Unleash(_) => Self::Unleash,
            protogen::keywords::keyword::Keyword::Vanishing(_) => Self::Vanishing,
            protogen::keywords::keyword::Keyword::Vigilance(_) => Self::Vigilance,
            protogen::keywords::keyword::Keyword::Visit(_) => Self::Visit,
            protogen::keywords::keyword::Keyword::Ward(_) => Self::Ward,
            protogen::keywords::keyword::Keyword::Wither(_) => Self::Wither,
        }
    }
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

    pub keywords: HashMap<String, u32>,

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
        let mut this = Self {
            name: value.name.clone(),
            types: value
                .typeline
                .types
                .iter()
                .map(Type::try_from)
                .collect::<anyhow::Result<_>>()?,
            subtypes: value
                .typeline
                .subtypes
                .iter()
                .map(Subtype::try_from)
                .collect::<anyhow::Result<_>>()?,
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
            keywords: value.keywords.clone(),
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
