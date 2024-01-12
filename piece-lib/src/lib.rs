#![allow(clippy::single_match)]

#[macro_use]
extern crate tracing;

use std::{collections::HashMap, marker::PhantomData};

use anyhow::{anyhow, Context};

use ariadne::{Label, Report, ReportKind, Source};
use convert_case::{Case, Casing};
use include_dir::{include_dir, Dir, File};
use indexmap::IndexMap;
use itertools::Itertools;
use protobuf::{Enum, MessageDyn, MessageFull};
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};

use crate::protogen::{
    card::Card,
    cost::ManaCost,
    counters::Counter,
    effects::gain_mana,
    empty::Empty,
    keywords::Keyword,
    mana::Mana,
    types::{Subtype, Type, Typeline},
};

#[cfg(test)]
mod _tests;

pub mod abilities;
pub mod action_result;
pub mod ai;
pub mod battlefield;
pub mod card;
pub mod cost;
pub mod effects;
pub mod exile;
pub mod graveyard;
pub mod hand;
pub mod in_play;
pub mod library;
pub mod log;
pub mod mana;
pub mod pending_results;
pub mod player;
pub mod protogen;
pub mod stack;
pub mod turns;
pub mod types;

pub static FONT_DATA: &[u8] = include_bytes!("../../fonts/mana.ttf");

static CARD_DEFINITIONS: Dir = include_dir!("cards");

pub type Cards = IndexMap<String, Card>;

pub fn load_protos() -> anyhow::Result<Vec<(Card, &'static File<'static>)>> {
    fn dir_to_files(dir: &'static Dir) -> Vec<&'static File<'static>> {
        let mut results = vec![];
        for entry in dir.entries() {
            match entry {
                include_dir::DirEntry::Dir(dir) => results.extend(dir_to_files(dir)),
                include_dir::DirEntry::File(file) => {
                    results.push(file);
                }
            }
        }

        results
    }

    let mut results = vec![];
    for card_file in CARD_DEFINITIONS
        .entries()
        .iter()
        .flat_map(|entry| match entry {
            include_dir::DirEntry::Dir(dir) => dir_to_files(dir).into_iter(),
            include_dir::DirEntry::File(file) => vec![file].into_iter(),
        })
    {
        let contents = card_file.contents();

        let card: protogen::card::Card = serde_yaml::from_slice(contents)
            .map_err(|e| {
                let location = e.location().unwrap();
                Report::build(
                    ReportKind::Error,
                    card_file.path().display().to_string(),
                    location.index(),
                )
                .with_label(Label::new((
                    card_file.path().display().to_string(),
                    location.index()..location.index() + 1,
                )))
                .with_message(e.to_string())
                .finish()
                .eprint((
                    card_file.path().display().to_string(),
                    Source::from(std::str::from_utf8(contents).expect("Invalid utf8")),
                ))
                .unwrap();

                anyhow!(e.to_string())
            })
            .with_context(|| format!("Parsing file: {}", card_file.path().display()))?;

        results.push((card, card_file));
    }

    Ok(results)
}

pub fn load_cards() -> anyhow::Result<Cards> {
    let timer = std::time::Instant::now();
    let protos = load_protos()?;
    info!(
        "Loaded {} cards in {}ms",
        protos.len(),
        timer.elapsed().as_millis()
    );

    let timer = std::time::Instant::now();
    let mut cards = Cards::with_capacity(protos.len());
    for (card, _) in protos {
        if let Some(overwritten) = cards.insert(card.name.clone(), card) {
            warn!("Overwriting card {}", overwritten.name);
        };
    }

    info!(
        "Converted {} cards in {}ms",
        cards.len(),
        timer.elapsed().as_millis()
    );

    Ok(cards)
}

fn is_default_value<T: Default + PartialEq>(t: &T) -> bool {
    *t == T::default()
}

