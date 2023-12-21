#![allow(clippy::single_match)]

#[allow(unused_imports)]
#[macro_use]
extern crate slog;
extern crate slog_scope;
extern crate slog_stdlog;
extern crate slog_term;
#[macro_use]
extern crate log;

use std::collections::HashMap;

use anyhow::{anyhow, Context};

use include_dir::{include_dir, Dir};

use crate::{
    battlefield::{Battlefield, PendingResults},
    card::Card,
    in_play::CardId,
    ui::{horizontal_list::HorizontalListState, list::ListState, CardSelectionState},
};

#[cfg(test)]
mod _tests;

pub mod abilities;
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

pub fn load_cards() -> anyhow::Result<Cards> {
    let mut cards = Cards::default();
    for card in CARD_DEFINITIONS.entries().iter() {
        let card_file = card
            .as_file()
            .ok_or_else(|| anyhow!("Non-file entry in cards directory"))?;

        let card: protogen::card::Card = protobuf::text_format::parse_from_str(
            card_file
                .contents_utf8()
                .ok_or_else(|| anyhow!("Non utf-8 text proto"))?,
        )
        .with_context(|| format!("Parsing file: {}", card_file.path().display()))?;

        cards.insert(
            card.name.to_owned(),
            (&card)
                .try_into()
                .with_context(|| format!("Validating file: {}", card_file.path().display()))?,
        );
    }

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
        player2_graveyard_list_offset: usize,
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
        player2_graveyard_list_offset: usize,
    },
    SelectingOptions {
        to_resolve: PendingResults,
        organizing_stack: bool,
        selection_list_state: ListState,
        selection_list_offset: usize,
    },
    ExaminingCard(CardId),
}
