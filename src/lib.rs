#![allow(clippy::single_match)]

#[macro_use]
extern crate tracing;

use std::{collections::HashMap, marker::PhantomData, str::FromStr};

use anyhow::{anyhow, Context};

use ariadne::{Label, Report, ReportKind, Source};
use convert_case::{Case, Casing};
use include_dir::{include_dir, Dir, File};
use indexmap::IndexMap;
use itertools::Itertools;
use protobuf::{Enum, MessageDyn, MessageFull};
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    card::Card,
    protogen::{
        color::{color, Color},
        effects::gain_mana,
        empty::Empty,
        keywords::keyword,
        mana::Mana,
        types::{Subtype, Type, Typeline},
    },
};

#[cfg(test)]
mod _tests;

pub mod abilities;
pub mod ai;
pub mod battlefield;
pub mod card;
pub mod cost;
pub mod counters;
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
pub mod targets;
pub mod triggers;
pub mod turns;
pub mod types;
pub mod ui;

pub static FONT_DATA: &[u8] = include_bytes!("../fonts/mana.ttf");

static CARD_DEFINITIONS: Dir = include_dir!("cards");

pub type Cards = IndexMap<String, Card>;

pub fn load_protos() -> anyhow::Result<Vec<(protogen::card::Card, &'static File<'static>)>> {
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

    let mut total_lines = 0;
    let mut results = vec![];
    for card_file in CARD_DEFINITIONS
        .entries()
        .iter()
        .flat_map(|entry| match entry {
            include_dir::DirEntry::Dir(dir) => dir_to_files(dir).into_iter(),
            include_dir::DirEntry::File(file) => vec![file].into_iter(),
        })
    {
        let contents = card_file
            .contents_utf8()
            .ok_or_else(|| anyhow!("Non utf-8 text proto"))?;
        total_lines += contents.lines().count();
        let card: protogen::card::Card = serde_yaml::from_str(contents)
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
                    Source::from(contents),
                ))
                .unwrap();

                anyhow!(e.to_string())
            })
            .with_context(|| format!("Parsing file: {}", card_file.path().display()))?;

        results.push((card, card_file));
    }

    debug!("Loaded {} lines", total_lines);

    Ok(results)
}

pub fn load_cards() -> anyhow::Result<Cards> {
    let timer = std::time::Instant::now();
    let mut cards = Cards::default();
    let protos = load_protos()?;
    for (card, card_file) in protos {
        if cards
            .insert(
                card.name.clone(),
                (&card)
                    .try_into()
                    .with_context(|| format!("Validating file: {}", card_file.path().display()))?,
            )
            .is_some()
        {
            warn!("Overwriting card {}", card.name);
        };
    }

    info!(
        "Loaded {} cards in {}ms",
        cards.len(),
        timer.elapsed().as_millis()
    );

    Ok(cards)
}

fn is_default_value<T: Default + PartialEq>(t: &T) -> bool {
    *t == T::default()
}

fn deserialize_gain_mana<'de, D>(deserializer: D) -> Result<Vec<Mana>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visit;
    impl<'de> Visitor<'de> for Visit {
        type Value = Vec<Mana>;

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

fn parse_mana_symbol<E>(v: &str) -> Result<Vec<Mana>, E>
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
        let mut mana = Mana::default();
        match symbol {
            "W" => mana.set_white(Default::default()),
            "U" => mana.set_blue(Default::default()),
            "B" => mana.set_black(Default::default()),
            "R" => mana.set_red(Default::default()),
            "G" => mana.set_green(Default::default()),
            "C" => mana.set_colorless(Default::default()),
            s => {
                return Err(E::custom(format!("Invalid mana {}", s)));
            }
        }

        results.push(mana);
    }
    Ok(results)
}

fn serialize_gain_mana<S>(value: &[Mana], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let result = mana_to_string(value);

    serializer.serialize_str(&result)
}