fn deserialize_gain_mana<'de, D>(
    deserializer: D,
) -> Result<Vec<protobuf::EnumOrUnknown<Mana>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visit;
    impl<'de> Visitor<'de> for Visit {
        type Value = Vec<protobuf::EnumOrUnknown<Mana>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("expected a sequence of {W}, {U}, {B}, {R}, {G}, or {C}")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            parse_mana_symbol(v)
        }
    }

    deserializer.deserialize_str(Visit)
}

fn parse_mana_symbol<E>(v: &str) -> Result<Vec<protobuf::EnumOrUnknown<Mana>>, E>
where
    E: serde::de::Error,
{
    let split = v
        .split('}')
        .map(|s| s.trim_start_matches('{'))
        .filter(|s| !s.is_empty())
        .collect_vec();

    let mut results = vec![];
    for symbol in split {
        let mana = match symbol {
            "W" => Mana::WHITE,
            "U" => Mana::BLUE,
            "B" => Mana::BLACK,
            "R" => Mana::RED,
            "G" => Mana::GREEN,
            "C" => Mana::COLORLESS,
            s => {
                return Err(E::custom(format!("Invalid mana {}", s)));
            }
        };

        results.push(protobuf::EnumOrUnknown::new(mana));
    }
    Ok(results)
}

fn serialize_gain_mana<S>(
    value: &[protobuf::EnumOrUnknown<Mana>],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let result = mana_to_string(value);

    serializer.serialize_str(&result)
}

fn mana_to_string(value: &[protobuf::EnumOrUnknown<Mana>]) -> String {
    let mut result = String::default();
    for mana in value.iter() {
        match mana.enum_value().unwrap() {
            protogen::mana::Mana::WHITE => result.push_str("{W}"),
            protogen::mana::Mana::BLUE => result.push_str("{U}"),
            protogen::mana::Mana::BLACK => result.push_str("{B}"),
            protogen::mana::Mana::RED => result.push_str("{R}"),
            protogen::mana::Mana::GREEN => result.push_str("{G}"),
            protogen::mana::Mana::COLORLESS => result.push_str("{C}"),
        }
    }
    result
}

fn deserialize_mana_choice<'de, D>(deserializer: D) -> Result<Vec<gain_mana::GainMana>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visit;
    impl<'de> Visitor<'de> for Visit {
        type Value = Vec<gain_mana::GainMana>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter
                .write_str("expected a comma separated sequence of {W}, {U}, {B}, {R}, {G}, or {C}")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            v.split(',')
                .map(|v| v.trim())
                .map(parse_mana_symbol)
                .map(|gains| {
                    Ok(gain_mana::GainMana {
                        gains: gains?,
                        ..Default::default()
                    })
                })
                .collect::<Result<Self::Value, E>>()
        }
    }

    deserializer.deserialize_str(Visit)
}

fn serialize_mana_choice<S>(value: &[gain_mana::GainMana], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(
        &value
            .iter()
            .map(|gain| mana_to_string(&gain.gains))
            .join(", "),
    )
}

fn deserialize_mana_cost<'de, D>(
    deserializer: D,
) -> Result<Vec<protobuf::EnumOrUnknown<ManaCost>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visit;
    impl<'de> Visitor<'de> for Visit {
        type Value = Vec<protobuf::EnumOrUnknown<ManaCost>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("expected a sequence of {W}, {U}, {B}, {R}, {G}, {C}, or {#}")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let split = v
                .split('}')
                .map(|s| s.trim_start_matches('{'))
                .filter(|s| !s.is_empty())
                .collect_vec();

            let mut results = vec![];
            for symbol in split {
                if let Ok(count) = symbol.parse::<usize>() {
                    for _ in 0..count {
                        results.push(ManaCost::GENERIC);
                    }
                } else {
                    let cost = match symbol {
                        "W" => ManaCost::WHITE,
                        "U" => ManaCost::BLUE,
                        "B" => ManaCost::BLACK,
                        "R" => ManaCost::RED,
                        "G" => ManaCost::GREEN,
                        "X" => ManaCost::X,
                        "C" => ManaCost::COLORLESS,
                        s => {
                            return Err(E::custom(format!("Invalid mana cost {}", s)));
                        }
                    };

                    if matches!(cost, ManaCost::X) && matches!(results.last(), Some(ManaCost::X)) {
                        results.pop();
                        results.push(ManaCost::TWO_X);
                    } else {
                        results.push(cost);
                    }
                }
            }

            Ok(results
                .into_iter()
                .map(protobuf::EnumOrUnknown::new)
                .collect_vec())
        }
    }

    deserializer.deserialize_str(Visit)
}

