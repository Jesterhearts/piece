#![allow(clippy::single_match)]

#[macro_use]
extern crate tracing;

use std::collections::HashMap;

use anyhow::{anyhow, Context};

use include_dir::{include_dir, Dir, File};

use crate::{
    battlefield::{Battlefield, PendingResults},
    card::Card,
    in_play::CardId,
    ui::{horizontal_list::HorizontalListState, list::ListState, CardSelectionState},
};

#[cfg(test)]
mod _tests;

pub mod abilities;
pub mod ai;
pub mod battlefield;
pub mod card;
pub mod controller;
pub mod cost;
pub mod deck;
pub mod effects;
pub mod in_play;
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

static CARD_DEFINITIONS: Dir = include_dir!("cards");

pub type Cards = HashMap<String, Card>;

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
        let card: protogen::card::Card = protobuf::text_format::parse_from_str(contents)
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
        cards.insert(
            card.name.clone(),
            (&card)
                .try_into()
                .with_context(|| format!("Validating file: {}", card_file.path().display()))?,
        );
    }

    debug!(
        "Loaded {} cards in {}ms",
        cards.len(),
        timer.elapsed().as_millis()
    );

    Ok(cards)
}

#[derive(Debug)]
pub enum UiState {
    Battlefield {
        phase_options_selection_state: HorizontalListState,
        phase_options_list_page: u16,
        selected_state: CardSelectionState,
        action_selection_state: HorizontalListState,
        action_list_page: u16,
        hand_selection_state: HorizontalListState,
        hand_list_page: u16,
        stack_view_state: ListState,
        stack_list_offset: usize,
        player1_mana_list_offset: usize,
        player2_mana_list_offset: usize,
        player1_graveyard_selection_state: ListState,
        player1_graveyard_list_offset: usize,
        player1_exile_selection_state: ListState,
        player1_exile_list_offset: usize,
        player2_graveyard_list_offset: usize,
        player2_exile_list_offset: usize,
    },
    BattlefieldPreview {
        phase_options_selection_state: HorizontalListState,
        phase_options_list_page: u16,
        selected_state: CardSelectionState,
        action_selection_state: HorizontalListState,
        action_list_page: u16,
        hand_selection_state: HorizontalListState,
        hand_list_page: u16,
        stack_view_state: ListState,
        stack_list_offset: usize,
        player1_mana_list_offset: usize,
        player2_mana_list_offset: usize,
        player1_graveyard_selection_state: ListState,
        player1_graveyard_list_offset: usize,
        player1_exile_selection_state: ListState,
        player1_exile_list_offset: usize,
        player2_graveyard_list_offset: usize,
        player2_exile_list_offset: usize,
    },
    SelectingOptions {
        to_resolve: Box<PendingResults>,
        organizing_stack: bool,
        selection_list_state: ListState,
        selection_list_offset: usize,
    },
    ExaminingCard(CardId),
}
