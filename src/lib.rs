#![allow(clippy::single_match)]

#[macro_use]
extern crate tracing;

use std::str::FromStr;

use anyhow::{anyhow, Context};

use ariadne::{Label, Report, ReportKind, Source};
use include_dir::{include_dir, Dir, File};
use indexmap::IndexMap;
use itertools::Itertools;
use protobuf::{MessageDyn, MessageFull};
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    card::Card,
    protogen::{
        effects::gain_mana,
        mana::Mana,
        types::{subtype, type_, Subtype, Type},
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
pub mod deck;
pub mod effects;
pub mod in_play;
pub mod log;
pub mod mana;
pub mod newtype_enum;
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

pub fn is_default_value<T: Default + PartialEq>(t: &T) -> bool {
    *t == T::default()
}

pub fn deserialize_gain_mana<'de, D>(deserializer: D) -> Result<Vec<Mana>, D::Error>
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

pub fn deserialize_mana_choice<'de, D>(
    deserializer: D,
) -> Result<Vec<gain_mana::GainMana>, D::Error>
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

pub fn deserialize_types<'de, D>(deserializer: D) -> Result<Vec<Type>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visit;
    impl<'de> Visitor<'de> for Visit {
        type Value = Vec<Type>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("expected a comma separate sequence of types")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            v.split(',')
                .map(|v| v.trim())
                .map(type_::Type::from_str)
                .map(|type_| {
                    Ok(Type {
                        type_: Some(type_.map_err(|e| E::custom(e.to_string()))?),
                        ..Default::default()
                    })
                })
                .collect::<Result<Self::Value, E>>()
        }
    }

    deserializer.deserialize_str(Visit)
}

pub fn serialize_types<S>(value: &[Type], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(
        &value
            .iter()
            .map(|ty| ty.type_.as_ref().unwrap().as_ref())
            .join(", "),
    )
}

pub fn deserialize_subtypes<'de, D>(deserializer: D) -> Result<Vec<Subtype>, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visit;
    impl<'de> Visitor<'de> for Visit {
        type Value = Vec<Subtype>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("expected a comma separate sequence of types")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            v.split(',')
                .map(|v| v.trim())
                .map(subtype::Subtype::from_str)
                .map(|subtype| {
                    Ok(Subtype {
                        subtype: Some(subtype.map_err(|e| E::custom(e.to_string()))?),
                        ..Default::default()
                    })
                })
                .collect::<Result<Self::Value, E>>()
        }
    }

    deserializer.deserialize_str(Visit)
}

pub fn serialize_subtypes<S>(value: &[Subtype], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(
        &value
            .iter()
            .map(|ty| ty.subtype.as_ref().unwrap().as_ref())
            .join(", "),
    )
}

pub fn serialize_message<T, S>(
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

pub fn deserialize_message<'de, T, D>(
    deserializer: D,
) -> Result<::protobuf::MessageField<T>, D::Error>
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