fn serialize_mana_cost<S>(
    value: &[protobuf::EnumOrUnknown<ManaCost>],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut result = String::default();

    let generic = value
        .iter()
        .filter(|cost| matches!(cost.enum_value().unwrap(), ManaCost::GENERIC))
        .count();

    let mut pushed_generic = false;
    for mana in value.iter() {
        match mana.enum_value().unwrap() {
            ManaCost::WHITE => result.push_str("{W}"),
            ManaCost::BLUE => result.push_str("{U}"),
            ManaCost::BLACK => result.push_str("{B}"),
            ManaCost::RED => result.push_str("{R}"),
            ManaCost::GREEN => result.push_str("{G}"),
            ManaCost::COLORLESS => result.push_str("{C}"),
            ManaCost::GENERIC => {
                if !pushed_generic {
                    match generic {
                        0 => result.push_str("{0}"),
                        1 => result.push_str("{1}"),
                        2 => result.push_str("{2}"),
                        3 => result.push_str("{3}"),
                        4 => result.push_str("{4}"),
                        5 => result.push_str("{5}"),
                        6 => result.push_str("{6}"),
                        7 => result.push_str("{7}"),
                        8 => result.push_str("{8}"),
                        9 => result.push_str("{9}"),
                        10 => result.push_str("{10}"),
                        11 => result.push_str("{11}"),
                        12 => result.push_str("{12}"),
                        13 => result.push_str("{13}"),
                        14 => result.push_str("{14}"),
                        15 => result.push_str("{15}"),
                        16 => result.push_str("{16}"),
                        17 => result.push_str("{17}"),
                        18 => result.push_str("{18}"),
                        19 => result.push_str("{19}"),
                        20 => result.push_str("{20}"),
                        _ => result.push_str(&format!("{{{}}}", generic)),
                    }
                    pushed_generic = true;
                }
            }
            ManaCost::X => result.push_str("{X}"),
            ManaCost::TWO_X => result.push_str("{X}{X}"),
        }
    }

    serializer.serialize_str(&result)
}
fn deserialize_typeline<'de, D>(
    deserializer: D,
) -> Result<protobuf::MessageField<Typeline>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visit;
    impl<'de> Visitor<'de> for Visit {
        type Value = protobuf::MessageField<Typeline>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("expected a comma separate sequence of types")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if v.is_empty() {
                return Err(E::custom("Expected typeline to be set"));
            }

            let types_and_subtypes = v.split('-').collect_vec();
            let (types, subtypes) = match types_and_subtypes.as_slice() {
                [types] => (types, &""),
                [types, subtypes] => (types, subtypes),
                _ => return Err(E::custom(format!("Invalid typeline {}", v))),
            };

            let types = types
                .split(' ')
                .filter(|ty| !ty.is_empty())
                .map(|ty| {
                    Type::from_str(&ty.to_case(Case::ScreamingSnake))
                        .map(protobuf::EnumOrUnknown::new)
                        .ok_or_else(|| E::custom(format!("Unknown variant: {}", ty)))
                })
                .collect::<Result<Vec<protobuf::EnumOrUnknown<Type>>, E>>()?;

            let subtypes = subtypes
                .split(' ')
                .filter(|ty| !ty.is_empty())
                .map(|ty| {
                    Subtype::from_str(&ty.to_case(Case::ScreamingSnake))
                        .map(protobuf::EnumOrUnknown::new)
                        .ok_or_else(|| E::custom(format!("Unknown variant: {}", ty)))
                })
                .collect::<Result<Vec<protobuf::EnumOrUnknown<Subtype>>, E>>()?;

            Ok(protobuf::MessageField::some(Typeline {
                types,
                subtypes,
                ..Default::default()
            }))
        }
    }

    deserializer.deserialize_str(Visit)
}

