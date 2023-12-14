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
    style::Stylize,
    text::Span,
    widgets::{block::Title, Block, Borders},
};

use crate::{
    battlefield::{Battlefield, PendingResults, ResolutionResult},
    card::Card,
    in_play::{CardId, Database, InGraveyard, InHand},
    player::AllPlayers,
    stack::Stack,
    ui::{
        horizontal_list::{HorizontalList, HorizontalListState},
        list::{List, ListState},
        CardSelectionState,
    },
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

#[derive(Debug)]
enum UiState {
    Battlefield {
        phase_options_selection_state: HorizontalListState,
        phase_options_list_page: u16,
        selected_state: CardSelectionState,
        action_selection_state: HorizontalListState,
        action_list_page: u16,
        hand_selection_state: HorizontalListState,
        hand_list_page: u16,
        stack_list_offset: usize,
        player1_mana_list_offset: usize,
        player2_mana_list_offset: usize,
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
        stack_list_offset: usize,
        player1_mana_list_offset: usize,
        player2_mana_list_offset: usize,
        player1_graveyard_list_offset: usize,
        player2_graveyard_list_offset: usize,
    },
    SelectingOptions {
        selection_list_state: ListState,
        selection_list_offset: usize,
    },
    ExaminingCard(CardId),
}

fn main() -> anyhow::Result<()> {
    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();

    let player1 = all_players.new_player(20);
    let player2 = all_players.new_player(20);
    all_players[player1].infinite_mana();

    let land1 = CardId::upload(&mut db, &cards, player1, "Forest");
    let land2 = CardId::upload(&mut db, &cards, player1, "Forest");
    let land3 = CardId::upload(&mut db, &cards, player1, "Forest");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, land1, vec![]);
    let _ = Battlefield::add_from_stack_or_hand(&mut db, land2, vec![]);
    let _ = Battlefield::add_from_stack_or_hand(&mut db, land3, vec![]);

    let card1 = CardId::upload(&mut db, &cards, player1, "Mace of the Valiant");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card1, vec![]);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, None),
        ResolutionResult::Complete
    );

    let card2 = CardId::upload(&mut db, &cards, player1, "Alpine Grizzly");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card2, vec![]);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, None),
        ResolutionResult::Complete
    );

    let card3 = CardId::upload(&mut db, &cards, player1, "Elesh Norn, Grand Cenobite");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card3, vec![]);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, None),
        ResolutionResult::Complete
    );

    let card4 = CardId::upload(&mut db, &cards, player1, "Abzan Banner");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card4, vec![]);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, None),
        ResolutionResult::Complete
    );

    let card5 = CardId::upload(&mut db, &cards, player1, "Allosaurus Shepherd");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card5, vec![]);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, None),
        ResolutionResult::Complete
    );

    let card6 = CardId::upload(&mut db, &cards, player1, "Titania, Protector of Argoth");
    card6.move_to_hand(&mut db);

    while !Stack::is_empty(&mut db) {
        let mut results = Stack::resolve_1(&mut db);
        let result = results.resolve(&mut db, &mut all_players, None);
        assert_eq!(result, ResolutionResult::Complete);
    }

    stdout()
        .execute(EnterAlternateScreen)?
        .execute(EnableMouseCapture)?;
    enable_raw_mode()?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    let mut previous_state = vec![];
    let mut state = UiState::Battlefield {
        phase_options_selection_state: HorizontalListState::default(),
        phase_options_list_page: 0,
        selected_state: CardSelectionState::default(),
        action_selection_state: HorizontalListState::default(),
        action_list_page: 0,
        hand_selection_state: HorizontalListState::default(),
        hand_list_page: 0,
        stack_list_offset: 0,
        player1_mana_list_offset: 0,
        player2_mana_list_offset: 0,
        player1_graveyard_list_offset: 0,
        player2_graveyard_list_offset: 0,
    };

    let mut last_down = None;
    let mut last_click = None;
    let mut last_hover = None;
    let mut key_selected = None;
    let mut to_resolve: Option<PendingResults> = None;
    let mut choice;

    loop {
        choice = None;
        terminal.draw(|frame| {
            let mut area = frame.size();
            let in_preview = matches!(state, UiState::BattlefieldPreview { .. });

            match &mut state {
                UiState::Battlefield {
                    phase_options_list_page,
                    phase_options_selection_state,
                    selected_state,
                    action_selection_state,
                    action_list_page,
                    hand_selection_state,
                    hand_list_page,
                    stack_list_offset,
                    player1_mana_list_offset,
                    player2_mana_list_offset,
                    player1_graveyard_list_offset,
                    player2_graveyard_list_offset,
                }
                | UiState::BattlefieldPreview {
                    phase_options_list_page,
                    phase_options_selection_state,
                    selected_state,
                    action_selection_state,
                    action_list_page,
                    hand_selection_state,
                    hand_list_page,
                    stack_list_offset,
                    player1_mana_list_offset,
                    player2_mana_list_offset,
                    player1_graveyard_list_offset,
                    player2_graveyard_list_offset,
                } => {
                    if in_preview {
                        let block = Block::default()
                            .title(Title::from(" PREVIEW "))
                            .italic()
                            .borders(Borders::all());
                        area = block.inner(area);
                        frame.render_widget(block, area);
                    } else {
                        let phase_options_rest = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([Constraint::Length(1), Constraint::Min(1)])
                            .split(area);

                        frame.render_stateful_widget(
                            HorizontalList::new(
                                ["Pass"].into_iter().map(Span::from).collect_vec(),
                                last_hover,
                                last_click,
                            )
                            .page(*phase_options_list_page),
                            phase_options_rest[0],
                            phase_options_selection_state,
                        );

                        if phase_options_selection_state.has_overflow
                            && phase_options_selection_state.right_clicked
                        {
                            *phase_options_list_page += 1
                        } else if phase_options_selection_state.left_clicked {
                            *phase_options_list_page = phase_options_list_page.saturating_sub(1);
                        }

                        area = phase_options_rest[1];
                    }

                    let stack_battlefield_graveyard = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(12),
                            Constraint::Percentage(76),
                            Constraint::Percentage(12),
                        ])
                        .split(area);

                    let stack_and_mana = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(10),
                            Constraint::Min(1),
                            Constraint::Length(10),
                        ])
                        .split(stack_battlefield_graveyard[0]);

                    let mut state = ListState::default();
                    frame.render_stateful_widget(
                        List {
                            title: " Mana ".to_owned(),
                            items: all_players[player2]
                                .mana_pool
                                .pools_display()
                                .into_iter()
                                .map(Span::from)
                                .collect_vec(),
                            last_hover,
                            last_click,
                            offset: *player2_mana_list_offset,
                        },
                        stack_and_mana[0],
                        &mut state,
                    );

                    if state.selected_up {
                        *player2_mana_list_offset = player2_mana_list_offset.saturating_sub(1);
                    } else if state.selected_down {
                        *player2_mana_list_offset += 1;
                    }

                    let mut state = ListState::default();
                    frame.render_stateful_widget(
                        List {
                            title: " Stack (Enter) ".to_owned(),
                            items: Stack::entries(&mut db)
                                .into_iter()
                                .map(|e| format!("({}) {}", e.0, e.1.display(&mut db)))
                                .map(Span::from)
                                .collect_vec(),
                            last_hover,
                            last_click,
                            offset: *stack_list_offset,
                        },
                        stack_and_mana[1],
                        &mut state,
                    );

                    if state.selected_up {
                        *stack_list_offset = stack_list_offset.saturating_sub(1);
                    } else if state.selected_down {
                        *stack_list_offset += 1;
                    }

                    let mut state = ListState::default();
                    frame.render_stateful_widget(
                        List {
                            title: " Mana ".to_owned(),
                            items: all_players[player1]
                                .mana_pool
                                .pools_display()
                                .into_iter()
                                .map(Span::from)
                                .collect_vec(),
                            last_hover,
                            last_click,
                            offset: *player1_mana_list_offset,
                        },
                        stack_and_mana[2],
                        &mut state,
                    );

                    if state.selected_up {
                        *player1_mana_list_offset = player1_mana_list_offset.saturating_sub(1);
                    } else if state.selected_down {
                        *player1_mana_list_offset += 1;
                    }

                    let battlefield_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Percentage(40),
                            Constraint::Min(1),
                            Constraint::Length(4),
                            Constraint::Length(2),
                        ])
                        .split(stack_battlefield_graveyard[1]);

                    frame.render_stateful_widget(
                        ui::Battlefield {
                            db: &mut db,
                            owner: player2,
                            player_name: " Player 2 ".to_owned(),
                            last_hover,
                            last_click,
                        },
                        battlefield_layout[0],
                        &mut CardSelectionState::default(),
                    );

                    frame.render_stateful_widget(
                        ui::Battlefield {
                            db: &mut db,
                            owner: player1,
                            player_name: " Player 1 ".to_owned(),
                            last_hover,
                            last_click,
                        },
                        battlefield_layout[1],
                        selected_state,
                    );

                    frame.render_stateful_widget(
                        ui::SelectedAbilities {
                            db: &mut db,
                            card: selected_state.selected,
                            page: *action_list_page,
                            last_hover,
                            last_click,
                        },
                        battlefield_layout[2],
                        action_selection_state,
                    );

                    if action_selection_state.has_overflow && action_selection_state.right_clicked {
                        *action_list_page += 1
                    } else if action_selection_state.left_clicked {
                        *action_list_page = action_list_page.saturating_sub(1);
                    }

                    frame.render_stateful_widget(
                        HorizontalList::new(
                            player1
                                .get_cards::<InHand>(&mut db)
                                .into_iter()
                                .map(|card| card.name(&db))
                                .map(Span::from)
                                .collect_vec(),
                            last_hover,
                            last_click,
                        )
                        .page(*hand_list_page)
                        .block(
                            Block::default()
                                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                                .title(" Hand ".to_owned()),
                        ),
                        battlefield_layout[3],
                        hand_selection_state,
                    );

                    if hand_selection_state.has_overflow && hand_selection_state.right_clicked {
                        *hand_list_page += 1
                    } else if hand_selection_state.left_clicked {
                        *hand_list_page = hand_list_page.saturating_sub(1);
                    }

                    let graveyards = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                        .split(stack_battlefield_graveyard[2]);

                    let mut state = ListState::default();
                    frame.render_stateful_widget(
                        List {
                            title: " Graveyard ".to_owned(),
                            items: player2
                                .get_cards::<InGraveyard>(&mut db)
                                .into_iter()
                                .map(|card| format!("({}) {}", card.id(&db), card.name(&db)))
                                .map(Span::from)
                                .collect_vec(),
                            last_click,
                            last_hover,
                            offset: *player2_graveyard_list_offset,
                        },
                        graveyards[0],
                        &mut state,
                    );

                    if state.selected_up {
                        *player2_graveyard_list_offset =
                            player2_graveyard_list_offset.saturating_sub(1);
                    } else if state.selected_down {
                        *player2_graveyard_list_offset += 1;
                    }

                    let mut state = ListState::default();
                    frame.render_stateful_widget(
                        List {
                            title: " Graveyard ".to_owned(),
                            items: player1
                                .get_cards::<InGraveyard>(&mut db)
                                .into_iter()
                                .map(|card| format!("({}) {}", card.id(&db), card.name(&db)))
                                .map(Span::from)
                                .collect_vec(),
                            last_click,
                            last_hover,
                            offset: *player1_graveyard_list_offset,
                        },
                        graveyards[1],
                        &mut state,
                    );

                    if state.selected_up {
                        *player1_graveyard_list_offset =
                            player1_graveyard_list_offset.saturating_sub(1);
                    } else if state.selected_down {
                        *player1_graveyard_list_offset += 1;
                    }
                }
                UiState::SelectingOptions {
                    selection_list_offset,
                    selection_list_state,
                } => {
                    if selection_list_state.selected.is_none() {
                        selection_list_state.selected = Some(0);
                    }
                    let options = to_resolve.as_ref().unwrap().options(&mut db, &all_players);
                    if selection_list_state.selected.unwrap_or_default() >= options.len() {
                        selection_list_state.selected = Some(options.len() - 1);
                    }

                    frame.render_stateful_widget(
                        List {
                            title: " Select an option ".to_owned(),
                            items: options.into_iter().map(Span::from).collect_vec(),
                            last_click,
                            last_hover,
                            offset: *selection_list_offset,
                        },
                        area,
                        selection_list_state,
                    );

                    if selection_list_state.selected_up {
                        *selection_list_offset = selection_list_offset.saturating_sub(1);
                    } else if selection_list_state.selected_down {
                        *selection_list_offset += 1;
                    }
                }
                UiState::ExaminingCard(card) => {
                    let title = card.name(&db);
                    let pt = card.pt_text(&db);
                    frame.render_stateful_widget(
                        ui::Card {
                            db: &mut db,
                            card: *card,
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

        last_click = None;
        if event::poll(std::time::Duration::from_millis(16))? {
            let event = event::read()?;

            if let event::Event::Mouse(mouse) = event {
                if let MouseEventKind::Down(_) = mouse.kind {
                    last_down = Some((mouse.row, mouse.column));
                } else if let MouseEventKind::Up(MouseButton::Left) = mouse.kind {
                    if last_down == Some((mouse.row, mouse.column)) {
                        last_click = Some((mouse.row, mouse.column));
                        if let UiState::Battlefield {
                            action_selection_state:
                                HorizontalListState {
                                    hovered: Some(hovered),
                                    ..
                                },
                            ..
                        }
                        | UiState::Battlefield {
                            hand_selection_state:
                                HorizontalListState {
                                    hovered: Some(hovered),
                                    ..
                                },
                            ..
                        }
                        | UiState::Battlefield {
                            phase_options_selection_state:
                                HorizontalListState {
                                    hovered: Some(hovered),
                                    ..
                                },
                            ..
                        } = &state
                        {
                            key_selected = Some(*hovered)
                        } else if let UiState::SelectingOptions {
                            selection_list_state: ListState { hovered, .. },
                            ..
                        } = &state
                        {
                            choice = *hovered;
                            key_selected = None;
                        } else {
                            key_selected = None;
                        }
                    }
                } else if let MouseEventKind::Up(MouseButton::Right) = mouse.kind {
                    if last_down == Some((mouse.row, mouse.column)) {
                        if let UiState::Battlefield {
                            selected_state:
                                CardSelectionState {
                                    hovered: Some(hovered),
                                    ..
                                },
                            ..
                        }
                        | UiState::BattlefieldPreview {
                            selected_state:
                                CardSelectionState {
                                    hovered: Some(hovered),
                                    ..
                                },
                            ..
                        } = &state
                        {
                            let hovered = *hovered;
                            previous_state.push(state);
                            state = UiState::ExaminingCard(hovered);
                        } else if let UiState::Battlefield {
                            hand_selection_state:
                                HorizontalListState {
                                    hovered: Some(hovered),
                                    start_index,
                                    ..
                                },
                            ..
                        } = &state
                        {
                            let start_index = *start_index;
                            let hovered = *hovered;
                            previous_state.push(state);
                            state = UiState::ExaminingCard(
                                player1.get_cards::<InHand>(&mut db)[start_index + hovered],
                            );
                        }
                    }
                } else if let MouseEventKind::Moved = mouse.kind {
                    last_hover = Some((mouse.row, mouse.column));
                } else if let MouseEventKind::ScrollUp | MouseEventKind::ScrollLeft = mouse.kind {
                    if let UiState::Battlefield {
                        phase_options_selection_state:
                            HorizontalListState {
                                hovered: phases_hovered,
                                ..
                            },
                        phase_options_list_page,
                        action_list_page,
                        hand_selection_state:
                            HorizontalListState {
                                hovered: hand_hovered,
                                ..
                            },
                        hand_list_page,
                        ..
                    } = &mut state
                    {
                        if hand_hovered.is_some() {
                            *hand_list_page = hand_list_page.saturating_sub(1);
                        } else if phases_hovered.is_some() {
                            *phase_options_list_page = phase_options_list_page.saturating_sub(1);
                        } else {
                            *action_list_page = action_list_page.saturating_sub(1);
                        }
                    };
                } else if let MouseEventKind::ScrollDown | MouseEventKind::ScrollRight = mouse.kind
                {
                    if let UiState::Battlefield {
                        phase_options_selection_state:
                            HorizontalListState {
                                hovered: phases_hovered,
                                has_overflow: phases_has_overflow,
                                ..
                            },
                        phase_options_list_page,
                        action_selection_state:
                            HorizontalListState {
                                has_overflow: actions_has_overflow,
                                ..
                            },
                        action_list_page,
                        hand_selection_state:
                            HorizontalListState {
                                hovered: hand_hovered,
                                has_overflow: hand_has_overflow,
                                ..
                            },
                        hand_list_page,
                        ..
                    } = &mut state
                    {
                        if hand_hovered.is_some() && *hand_has_overflow {
                            *hand_list_page += 1;
                        } else if phases_hovered.is_some() && *phases_has_overflow {
                            *phase_options_list_page += 1;
                        } else if *actions_has_overflow {
                            *action_list_page += 1;
                        }
                    };
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
                            if let UiState::SelectingOptions {
                                selection_list_state:
                                    ListState {
                                        selected: Some(selected),
                                        ..
                                    },
                                ..
                            } = &mut state
                            {
                                *selected = selected.saturating_sub(1);
                            }
                        }
                        KeyCode::Down => {
                            if let UiState::SelectingOptions {
                                selection_list_state:
                                    ListState {
                                        selected: Some(selected),
                                        ..
                                    },
                                ..
                            } = &mut state
                            {
                                *selected += 1;
                            }
                        }
                        KeyCode::Left => {
                            if let UiState::Battlefield {
                                phase_options_selection_state:
                                    HorizontalListState {
                                        hovered: phases_hovered,
                                        ..
                                    },
                                phase_options_list_page,
                                action_list_page,
                                hand_selection_state:
                                    HorizontalListState {
                                        hovered: hand_hovered,
                                        ..
                                    },
                                hand_list_page,
                                ..
                            } = &mut state
                            {
                                if hand_hovered.is_some() {
                                    *hand_list_page = hand_list_page.saturating_sub(1);
                                } else if phases_hovered.is_some() {
                                    *phase_options_list_page =
                                        phase_options_list_page.saturating_sub(1);
                                } else {
                                    *action_list_page = action_list_page.saturating_sub(1);
                                }
                            };
                        }
                        KeyCode::Right => {
                            if let UiState::Battlefield {
                                phase_options_selection_state:
                                    HorizontalListState {
                                        hovered: phases_hovered,
                                        has_overflow: phases_has_overflow,
                                        ..
                                    },
                                phase_options_list_page,
                                action_selection_state:
                                    HorizontalListState {
                                        has_overflow: actions_has_overflow,
                                        ..
                                    },
                                action_list_page,
                                hand_selection_state:
                                    HorizontalListState {
                                        hovered: hand_hovered,
                                        has_overflow: hand_has_overflow,
                                        ..
                                    },
                                hand_list_page,
                                ..
                            } = &mut state
                            {
                                if hand_hovered.is_some() && *hand_has_overflow {
                                    *hand_list_page += 1;
                                } else if phases_hovered.is_some() && *phases_has_overflow {
                                    *phase_options_list_page += 1;
                                } else if *actions_has_overflow {
                                    *action_list_page += 1;
                                }
                            };
                        }
                        KeyCode::Enter => {
                            if to_resolve.is_none() && !Stack::is_empty(&mut db) {
                                let mut results = Stack::resolve_1(&mut db);
                                if results.only_immediate_results() {
                                    let result = results.resolve(&mut db, &mut all_players, None);
                                    assert_eq!(result, ResolutionResult::Complete);
                                } else if !results.is_empty() {
                                    to_resolve = Some(results);
                                    state = UiState::SelectingOptions {
                                        selection_list_state: ListState::default(),
                                        selection_list_offset: 0,
                                    };
                                }
                            }

                            if matches!(
                                state,
                                UiState::ExaminingCard(_) | UiState::BattlefieldPreview { .. }
                            ) {
                                state = previous_state.pop().unwrap_or(UiState::Battlefield {
                                    phase_options_selection_state: HorizontalListState::default(),
                                    phase_options_list_page: 0,
                                    selected_state: CardSelectionState::default(),
                                    action_selection_state: HorizontalListState::default(),
                                    action_list_page: 0,
                                    hand_selection_state: HorizontalListState::default(),
                                    hand_list_page: 0,
                                    stack_list_offset: 0,
                                    player1_mana_list_offset: 0,
                                    player2_mana_list_offset: 0,
                                    player1_graveyard_list_offset: 0,
                                    player2_graveyard_list_offset: 0,
                                });
                            }

                            if let UiState::SelectingOptions {
                                selection_list_state: ListState { selected, .. },
                                ..
                            } = state
                            {
                                choice = selected;
                            }
                        }
                        KeyCode::Esc => {
                            if matches!(state, UiState::SelectingOptions { .. }) {
                                previous_state.push(state);
                                state = UiState::BattlefieldPreview {
                                    phase_options_selection_state: HorizontalListState::default(),
                                    phase_options_list_page: 0,
                                    selected_state: CardSelectionState::default(),
                                    action_selection_state: HorizontalListState::default(),
                                    action_list_page: 0,
                                    hand_selection_state: HorizontalListState::default(),
                                    hand_list_page: 0,
                                    stack_list_offset: 0,
                                    player1_mana_list_offset: 0,
                                    player2_mana_list_offset: 0,
                                    player1_graveyard_list_offset: 0,
                                    player2_graveyard_list_offset: 0,
                                };
                            } else if !matches!(state, UiState::SelectingOptions { .. }) {
                                state = previous_state.pop().unwrap_or(UiState::Battlefield {
                                    phase_options_selection_state: HorizontalListState::default(),
                                    phase_options_list_page: 0,
                                    selected_state: CardSelectionState::default(),
                                    action_selection_state: HorizontalListState::default(),
                                    action_list_page: 0,
                                    hand_selection_state: HorizontalListState::default(),
                                    hand_list_page: 0,
                                    stack_list_offset: 0,
                                    player1_mana_list_offset: 0,
                                    player2_mana_list_offset: 0,
                                    player1_graveyard_list_offset: 0,
                                    player2_graveyard_list_offset: 0,
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        match &state {
            UiState::Battlefield {
                phase_options_selection_state:
                    HorizontalListState {
                        hovered: phases_hovered,
                        start_index: phases_start_index,
                        ..
                    },
                action_selection_state:
                    HorizontalListState {
                        start_index: actions_start_index,
                        ..
                    },
                hand_selection_state:
                    HorizontalListState {
                        hovered: hand_hovered,
                        start_index: hand_start_index,
                        ..
                    },
                selected_state,
                ..
            } => {
                if phases_hovered.is_some() {
                    if let Some(0) = key_selected.map(|offset| phases_start_index + offset) {
                        todo!()
                    }
                } else if hand_hovered.is_some() {
                    if let Some(selected) = key_selected.map(|offset| hand_start_index + offset) {
                        state =
                            UiState::ExaminingCard(player1.get_cards::<InHand>(&mut db)[selected]);
                    }
                } else if let Some(card) = selected_state.selected {
                    let abilities = card.activated_abilities(&mut db);
                    if let Some(selected) = key_selected.map(|offset| actions_start_index + offset)
                    {
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
                                        state = UiState::SelectingOptions {
                                            selection_list_state: ListState::default(),
                                            selection_list_offset: 0,
                                        };
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            UiState::SelectingOptions { .. } => loop {
                match to_resolve
                    .as_mut()
                    .unwrap()
                    .resolve(&mut db, &mut all_players, choice)
                {
                    battlefield::ResolutionResult::Complete => {
                        to_resolve = None;
                        state = UiState::Battlefield {
                            phase_options_selection_state: HorizontalListState::default(),
                            phase_options_list_page: 0,
                            selected_state: CardSelectionState::default(),
                            action_selection_state: HorizontalListState::default(),
                            action_list_page: 0,
                            hand_selection_state: HorizontalListState::default(),
                            hand_list_page: 0,
                            stack_list_offset: 0,
                            player1_mana_list_offset: 0,
                            player2_mana_list_offset: 0,
                            player1_graveyard_list_offset: 0,
                            player2_graveyard_list_offset: 0,
                        };
                        break;
                    }
                    battlefield::ResolutionResult::TryAgain => {}
                    battlefield::ResolutionResult::PendingChoice => {
                        break;
                    }
                }
            },
            UiState::ExaminingCard(_) => {}
            UiState::BattlefieldPreview { .. } => {}
        }

        key_selected = None;
    }

    stdout()
        .execute(DisableMouseCapture)?
        .execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}
