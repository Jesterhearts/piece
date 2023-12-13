#![allow(clippy::single_match)]

use std::{collections::HashMap, io::stdout};

use anyhow::{anyhow, Context};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEventKind, MouseButton,
        MouseEventKind,
    },
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use include_dir::{include_dir, Dir};
use itertools::Itertools;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::{CrosstermBackend, Terminal},
    style::{Style, Stylize},
    widgets::{block::Title, Block, Borders, List, ListItem, ListState},
};

use crate::{
    battlefield::{Battlefield, PendingResults, ResolutionResult},
    card::Card,
    in_play::{CardId, Database},
    player::AllPlayers,
    stack::Stack,
    ui::{horizontal_list::HorizontalListState, CardSelectionState},
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
            card.try_into()
                .with_context(|| format!("Validating file: {}", card_file.path().display()))?,
        );
    }

    Ok(cards)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UiState {
    Battlefield,
    BattlefieldPreview,
    SelectingOptions,
    ExaminingCard(CardId),
}

fn main() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();

    let player = all_players.new_player(20);
    all_players[player].infinite_mana();

    let land1 = CardId::upload(&mut db, &cards, player, "Forest");
    let land2 = CardId::upload(&mut db, &cards, player, "Forest");
    let land3 = CardId::upload(&mut db, &cards, player, "Forest");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, land1, vec![]);
    let _ = Battlefield::add_from_stack_or_hand(&mut db, land2, vec![]);
    let _ = Battlefield::add_from_stack_or_hand(&mut db, land3, vec![]);

    let card1 = CardId::upload(&mut db, &cards, player, "Mace of the Valiant");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card1, vec![]);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, None),
        ResolutionResult::Complete
    );

    let card2 = CardId::upload(&mut db, &cards, player, "Alpine Grizzly");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card2, vec![]);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, None),
        ResolutionResult::Complete
    );

    let card3 = CardId::upload(&mut db, &cards, player, "Elesh Norn, Grand Cenobite");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card3, vec![]);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, None),
        ResolutionResult::Complete
    );

    let card4 = CardId::upload(&mut db, &cards, player, "Abzan Banner");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card4, vec![]);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, None),
        ResolutionResult::Complete
    );

    let card5 = CardId::upload(&mut db, &cards, player, "Allosaurus Shepherd");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card5, vec![]);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, None),
        ResolutionResult::Complete
    );

    let card6 = CardId::upload(&mut db, &cards, player, "Titania, Protector of Argoth");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card6, vec![]);
    let _ = results.resolve(&mut db, &mut all_players, None);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, None),
        ResolutionResult::Complete
    );

    while !Stack::is_empty(&mut db) {
        let results = Stack::resolve_1(&mut db);
        let result = Battlefield::apply_action_results(&mut db, &mut all_players, &results);
        assert_eq!(result, PendingResults::default());
    }

    stdout()
        .execute(EnterAlternateScreen)?
        .execute(EnableMouseCapture)?;
    enable_raw_mode()?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    let mut previous_state = vec![];
    let mut state = UiState::Battlefield;

    let mut last_down = None;
    let mut last_click = None;
    let mut last_hover = None;
    let mut key_selected = None;
    let mut to_resolve: Option<PendingResults> = None;
    let mut choice;

    let mut selected_state = CardSelectionState::default();
    let mut horizontal_list_state = HorizontalListState::default();
    let mut horizontal_list_page = 0;
    let mut selection_list_state = ListState::default();

    loop {
        choice = None;
        terminal.draw(|frame| {
            let mut area = frame.size();

            match state {
                UiState::Battlefield | UiState::BattlefieldPreview => {
                    if matches!(state, UiState::BattlefieldPreview) {
                        let block = Block::default()
                            .title(Title::from(" PREVIEW "))
                            .italic()
                            .borders(Borders::all());
                        area = block.inner(area);
                        frame.render_widget(block, area);
                    }
                    let stack_and_battlefield = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([Constraint::Percentage(12), Constraint::Percentage(88)])
                        .split(area);

                    let battlefield_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(1), Constraint::Length(5)])
                        .split(stack_and_battlefield[1]);

                    let stack_and_mana = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(1), Constraint::Length(12)])
                        .split(stack_and_battlefield[0]);

                    frame.render_widget(
                        List::new(
                            Stack::entries(&mut db)
                                .into_iter()
                                .map(|e| e.display(&mut db))
                                .map(ListItem::new)
                                .interleave_shortest(
                                    std::iter::repeat("━━━━━━━━━━").map(ListItem::new),
                                )
                                .collect_vec(),
                        )
                        .block(Block::default().borders(Borders::ALL).title(" Stack ")),
                        stack_and_mana[0],
                    );
                    frame.render_widget(
                        List::new(
                            all_players[player]
                                .mana_pool
                                .pools_display()
                                .into_iter()
                                .map(ListItem::new)
                                .collect_vec(),
                        )
                        .block(Block::default().borders(Borders::ALL)),
                        stack_and_mana[1],
                    );

                    frame.render_stateful_widget(
                        ui::Battlefield {
                            db: &mut db,
                            owner: player,
                            player_name: " Player ".to_string(),
                            last_hover,
                            last_click,
                        },
                        battlefield_layout[0],
                        &mut selected_state,
                    );

                    frame.render_stateful_widget(
                        ui::SelectedAbilities {
                            db: &mut db,
                            card: selected_state.selected,
                            page: horizontal_list_page,
                        },
                        battlefield_layout[1],
                        &mut horizontal_list_state,
                    );
                }
                UiState::SelectingOptions => {
                    if selection_list_state.selected().is_none() {
                        selection_list_state.select(Some(0));
                    }
                    let options = to_resolve.as_ref().unwrap().options(&mut db, &all_players);
                    if selection_list_state.selected().unwrap_or_default() >= options.len() {
                        selection_list_state.select(Some(options.len() - 1));
                    }

                    frame.render_stateful_widget(
                        List::new(options.into_iter().map(ListItem::new).collect_vec())
                            .highlight_symbol("> ")
                            .highlight_style(Style::new().bold().white())
                            .block(
                                Block::default()
                                    .title(" Select an option ")
                                    .borders(Borders::ALL),
                            ),
                        area,
                        &mut selection_list_state,
                    );
                }
                UiState::ExaminingCard(card) => {
                    let title = card.name(&db);
                    let pt = card.pt_text(&db);
                    frame.render_stateful_widget(
                        ui::Card {
                            db: &mut db,
                            card,
                            title,
                            pt,
                            last_hover: None,
                            last_click: None,
                        },
                        area,
                        &mut CardSelectionState::default(),
                    );
                }
            }
        })?;

        if event::poll(std::time::Duration::from_millis(16))? {
            let event = event::read()?;

            if let event::Event::Resize(_, _) = event {
                last_down = None;
                last_click = None;
                last_hover = None;
            } else if let event::Event::Mouse(mouse) = event {
                if let MouseEventKind::Down(_) = mouse.kind {
                    last_down = Some((mouse.row, mouse.column));
                    last_click = None;
                } else if let MouseEventKind::Up(MouseButton::Left) = mouse.kind {
                    if last_down == Some((mouse.row, mouse.column)) {
                        last_click = Some((mouse.row, mouse.column));
                        key_selected = None;
                    }
                } else if let MouseEventKind::Up(MouseButton::Right) = mouse.kind {
                    if last_down == Some((mouse.row, mouse.column)) {
                        if let Some(hovered) = selected_state.hovered {
                            previous_state.push(state);
                            state = UiState::ExaminingCard(hovered);
                        }
                    }
                } else if let MouseEventKind::Moved = mouse.kind {
                    last_hover = Some((mouse.row, mouse.column));
                }
            } else if let event::Event::Key(key) = event {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    break;
                }
                if key.kind == KeyEventKind::Release {
                    match key.code {
                        KeyCode::Char('1') => {
                            key_selected = Some(0);
                        }
                        KeyCode::Char('2') => {
                            key_selected = Some(1);
                        }
                        KeyCode::Char('3') => {
                            key_selected = Some(2);
                        }
                        KeyCode::Char('4') => {
                            key_selected = Some(3);
                        }
                        KeyCode::Char('5') => {
                            key_selected = Some(4);
                        }
                        KeyCode::Char('6') => {
                            key_selected = Some(5);
                        }
                        KeyCode::Char('7') => {
                            key_selected = Some(6);
                        }
                        KeyCode::Char('8') => {
                            key_selected = Some(7);
                        }
                        KeyCode::Char('9') => {
                            key_selected = Some(8);
                        }
                        KeyCode::Up => {
                            let selected = selection_list_state.selected().unwrap_or_default();
                            selection_list_state.select(Some(selected.saturating_sub(1)));
                        }
                        KeyCode::Down => {
                            let selected = selection_list_state.selected().unwrap_or_default();
                            selection_list_state.select(Some(selected.saturating_add(1)));
                        }
                        KeyCode::Left => {
                            horizontal_list_page = horizontal_list_page.saturating_sub(1);
                        }
                        KeyCode::Right => {
                            if horizontal_list_state.has_overflow {
                                horizontal_list_page += 1;
                            }
                        }
                        KeyCode::Enter => {
                            if to_resolve.is_none() && !Stack::is_empty(&mut db) {
                                let results = Stack::resolve_1(&mut db);
                                let results = Battlefield::apply_action_results(
                                    &mut db,
                                    &mut all_players,
                                    &results,
                                );
                                if !results.is_empty() {
                                    to_resolve = Some(results);
                                    state = UiState::SelectingOptions;
                                }
                            }

                            if matches!(
                                state,
                                UiState::ExaminingCard(_) | UiState::BattlefieldPreview
                            ) {
                                state = previous_state.pop().unwrap_or(UiState::Battlefield);
                            }

                            choice = selection_list_state.selected();
                        }
                        KeyCode::Esc => {
                            if matches!(state, UiState::SelectingOptions) {
                                previous_state.push(state);
                                state = UiState::BattlefieldPreview;
                            } else if !matches!(state, UiState::SelectingOptions) {
                                state = previous_state.pop().unwrap_or(UiState::Battlefield);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        let selected = key_selected.map(|offset| horizontal_list_state.start_index + offset);
        key_selected = None;

        match state {
            UiState::Battlefield => {
                if let Some(card) = selected_state.selected {
                    let abilities = card.activated_abilities(&mut db);
                    if let Some(selected) = selected {
                        if selected < abilities.len() {
                            let mut results = Battlefield::activate_ability(
                                &mut db,
                                &mut all_players,
                                card,
                                selected,
                            );
                            loop {
                                match results.resolve(&mut db, &mut all_players, None) {
                                    battlefield::ResolutionResult::Complete => {
                                        break;
                                    }
                                    battlefield::ResolutionResult::TryAgain => {}
                                    battlefield::ResolutionResult::PendingChoice => {
                                        to_resolve = Some(results);
                                        state = UiState::SelectingOptions;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            UiState::SelectingOptions => loop {
                match to_resolve
                    .as_mut()
                    .unwrap()
                    .resolve(&mut db, &mut all_players, choice)
                {
                    battlefield::ResolutionResult::Complete => {
                        to_resolve = None;
                        state = UiState::Battlefield;
                        break;
                    }
                    battlefield::ResolutionResult::TryAgain => {}
                    battlefield::ResolutionResult::PendingChoice => {
                        break;
                    }
                }
            },
            UiState::ExaminingCard(_) => {}
            UiState::BattlefieldPreview => {}
        }
    }

    stdout()
        .execute(DisableMouseCapture)?
        .execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}