fn serialize_typeline<S>(value: &Typeline, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let types = value
        .types
        .iter()
        .map(|ty| ty.enum_value().unwrap().as_ref().to_case(Case::UpperCamel))
        .join(" ");
    let subtypes = value
        .subtypes
        .iter()
        .map(|ty| ty.enum_value().unwrap().as_ref().to_case(Case::UpperCamel))
        .join(" ");

    if subtypes.is_empty() {
        serializer.serialize_str(&types)
    } else {
        serializer.serialize_str(&format!("{} - {}", types, subtypes))
    }
}

fn deserialize_types<'de, D>(deserializer: D) -> Result<HashMap<i32, Empty>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visit;
    impl<'de> Visitor<'de> for Visit {
        type Value = HashMap<i32, Empty>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("expected a comma separate sequence of types")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            v.split(',')
                .map(|v| v.trim())
                .map(|ty| {
                    Type::from_str(&ty.to_case(Case::ScreamingSnake))
                        .ok_or_else(|| E::custom(format!("Unknown variant: {}", ty)))
                })
                .map(|type_| type_.map(|type_| (type_.value(), Empty::default())))
                .collect::<Result<Self::Value, E>>()
        }
    }

    deserializer.deserialize_str(Visit)
}

fn serialize_types<S>(value: &HashMap<i32, Empty>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(
        &value
            .keys()
            .sorted()
            .map(|ty| Type::from_i32(*ty).unwrap().as_ref().to_case(Case::Title))
            .join(", "),
    )
}

fn deserialize_subtypes<'de, D>(deserializer: D) -> Result<HashMap<i32, Empty>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visit;
    impl<'de> Visitor<'de> for Visit {
        type Value = HashMap<i32, Empty>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("expected a comma separate sequence of subtypes")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            v.split(',')
                .map(|v| v.trim())
                .map(|ty| {
                    Subtype::from_str(&ty.to_case(Case::ScreamingSnake))
                        .ok_or_else(|| E::custom(format!("Unknown variant: {}", ty)))
                })
                .map(|subtype| subtype.map(|subtype| (subtype.value(), Empty::default())))
                .collect::<Result<Self::Value, E>>()
        }
    }

    deserializer.deserialize_str(Visit)
}

fn serialize_subtypes<S>(value: &HashMap<i32, Empty>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(
        &value
            .keys()
            .sorted()
            .map(|ty| {
                Subtype::from_i32(*ty)
                    .unwrap()
                    .as_ref()
                    .to_case(Case::Title)
            })
            .join(", "),
    )
}

fn deserialize_keywords<'de, D>(deserializer: D) -> Result<HashMap<i32, u32>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visit;
    impl<'de> Visitor<'de> for Visit {
        type Value = HashMap<i32, u32>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("expected a comma separate sequence of keywords")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let mut result = HashMap::default();
            for kw in v
                .split(',')
                .map(|v| v.trim())
                .map(|ty| {
                    Keyword::from_str(&ty.to_case(Case::ScreamingSnake))
                        .ok_or_else(|| E::custom(format!("Unknown variant: {}", ty)))
                })
                .map(|keyword| keyword.map(|keyword| keyword.value()))
            {
                *result.entry(kw?).or_default() += 1;
            }

            Ok(result)
        }
    }

    deserializer.deserialize_str(Visit)
}

fn serialize_keywords<S>(value: &HashMap<i32, u32>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(
        &value
            .iter()
            .flat_map(|(kw, count)| {
                std::iter::repeat(
                    Keyword::from_i32(*kw)
                        .unwrap()
                        .as_ref()
                        .to_case(Case::Title),
                )
                .take((*count) as usize)
            })
            .sorted()
            .join(", "),
    )
}

