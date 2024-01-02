#![allow(clippy::single_match)]

#[macro_use]
extern crate tracing;

use anyhow::{anyhow, Context};

use ariadne::{Label, Report, ReportKind, Source};
use include_dir::{include_dir, Dir, File};
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::card::Card;

#[cfg(test)]
mod _tests;

pub mod abilities;
pub mod ai;
pub mod battlefield;
pub mod card;
pub mod cost;
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

    debug!(
        "Loaded {} cards in {}ms",
        cards.len(),
        timer.elapsed().as_millis()
    );

    Ok(cards)
}

pub fn is_default_value<T: Default + PartialEq>(t: &T) -> bool {
    *t == T::default()
}

pub fn serialize_message<T, S>(
    value: &::protobuf::MessageField<T>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
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
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(|t| ::protobuf::MessageField::from_option(t))
}
