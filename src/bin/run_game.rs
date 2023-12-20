#[macro_use]
extern crate slog;
extern crate slog_scope;
extern crate slog_stdlog;
extern crate slog_term;
#[macro_use]
extern crate log;

use std::{fs::OpenOptions, io::stdout};

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEventKind, MouseButton,
        MouseEventKind,
    },
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

use itertools::Itertools;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::{CrosstermBackend, Terminal},
    style::Stylize,
    text::Span,
    widgets::{block::Title, Block, Borders, Paragraph},
};
use slog::Drain;

use piece::{
    battlefield::{self, Battlefield, PendingResults, ResolutionResult},
    in_play::{self, CardId, Database, InGraveyard, InHand, OnBattlefield},
    load_cards,
    player::AllPlayers,
    stack::Stack,
    turns::Turn,
    ui::{
        self,
        horizontal_list::{HorizontalList, HorizontalListState},
        list::{List, ListState},
        CardSelectionState,
    },
    UiState,
};

fn main() -> anyhow::Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("logs.log")?;

    let decorator = slog_term::PlainSyncDecorator::new(file);
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let logger = slog::Logger::root(drain, o!());

    // slog_stdlog uses the logger from slog_scope, so set a logger there
    let _guard = slog_scope::set_global_logger(logger);
    slog_stdlog::init()?;

    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();

    let player1 = all_players.new_player("Player 1".to_string(), 20);
    let player2 = all_players.new_player("Player 2".to_string(), 20);
    all_players[player1].infinite_mana();

    let mut turn = Turn::new(&all_players);

    let land1 = CardId::upload(&mut db, &cards, player1, "Forest");
    let land2 = CardId::upload(&mut db, &cards, player1, "Forest");
    let land3 = CardId::upload(&mut db, &cards, player1, "Forest");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, land1, None);
    let _ = Battlefield::add_from_stack_or_hand(&mut db, land2, None);
    let _ = Battlefield::add_from_stack_or_hand(&mut db, land3, None);

    let card2 = CardId::upload(&mut db, &cards, player1, "Alpine Grizzly");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card2, None);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, None),
        ResolutionResult::Complete
    );

    let card3 = CardId::upload(&mut db, &cards, player1, "Elesh Norn, Grand Cenobite");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card3, None);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, None),
        ResolutionResult::Complete
    );

    let card8 = CardId::upload(&mut db, &cards, player1, "Adaptive Gemguard");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card8, None);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, None),
        ResolutionResult::Complete
    );

    let card9 = CardId::upload(&mut db, &cards, player1, "Bat Colony");
    card9.move_to_hand(&mut db);

    let card10 = CardId::upload(&mut db, &cards, player1, "Hidden Courtyard");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card10, None);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, None),
        ResolutionResult::Complete
    );

    while !Stack::is_empty(&mut db) {
        let mut results = Stack::resolve_1(&mut db);
        let result = results.resolve(&mut db, &mut all_players, None);
        assert_eq!(result, ResolutionResult::Complete);
    }

    for card in cards.keys() {
        let card = CardId::upload(&mut db, &cards, player1, card);
        all_players[player1].deck.place_on_top(&mut db, card);
    }
    all_players[player1].deck.shuffle();

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
        stack_view_state: ListState::default(),
        stack_list_offset: 0,
        player1_mana_list_offset: 0,
        player2_mana_list_offset: 0,
        player1_graveyard_selection_state: ListState::default(),
        player1_graveyard_list_offset: 0,
        player2_graveyard_list_offset: 0,
    };

    let mut last_down = None;
    let mut last_click = None;
    let mut last_hover = None;
    let mut key_selected = None;
    let mut last_entry_clicked = None;
    let mut choice;
    let mut selected_card = None;

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
                    stack_view_state,
                    stack_list_offset,
                    player1_mana_list_offset,
                    player2_mana_list_offset,
                    player1_graveyard_selection_state,
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
                    stack_view_state,
                    stack_list_offset,
                    player1_mana_list_offset,
                    player2_mana_list_offset,
                    player1_graveyard_selection_state,
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

                        let phase_options_display = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([Constraint::Min(1), Constraint::Length(30)])
                            .split(phase_options_rest[0]);

                        frame.render_stateful_widget(
                            HorizontalList::new(
                                [
                                    "Pass",
                                    "(Debug) Untap all",
                                    "(Debug) Draw",
                                    "(Debug) infinite mana",
                                ]
                                .into_iter()
                                .enumerate()
                                .map(|(idx, s)| (idx, Span::from(s)))
                                .collect_vec(),
                                last_hover,
                                last_click,
                            )
                            .page(*phase_options_list_page),
                            phase_options_display[0],
                            phase_options_selection_state,
                        );

                        if phase_options_selection_state.has_overflow
                            && phase_options_selection_state.right_clicked
                        {
                            *phase_options_list_page += 1
                        } else if phase_options_selection_state.left_clicked {
                            *phase_options_list_page = phase_options_list_page.saturating_sub(1);
                        }

                        frame.render_widget(
                            Paragraph::new(format!(
                                " {} {}",
                                all_players[turn.active_player()].name,
                                turn.phase.as_ref()
                            )),
                            phase_options_display[1],
                        );

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
                            title: " Mana ".to_string(),
                            items: all_players[player2]
                                .mana_pool
                                .pools_display()
                                .into_iter()
                                .enumerate()
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

                    frame.render_stateful_widget(
                        List {
                            title: " Stack (Enter) ".to_string(),
                            items: Stack::entries(&mut db)
                                .into_iter()
                                .map(|e| format!("({}) {}", e.0, e.1.display(&mut db)))
                                .enumerate()
                                .collect_vec(),
                            last_hover,
                            last_click,
                            offset: *stack_list_offset,
                        },
                        stack_and_mana[1],
                        stack_view_state,
                    );

                    if stack_view_state.selected_up {
                        *stack_list_offset = stack_list_offset.saturating_sub(1);
                    } else if stack_view_state.selected_down {
                        *stack_list_offset += 1;
                    }

                    let mut state = ListState::default();
                    frame.render_stateful_widget(
                        List {
                            title: " Mana ".to_string(),
                            items: all_players[player1]
                                .mana_pool
                                .pools_display()
                                .into_iter()
                                .enumerate()
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
                            player_name: format!(" {} ", all_players[player2].name),
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
                            player_name: format!(" {} ", all_players[player1].name),
                            last_hover,
                            last_click,
                        },
                        battlefield_layout[1],
                        selected_state,
                    );

                    frame.render_stateful_widget(
                        ui::SelectedAbilities {
                            db: &mut db,
                            all_players: &all_players,
                            turn: &turn,
                            card: selected_state.selected.or(selected_card),
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
                                .enumerate()
                                .map(|(idx, card)| (idx, Span::from(card.name(&db))))
                                .collect_vec(),
                            last_hover,
                            last_click,
                        )
                        .page(*hand_list_page)
                        .block(
                            Block::default()
                                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                                .title(" Hand ".to_string()),
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
                            title: " Graveyard ".to_string(),
                            items: player2
                                .get_cards::<InGraveyard>(&mut db)
                                .into_iter()
                                .map(|card| format!("({}) {}", card.id(&db), card.name(&db)))
                                .enumerate()
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

                    frame.render_stateful_widget(
                        List {
                            title: " Graveyard ".to_string(),
                            items: player1
                                .get_cards::<InGraveyard>(&mut db)
                                .into_iter()
                                .map(|card| format!("({}) {}", card.id(&db), card.name(&db)))
                                .enumerate()
                                .collect_vec(),
                            last_click,
                            last_hover,
                            offset: *player1_graveyard_list_offset,
                        },
                        graveyards[1],
                        player1_graveyard_selection_state,
                    );

                    if player1_graveyard_selection_state.selected_up {
                        *player1_graveyard_list_offset =
                            player1_graveyard_list_offset.saturating_sub(1);
                    } else if player1_graveyard_selection_state.selected_down {
                        *player1_graveyard_list_offset += 1;
                    }
                }
                UiState::SelectingOptions {
                    selection_list_offset,
                    selection_list_state,
                    to_resolve,
                    ..
                } => {
                    if selection_list_state.selected_index.is_none() {
                        selection_list_state.selected_index = Some(0);
                    }
                    let mut options = to_resolve.options(&mut db, &all_players);
                    if to_resolve.choices_optional(&db, &all_players) {
                        for option in options.iter_mut() {
                            option.0 += 1
                        }
                        options.insert(0, (0, "None".to_string()));
                    }

                    if selection_list_state.selected_index.unwrap_or_default() >= options.len() {
                        selection_list_state.selected_index = Some(options.len().saturating_sub(1));
                    }

                    frame.render_stateful_widget(
                        List {
                            title: format!(
                                " Select an option for {} ",
                                to_resolve.description(&db)
                            ),
                            items: options,
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
                            player1_graveyard_selection_state:
                                ListState {
                                    hovered_value: Some(hovered),
                                    ..
                                },
                            ..
                        } = &state
                        {
                            let card = player1.get_cards::<InGraveyard>(&mut db)[*hovered];
                            selected_card = Some(card);
                        } else if let UiState::Battlefield {
                            stack_view_state:
                                ListState {
                                    hovered_value: Some(_),
                                    ..
                                },
                            ..
                        } = &state
                        {
                            if !Stack::is_empty(&mut db) {
                                cleanup_stack(&mut db, &mut all_players, &mut state);
                            }
                        } else if let UiState::Battlefield {
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
                            last_entry_clicked = Some(*hovered);
                        } else if let UiState::SelectingOptions {
                            selection_list_state:
                                ListState {
                                    hovered_value: hovered,
                                    ..
                                },
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
                            player1_graveyard_selection_state:
                                ListState {
                                    hovered_value: Some(hovered),
                                    ..
                                },
                            ..
                        } = &state
                        {
                            let card = player1.get_cards::<InGraveyard>(&mut db)[*hovered];
                            previous_state.push(state);
                            state = UiState::ExaminingCard(card);
                        } else if let UiState::Battlefield {
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
                                    ..
                                },
                            ..
                        } = &state
                        {
                            let hovered = *hovered;
                            previous_state.push(state);
                            state = UiState::ExaminingCard(
                                player1.get_cards::<InHand>(&mut db)[hovered],
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
                        player1_graveyard_selection_state:
                            ListState {
                                hovered_value: graveyard_hovered,
                                ..
                            },
                        player1_graveyard_list_offset,
                        ..
                    } = &mut state
                    {
                        if hand_hovered.is_some() {
                            *hand_list_page = hand_list_page.saturating_sub(1);
                        } else if phases_hovered.is_some() {
                            *phase_options_list_page = phase_options_list_page.saturating_sub(1);
                        } else if graveyard_hovered.is_some() {
                            *player1_graveyard_list_offset =
                                player1_graveyard_list_offset.saturating_sub(1);
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
                        player1_graveyard_selection_state:
                            ListState {
                                hovered_value: graveyard_hovered,
                                ..
                            },
                        player1_graveyard_list_offset,
                        ..
                    } = &mut state
                    {
                        if hand_hovered.is_some() && *hand_has_overflow {
                            *hand_list_page += 1;
                        } else if phases_hovered.is_some() && *phases_has_overflow {
                            *phase_options_list_page += 1;
                        } else if graveyard_hovered.is_some() {
                            *player1_graveyard_list_offset += 1;
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
                                        selected_index: Some(selected),
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
                                        selected_index: Some(selected),
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
                            if !matches!(state, UiState::SelectingOptions { .. })
                                && !Stack::is_empty(&mut db)
                            {
                                cleanup_stack(&mut db, &mut all_players, &mut state);
                            } else if matches!(
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
                                    stack_view_state: ListState::default(),
                                    stack_list_offset: 0,
                                    player1_mana_list_offset: 0,
                                    player2_mana_list_offset: 0,
                                    player1_graveyard_selection_state: ListState::default(),
                                    player1_graveyard_list_offset: 0,
                                    player2_graveyard_list_offset: 0,
                                });
                            } else if let UiState::SelectingOptions {
                                selection_list_state:
                                    ListState {
                                        selected_value: selected,
                                        ..
                                    },
                                ..
                            } = state
                            {
                                debug!("Selected {:?}", selected);
                                choice = selected;
                            }
                        }
                        KeyCode::Tab => {
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
                                    stack_view_state: ListState::default(),
                                    stack_list_offset: 0,
                                    player1_mana_list_offset: 0,
                                    player2_mana_list_offset: 0,
                                    player1_graveyard_selection_state: ListState::default(),
                                    player1_graveyard_list_offset: 0,
                                    player2_graveyard_list_offset: 0,
                                };
                            } else {
                                state = previous_state.pop().unwrap_or(UiState::Battlefield {
                                    phase_options_selection_state: HorizontalListState::default(),
                                    phase_options_list_page: 0,
                                    selected_state: CardSelectionState::default(),
                                    action_selection_state: HorizontalListState::default(),
                                    action_list_page: 0,
                                    hand_selection_state: HorizontalListState::default(),
                                    hand_list_page: 0,
                                    stack_view_state: ListState::default(),
                                    stack_list_offset: 0,
                                    player1_mana_list_offset: 0,
                                    player2_mana_list_offset: 0,
                                    player1_graveyard_selection_state: ListState::default(),
                                    player1_graveyard_list_offset: 0,
                                    player2_graveyard_list_offset: 0,
                                });
                            }
                        }
                        KeyCode::Esc => {
                            if let UiState::SelectingOptions { to_resolve, .. } = &mut state {
                                if to_resolve.can_cancel() {
                                    state = previous_state.pop().unwrap_or(UiState::Battlefield {
                                        phase_options_selection_state: HorizontalListState::default(
                                        ),
                                        phase_options_list_page: 0,
                                        selected_state: CardSelectionState::default(),
                                        action_selection_state: HorizontalListState::default(),
                                        action_list_page: 0,
                                        hand_selection_state: HorizontalListState::default(),
                                        hand_list_page: 0,
                                        stack_view_state: ListState::default(),
                                        stack_list_offset: 0,
                                        player1_mana_list_offset: 0,
                                        player2_mana_list_offset: 0,
                                        player1_graveyard_selection_state: ListState::default(),
                                        player1_graveyard_list_offset: 0,
                                        player2_graveyard_list_offset: 0,
                                    });
                                }
                            } else {
                                state = previous_state.pop().unwrap_or(UiState::Battlefield {
                                    phase_options_selection_state: HorizontalListState::default(),
                                    phase_options_list_page: 0,
                                    selected_state: CardSelectionState::default(),
                                    action_selection_state: HorizontalListState::default(),
                                    action_list_page: 0,
                                    hand_selection_state: HorizontalListState::default(),
                                    hand_list_page: 0,
                                    stack_view_state: ListState::default(),
                                    stack_list_offset: 0,
                                    player1_mana_list_offset: 0,
                                    player2_mana_list_offset: 0,
                                    player1_graveyard_selection_state: ListState::default(),
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

        match &mut state {
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
                    if let Some(index) = key_selected
                        .map(|offset| *phases_start_index + offset)
                        .or(last_entry_clicked)
                    {
                        match index {
                            0 => {
                                let mut pending = turn.step(&mut db, &mut all_players);
                                while pending.only_immediate_results(&db, &all_players) {
                                    let result = pending.resolve(&mut db, &mut all_players, None);
                                    if result == ResolutionResult::Complete {
                                        break;
                                    }
                                }
                                Battlefield::check_sba(&mut db);
                                maybe_organize_stack(&mut db, pending, &mut state);
                            }
                            1 => {
                                for card in in_play::cards::<OnBattlefield>(&mut db) {
                                    card.untap(&mut db);
                                }
                            }
                            2 => {
                                let mut pending = all_players[player1].draw(&mut db, 1);
                                while pending.only_immediate_results(&db, &all_players) {
                                    let result = pending.resolve(&mut db, &mut all_players, None);
                                    if result == ResolutionResult::Complete {
                                        break;
                                    }
                                }

                                Battlefield::check_sba(&mut db);
                                maybe_organize_stack(&mut db, pending, &mut state);
                            }
                            3 => {
                                all_players[player1].infinite_mana();
                            }
                            _ => {}
                        }
                    }
                } else if hand_hovered.is_some() {
                    if let Some(selected) = key_selected
                        .map(|offset| *hand_start_index + offset)
                        .or(last_entry_clicked)
                    {
                        let card = player1.get_cards::<InHand>(&mut db)[selected];
                        if turn.can_cast(&mut db, card) {
                            let mut pending = all_players[player1].play_card(&mut db, selected);
                            while pending.only_immediate_results(&db, &all_players) {
                                let result = pending.resolve(&mut db, &mut all_players, None);
                                if result == ResolutionResult::Complete {
                                    break;
                                }
                            }

                            Battlefield::check_sba(&mut db);
                            maybe_organize_stack(&mut db, pending, &mut state);
                        }
                    }
                } else if let Some(card) = selected_state.selected {
                    let abilities = card.activated_abilities(&db);
                    if let Some(selected) = key_selected
                        .map(|offset| *actions_start_index + offset)
                        .or(last_entry_clicked)
                    {
                        if selected < abilities.len() {
                            let mut results = Battlefield::activate_ability(
                                &mut db,
                                &mut all_players,
                                &turn,
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
                                        state = UiState::SelectingOptions {
                                            to_resolve: results,
                                            selection_list_state: ListState::default(),
                                            selection_list_offset: 0,
                                            organizing_stack: false,
                                        };
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            UiState::SelectingOptions {
                to_resolve,
                organizing_stack,
                ..
            } => {
                let mut real_choice = choice;
                #[allow(clippy::unnecessary_unwrap)] // I would if I could
                if to_resolve.choices_optional(&db, &all_players) && choice.is_some() {
                    if choice == Some(0) {
                        real_choice = None
                    } else {
                        real_choice = Some(choice.unwrap() - 1);
                    }
                }
                if choice.is_some() {
                    loop {
                        match to_resolve.resolve(&mut db, &mut all_players, real_choice) {
                            battlefield::ResolutionResult::Complete => {
                                let entries = Stack::entries_unsettled(&mut db)
                                    .into_iter()
                                    .map(|(_, entry)| entry)
                                    .collect_vec();
                                if !*organizing_stack && entries.len() > 1 {
                                    to_resolve.set_organize_stack(entries);
                                    *organizing_stack = true;
                                } else {
                                    state = previous_state.pop().unwrap_or(UiState::Battlefield {
                                        phase_options_selection_state: HorizontalListState::default(
                                        ),
                                        phase_options_list_page: 0,
                                        selected_state: CardSelectionState::default(),
                                        action_selection_state: HorizontalListState::default(),
                                        action_list_page: 0,
                                        hand_selection_state: HorizontalListState::default(),
                                        hand_list_page: 0,
                                        stack_view_state: ListState::default(),
                                        stack_list_offset: 0,
                                        player1_mana_list_offset: 0,
                                        player2_mana_list_offset: 0,
                                        player1_graveyard_selection_state: ListState::default(),
                                        player1_graveyard_list_offset: 0,
                                        player2_graveyard_list_offset: 0,
                                    });
                                }
                                break;
                            }
                            battlefield::ResolutionResult::TryAgain => {
                                debug!("Trying again for {:#?}", to_resolve);
                                if !to_resolve.only_immediate_results(&db, &all_players) {
                                    break;
                                }
                            }
                            battlefield::ResolutionResult::PendingChoice => {
                                break;
                            }
                        }
                        real_choice = None;
                    }
                }
            }
            UiState::ExaminingCard(_) => {}
            UiState::BattlefieldPreview { .. } => {}
        }

        last_entry_clicked = None;
        key_selected = None;
    }

    stdout()
        .execute(DisableMouseCapture)?
        .execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}

fn cleanup_stack(db: &mut Database, all_players: &mut AllPlayers, state: &mut UiState) {
    let mut pending = Stack::resolve_1(db);
    while pending.only_immediate_results(db, all_players) {
        let result = pending.resolve(db, all_players, None);
        if result == ResolutionResult::Complete {
            break;
        }
    }

    Battlefield::check_sba(db);
    maybe_organize_stack(db, pending, state);
}

fn maybe_organize_stack(db: &mut Database, mut pending: PendingResults, state: &mut UiState) {
    if !pending.is_empty() {
        *state = UiState::SelectingOptions {
            to_resolve: pending,
            selection_list_state: ListState::default(),
            organizing_stack: false,
            selection_list_offset: 0,
        };
    } else {
        let entries = Stack::entries_unsettled(db)
            .into_iter()
            .map(|(_, entry)| entry)
            .collect_vec();
        debug!("Stack entries: {:?}", entries);
        if entries.len() > 1 {
            pending.set_organize_stack(entries);
            *state = UiState::SelectingOptions {
                to_resolve: pending,
                selection_list_state: ListState::default(),
                organizing_stack: true,
                selection_list_offset: 0,
            };
        }
    }
}