fn mana_to_string(value: &[Mana]) -> String {
    let mut result = String::default();
    for mana in value.iter() {
        match mana.mana.as_ref().unwrap() {
            protogen::mana::mana::Mana::White(_) => result.push_str("{W}"),
            protogen::mana::mana::Mana::Blue(_) => result.push_str("{U}"),
            protogen::mana::mana::Mana::Black(_) => result.push_str("{B}"),
            protogen::mana::mana::Mana::Red(_) => result.push_str("{R}"),
            protogen::mana::mana::Mana::Green(_) => result.push_str("{G}"),
            protogen::mana::mana::Mana::Colorless(_) => result.push_str("{C}"),
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
        .sorted()
        .map(|ty| ty.enum_value().unwrap().as_ref().to_case(Case::Title))
        .join(" ");
    let subtypes = value
        .subtypes
        .iter()
        .sorted()
        .map(|ty| ty.enum_value().unwrap().as_ref().to_case(Case::Title))
        .join(" ");

    if subtypes.is_empty() {
        serializer.serialize_str(&types)
    } else {
        serializer.serialize_str(&format!("{} - {}", types, subtypes))
    }
}

fn deserialize_types<'de, D>(deserializer: D) -> Result<HashMap<String, Empty>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visit;
    impl<'de> Visitor<'de> for Visit {
        type Value = HashMap<String, Empty>;

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
                .map(|type_| {
                    type_
                        .map(|type_| (type_.as_ref().to_string(), Empty::default()))
                        .map_err(|e| E::custom(e.to_string()))
                })
                .collect::<Result<Self::Value, E>>()
        }
    }

    deserializer.deserialize_str(Visit)
}

fn serialize_types<S>(value: &HashMap<String, Empty>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(
        &value
            .keys()
            .sorted()
            .map(|ty| ty.to_case(Case::Title))
            .join(", "),
    )
}

fn deserialize_subtypes<'de, D>(deserializer: D) -> Result<HashMap<String, Empty>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visit;
    impl<'de> Visitor<'de> for Visit {
        type Value = HashMap<String, Empty>;

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
                .map(|subtype| {
                    subtype
                        .map(|subtype| (subtype.as_ref().to_string(), Empty::default()))
                        .map_err(|e| E::custom(e.to_string()))
                })
                .collect::<Result<Self::Value, E>>()
        }
    }

    deserializer.deserialize_str(Visit)
}

fn serialize_subtypes<S>(value: &HashMap<String, Empty>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(
        &value
            .keys()
            .sorted()
            .map(|ty| ty.to_case(Case::Title))
            .join(", "),
    )
}

fn deserialize_keywords<'de, D>(deserializer: D) -> Result<HashMap<String, u32>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visit;
    impl<'de> Visitor<'de> for Visit {
        type Value = HashMap<String, u32>;

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
                .map(keyword::Keyword::from_str)
                .map(|keyword| {
                    keyword
                        .map(|kw| kw.as_ref().to_string())
                        .map_err(|e| E::custom(e.to_string()))
                })
            {
                *result.entry(kw?).or_default() += 1;
            }

            Ok(result)
        }
    }

    deserializer.deserialize_str(Visit)
}

fn serialize_keywords<S>(value: &HashMap<String, u32>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(
        &value
            .iter()
            .flat_map(|(kw, count)| std::iter::repeat(kw).take((*count) as usize))
            .sorted()
            .join(", "),
    )
}

fn deserialize_colors<'de, D>(deserializer: D) -> Result<Vec<Color>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visit;
    impl<'de> Visitor<'de> for Visit {
        type Value = Vec<Color>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("expected a comma separate sequence of colors")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            v.split(',')
                .map(|v| v.trim())
                .map(color::Color::from_str)
                .map(|color| {
                    Ok(Color {
                        color: Some(color.map_err(|e| E::custom(e.to_string()))?),
                        ..Default::default()
                    })
                })
                .collect::<Result<Self::Value, E>>()
        }
    }

    deserializer.deserialize_str(Visit)
}

fn serialize_colors<S>(value: &[Color], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(
        &value
            .iter()
            .map(|color| color.color.as_ref().unwrap().as_ref())
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
            formatter.write_str("expected a comma separate sequence of subtypes")
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