fn deserialize_enum_list<'de, T, D>(
    deserializer: D,
) -> Result<Vec<protobuf::EnumOrUnknown<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Enum,
{
    #[derive(Default)]
    struct Visit<T> {
        _p: PhantomData<T>,
    }

    impl<'de, T> Visitor<'de> for Visit<T>
    where
        T: Enum,
    {
        type Value = Vec<protobuf::EnumOrUnknown<T>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("expected a comma separate sequence of values")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            v.split(',')
                .map(|v| v.trim())
                .map(|ty| {
                    T::from_str(&ty.to_case(Case::ScreamingSnake))
                        .map(protobuf::EnumOrUnknown::new)
                        .ok_or_else(|| E::custom(format!("Unknown variant: {}", ty)))
                })
                .collect::<Result<Self::Value, E>>()
        }
    }

    deserializer.deserialize_str(Visit::<T>::default())
}

fn serialize_enum_list<T, S>(
    values: &[protobuf::EnumOrUnknown<T>],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize + AsRef<str> + Enum + std::cmp::Ord,
{
    serializer.serialize_str(
        &values
            .iter()
            .sorted()
            .map(|v| v.enum_value().unwrap().as_ref().to_case(Case::Title))
            .join(", "),
    )
}

fn serialize_message<T, S>(
    value: &::protobuf::MessageField<T>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize + MessageFull,
{
    if let Some(value) = value.as_ref() {
        value.serialize(serializer)
    } else {
        serializer.serialize_none()
    }
}

fn deserialize_message<'de, T, D>(deserializer: D) -> Result<::protobuf::MessageField<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + MessageFull,
{
    Option::<T>::deserialize(deserializer).and_then(|t| {
        let message = ::protobuf::MessageField::from_option(t);

        let descriptor = message.descriptor_dyn();
        for oneof in descriptor.oneofs() {
            if oneof
                .fields()
                .all(|field| field.get_singular(message.get_or_default()).is_none())
            {
                return Err(<D::Error as serde::de::Error>::custom(format!(
                    "Expected at least one type of {} to be set",
                    oneof.name()
                )));
            }
        }
        Ok(message)
    })
}

fn serialize_enum<T, S>(
    value: &::protobuf::EnumOrUnknown<T>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize + Enum + AsRef<str>,
{
    serializer.serialize_str(&value.enum_value().unwrap().as_ref().to_case(Case::Lower))
}

fn deserialize_enum<'de, T, D>(deserializer: D) -> Result<::protobuf::EnumOrUnknown<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Enum,
{
    #[derive(Default)]
    struct Visit<T> {
        _p: PhantomData<T>,
    }

    impl<'de, T> Visitor<'de> for Visit<T>
    where
        T: Enum,
    {
        type Value = protobuf::EnumOrUnknown<T>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("expected a comma separate sequence of values")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            T::from_str(&v.to_case(Case::ScreamingSnake))
                .map(protobuf::EnumOrUnknown::new)
                .ok_or_else(|| E::custom(format!("Unknown variant: {}", v)))
        }
    }

    deserializer.deserialize_str(Visit::<T>::default())
}

fn serialize_counter<S>(
    value: &::protobuf::EnumOrUnknown<Counter>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value.enum_value().unwrap() {
        Counter::P1P1 => serializer.serialize_str("+1/+1"),
        Counter::M1M1 => serializer.serialize_str("-1/-1"),
        value => serializer.serialize_str(&value.as_ref().to_case(Case::Lower)),
    }
}

fn deserialize_counter<'de, D>(
    deserializer: D,
) -> Result<::protobuf::EnumOrUnknown<Counter>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visit;

    impl<'de> Visitor<'de> for Visit {
        type Value = protobuf::EnumOrUnknown<Counter>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("expected a comma separate sequence of values")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match v.trim() {
                "+1/+1" => Ok(protobuf::EnumOrUnknown::new(Counter::P1P1)),
                "-1/-1" => Ok(protobuf::EnumOrUnknown::new(Counter::M1M1)),
                v => Counter::from_str(&v.to_case(Case::ScreamingSnake))
                    .map(protobuf::EnumOrUnknown::new)
                    .ok_or_else(|| E::custom(format!("Unknown variant: {}", v))),
            }
        }
    }

    deserializer.deserialize_str(Visit)
}
