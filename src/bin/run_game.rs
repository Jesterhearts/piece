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
    ai::AI,
    battlefield::{self, Battlefield, PendingResults, ResolutionResult},
    in_play::{self, CardId, Database, InExile, InGraveyard, InHand, OnBattlefield},
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

#[allow(clippy::large_enum_variant)]
enum UiAction {
    CleanupStack,
    UpdatePushPreviousState(UiState),
}

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

    let ai = AI::new(player2);

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

    let card11 = CardId::upload(&mut db, &cards, player1, "Abzan Banner");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card11, None);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, None),
        ResolutionResult::Complete
    );

    let card12 = CardId::upload(&mut db, &cards, player1, "Dauntless Dismantler");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card12, None);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, None),
        ResolutionResult::Complete
    );

    let card13 = CardId::upload(&mut db, &cards, player1, "Clay-Fired Bricks");
    card13.move_to_hand(&mut db);

    let card14 = CardId::upload(&mut db, &cards, player1, "Cosmium Blast");
    card14.move_to_hand(&mut db);

    while !Stack::is_empty(&mut db) {
        let mut results = Stack::resolve_1(&mut db);
        while results.resolve(&mut db, &mut all_players, None) != ResolutionResult::Complete {}
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
        player1_exile_selection_state: ListState::default(),
        player1_exile_list_offset: 0,
        player2_graveyard_list_offset: 0,
        player2_exile_list_offset: 0,
    };

    let mut last_down = None;
    let mut last_click = None;
    let mut last_rclick = false;
    let mut last_hover = None;
    let mut key_selected = None;
    let mut last_entry_clicked = None;
    let mut choice = None;
    let mut selected_card = None;
    let mut scroll_left_up = false;
    let mut scroll_right_down = false;

    let mut next_action = None;

    loop {
        if event::poll(std::time::Duration::from_millis(16))? {
            let event = event::read()?;

            if let event::Event::Mouse(mouse) = event {
                if let MouseEventKind::Down(_) = mouse.kind {
                    last_down = Some((mouse.row, mouse.column));
                } else if let MouseEventKind::Up(MouseButton::Left) = mouse.kind {
                    if last_down == Some((mouse.row, mouse.column)) {
                        last_click = Some((mouse.row, mouse.column));
                    }
                    last_down = None;
                } else if let MouseEventKind::Up(MouseButton::Right) = mouse.kind {
                    if last_down == Some((mouse.row, mouse.column)) {
                        last_rclick = true;
                    }
                    last_down = None;
                } else if let MouseEventKind::Moved = mouse.kind {
                    last_hover = Some((mouse.row, mouse.column));
                } else if let MouseEventKind::ScrollUp | MouseEventKind::ScrollLeft = mouse.kind {
                    scroll_left_up = true;
                } else if let MouseEventKind::ScrollDown | MouseEventKind::ScrollRight = mouse.kind
                {
                    scroll_right_down = true;
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
                        KeyCode::Enter => {
                            if !matches!(state, UiState::SelectingOptions { .. })
                                && !Stack::is_empty(&mut db)
                            {
                                cleanup_stack(&mut db, &mut all_players, &turn, &mut state);
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
                                    player1_exile_selection_state: ListState::default(),
                                    player1_exile_list_offset: 0,
                                    player1_graveyard_list_offset: 0,
                                    player2_graveyard_list_offset: 0,
                                    player2_exile_list_offset: 0,
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
                                    player1_exile_selection_state: ListState::default(),
                                    player1_exile_list_offset: 0,
                                    player2_graveyard_list_offset: 0,
                                    player2_exile_list_offset: 0,
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
                                    player1_exile_selection_state: ListState::default(),
                                    player1_exile_list_offset: 0,
                                    player2_graveyard_list_offset: 0,
                                    player2_exile_list_offset: 0,
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
                                        player1_exile_selection_state: ListState::default(),
                                        player1_exile_list_offset: 0,
                                        player2_graveyard_list_offset: 0,
                                        player2_exile_list_offset: 0,
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
                                    player1_exile_selection_state: ListState::default(),
                                    player1_exile_list_offset: 0,
                                    player2_graveyard_list_offset: 0,
                                    player2_exile_list_offset: 0,
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

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
                    player1_exile_selection_state,
                    player1_exile_list_offset,
                    player2_graveyard_list_offset,
                    player2_exile_list_offset,
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
                    player1_exile_selection_state,
                    player1_exile_list_offset,
                    player2_graveyard_list_offset,
                    player2_exile_list_offset,
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

                        if let Some(hovered) = phase_options_selection_state.hovered {
                            if last_click.is_some() {
                                last_entry_clicked = Some(hovered);
                            }
                        }

                        frame.render_widget(
                            Paragraph::new(format!(
                                " {} {} ",
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

                    if stack_view_state.hovered_value.is_some() {
                        if scroll_right_down {
                            *stack_list_offset += 1;
                        } else if scroll_left_up {
                            *stack_list_offset = stack_list_offset.saturating_sub(1)
                        } else if last_click.is_some() && !Stack::is_empty(&mut db) {
                            next_action = Some(UiAction::CleanupStack)
                        }
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

                    if state.hovered_value.is_some() {
                        if scroll_right_down {
                            *player1_mana_list_offset += 1;
                        } else if scroll_left_up {
                            *player1_mana_list_offset = player1_mana_list_offset.saturating_sub(1);
                        }
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

                    let mut state = CardSelectionState::default();
                    frame.render_stateful_widget(
                        ui::Battlefield {
                            db: &mut db,
                            owner: player2,
                            player_name: format!(
                                " {} ({}) ",
                                all_players[player2].name, all_players[player2].life_total
                            ),
                            last_hover,
                            last_click,
                        },
                        battlefield_layout[0],
                        &mut state,
                    );
                    if let Some(hovered) = state.hovered {
                        if last_rclick {
                            next_action = Some(UiAction::UpdatePushPreviousState(
                                UiState::ExaminingCard(hovered),
                            ));
                        }
                    }

                    frame.render_stateful_widget(
                        ui::Battlefield {
                            db: &mut db,
                            owner: player1,
                            player_name: format!(
                                " {} ({}) ",
                                all_players[player1].name, all_players[player1].life_total
                            ),
                            last_hover,
                            last_click,
                        },
                        battlefield_layout[1],
                        selected_state,
                    );

                    if let Some(hovered) = selected_state.hovered {
                        if last_rclick {
                            next_action = Some(UiAction::UpdatePushPreviousState(
                                UiState::ExaminingCard(hovered),
                            ));
                        }
                    }

                    frame.render_stateful_widget(
                        ui::SelectedAbilities {
                            db: &mut db,
                            all_players: &all_players,
                            turn: &turn,
                            player: player1,
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

                    if let Some(hovered) = action_selection_state.hovered {
                        if scroll_left_up {
                            *action_list_page = action_list_page.saturating_sub(1);
                        } else if scroll_right_down {
                            *action_list_page += 1;
                        } else if last_click.is_some() {
                            last_entry_clicked = Some(hovered);
                        }
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

                    if let Some(hovered) = hand_selection_state.hovered {
                        if scroll_left_up {
                            *hand_list_page = hand_list_page.saturating_sub(1);
                        } else if scroll_right_down {
                            *hand_list_page += 1;
                        } else if last_rclick {
                            next_action =
                                Some(UiAction::UpdatePushPreviousState(UiState::ExaminingCard(
                                    player1.get_cards::<InHand>(&mut db)[hovered],
                                )));
                        } else if last_click.is_some() {
                            last_entry_clicked = Some(hovered);
                        }
                    }

                    let exile_and_graveyards = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Percentage(10),
                            Constraint::Percentage(30),
                            Constraint::Percentage(40),
                            Constraint::Percentage(20),
                        ])
                        .split(stack_battlefield_graveyard[2]);

                    let mut state = ListState::default();
                    frame.render_stateful_widget(
                        List {
                            title: " Exile ".to_string(),
                            items: player2
                                .get_cards::<InExile>(&mut db)
                                .into_iter()
                                .map(|card| format!("({}) {}", card.id(&db), card.name(&db)))
                                .enumerate()
                                .collect_vec(),
                            last_click,
                            last_hover,
                            offset: *player2_exile_list_offset,
                        },
                        exile_and_graveyards[0],
                        &mut state,
                    );

                    if state.selected_up {
                        *player2_exile_list_offset = player2_exile_list_offset.saturating_sub(1);
                    } else if state.selected_down {
                        *player2_exile_list_offset += 1;
                    }

                    if state.selected_value.is_some() {
                        if scroll_left_up {
                            *player2_exile_list_offset =
                                player2_exile_list_offset.saturating_sub(1);
                        } else if scroll_right_down {
                            *player2_exile_list_offset += 1;
                        }
                    }

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
                        exile_and_graveyards[1],
                        &mut state,
                    );

                    if state.selected_up {
                        *player2_graveyard_list_offset =
                            player2_graveyard_list_offset.saturating_sub(1);
                    } else if state.selected_down {
                        *player2_graveyard_list_offset += 1;
                    }

                    if state.selected_value.is_some() {
                        if scroll_left_up {
                            *player2_graveyard_list_offset =
                                player2_graveyard_list_offset.saturating_sub(1);
                        } else if scroll_right_down {
                            *player2_graveyard_list_offset += 1;
                        }
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
                        exile_and_graveyards[2],
                        player1_graveyard_selection_state,
                    );

                    if player1_graveyard_selection_state.selected_up {
                        *player1_graveyard_list_offset =
                            player1_graveyard_list_offset.saturating_sub(1);
                    } else if player1_graveyard_selection_state.selected_down {
                        *player1_graveyard_list_offset += 1;
                    }

                    if let Some(hovered) = player1_graveyard_selection_state.hovered_value {
                        if scroll_left_up {
                            *player1_graveyard_list_offset =
                                player1_graveyard_list_offset.saturating_sub(1);
                        } else if scroll_right_down {
                            *player1_graveyard_list_offset += 1;
                        } else if last_rclick {
                            let card = player1.get_cards::<InGraveyard>(&mut db)[hovered];
                            next_action = Some(UiAction::UpdatePushPreviousState(
                                UiState::ExaminingCard(card),
                            ))
                        } else if last_click.is_some() {
                            let card = player1.get_cards::<InGraveyard>(&mut db)[hovered];
                            selected_card = Some(card);
                        }
                    }

                    frame.render_stateful_widget(
                        List {
                            title: " Exile ".to_string(),
                            items: player1
                                .get_cards::<InExile>(&mut db)
                                .into_iter()
                                .map(|card| format!("({}) {}", card.id(&db), card.name(&db)))
                                .enumerate()
                                .collect_vec(),
                            last_click,
                            last_hover,
                            offset: *player1_exile_list_offset,
                        },
                        exile_and_graveyards[3],
                        player1_exile_selection_state,
                    );

                    if state.selected_up {
                        *player1_exile_list_offset = player2_exile_list_offset.saturating_sub(1);
                    } else if state.selected_down {
                        *player1_exile_list_offset += 1;
                    }

                    if let Some(hovered) = player1_exile_selection_state.hovered_value {
                        if scroll_left_up {
                            *player1_exile_list_offset =
                                player1_exile_list_offset.saturating_sub(1);
                        } else if scroll_right_down {
                            *player1_exile_list_offset += 1;
                        } else if last_rclick {
                            let card = player1.get_cards::<InExile>(&mut db)[hovered];
                            next_action = Some(UiAction::UpdatePushPreviousState(
                                UiState::ExaminingCard(card),
                            ));
                        }
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

                    if let Some(hovered) = selection_list_state.hovered_value {
                        if last_click.is_some() {
                            choice = Some(hovered);
                        }
                    }
                }
                UiState::ExaminingCard(card) => {
                    let cost = card.cost(&db);

                    let title = if cost.mana_cost.is_empty() {
                        card.name(&db)
                    } else {
                        format!("{} - {}", card.name(&db), cost.text())
                    };
                    let pt = card.pt_text(&db);
                    frame.render_stateful_widget(
                        ui::Card {
                            db: &mut db,
                            card: *card,
                            title,
                            pt,
                            highlight: false,
                            last_hover: None,
                            last_click: None,
                        },
                        area,
                        &mut CardSelectionState::default(),
                    );
                }
            }
        })?;

        if let Some(action) = next_action {
            match action {
                UiAction::CleanupStack => {
                    cleanup_stack(&mut db, &mut all_players, &turn, &mut state);
                }
                UiAction::UpdatePushPreviousState(new_state) => {
                    previous_state.push(state);
                    state = new_state
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
                                turn.pass_priority();
                                if !turn.passed_full_round() {
                                    debug!("Giving ai priority");
                                    let mut pending = ai.priority(
                                        &mut db,
                                        &mut all_players,
                                        &mut turn,
                                        &mut PendingResults::default(),
                                    );

                                    while pending.only_immediate_results(&db, &all_players) {
                                        let result =
                                            pending.resolve(&mut db, &mut all_players, None);
                                        if result == ResolutionResult::Complete {
                                            break;
                                        }
                                    }

                                    maybe_organize_stack(&mut db, &turn, pending, &mut state);
                                } else {
                                    let mut pending = turn.step(&mut db, &mut all_players);

                                    while pending.only_immediate_results(&db, &all_players) {
                                        let result =
                                            pending.resolve(&mut db, &mut all_players, None);
                                        if result == ResolutionResult::Complete {
                                            break;
                                        }
                                    }

                                    maybe_organize_stack(&mut db, &turn, pending, &mut state);
                                }
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

                                maybe_organize_stack(&mut db, &turn, pending, &mut state);
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

                            maybe_organize_stack(&mut db, &turn, pending, &mut state);
                        }
                    }
                } else if let Some(card) = selected_state.selected {
                    let abilities = card.activated_abilities(&db);
                    if let Some(selected) = key_selected
                        .map(|offset| *actions_start_index + offset)
                        .or(last_entry_clicked)
                    {
                        if selected < abilities.len() {
                            let mut pending = Battlefield::activate_ability(
                                &mut db,
                                &mut all_players,
                                &turn,
                                player1,
                                card,
                                selected,
                            );

                            while pending.only_immediate_results(&db, &all_players) {
                                let result = pending.resolve(&mut db, &mut all_players, None);
                                if result == ResolutionResult::Complete {
                                    break;
                                }
                            }

                            maybe_organize_stack(&mut db, &turn, pending, &mut state);
                        }
                    }
                }
            }
            UiState::SelectingOptions {
                to_resolve,
                organizing_stack,
                ..
            } => {
                if turn.priority_player() == player2 {
                    debug!("Giving ai priority");
                    let pending = ai.priority(&mut db, &mut all_players, &mut turn, to_resolve);
                    if pending.is_empty() {
                        maybe_organize_stack(&mut db, &turn, PendingResults::default(), &mut state);
                    } else {
                        *to_resolve = Box::new(pending);
                    }
                } else {
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
                                    let mut pending = Battlefield::check_sba(&mut db);
                                    while pending.only_immediate_results(&db, &all_players) {
                                        let result =
                                            pending.resolve(&mut db, &mut all_players, None);
                                        if result == ResolutionResult::Complete {
                                            break;
                                        }
                                    }

                                    if pending.is_empty() {
                                        let entries = Stack::entries_unsettled(&mut db)
                                            .into_iter()
                                            .map(|(_, entry)| entry)
                                            .collect_vec();
                                        if !*organizing_stack && entries.len() > 1 {
                                            to_resolve.set_organize_stack(&db, entries, &turn);
                                            *organizing_stack = true;
                                        } else {
                                            turn.step_priority();
                                            debug!("Giving ai priority");
                                            let pending = ai.priority(
                                                &mut db,
                                                &mut all_players,
                                                &mut turn,
                                                &mut PendingResults::default(),
                                            );
                                            maybe_organize_stack(
                                                &mut db, &turn, pending, &mut state,
                                            );

                                            state = previous_state.pop().unwrap_or(
                                                UiState::Battlefield {
                                                    phase_options_selection_state:
                                                        HorizontalListState::default(),
                                                    phase_options_list_page: 0,
                                                    selected_state: CardSelectionState::default(),
                                                    action_selection_state:
                                                        HorizontalListState::default(),
                                                    action_list_page: 0,
                                                    hand_selection_state:
                                                        HorizontalListState::default(),
                                                    hand_list_page: 0,
                                                    stack_view_state: ListState::default(),
                                                    stack_list_offset: 0,
                                                    player1_mana_list_offset: 0,
                                                    player2_mana_list_offset: 0,
                                                    player1_graveyard_selection_state:
                                                        ListState::default(),
                                                    player1_graveyard_list_offset: 0,
                                                    player1_exile_selection_state:
                                                        ListState::default(),
                                                    player1_exile_list_offset: 0,
                                                    player2_graveyard_list_offset: 0,
                                                    player2_exile_list_offset: 0,
                                                },
                                            );
                                        }
                                    } else {
                                        *to_resolve = Box::new(pending);
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
            }
            UiState::ExaminingCard(_) => {}
            UiState::BattlefieldPreview { .. } => {}
        }

        last_click = None;
        last_rclick = false;
        key_selected = None;
        last_entry_clicked = None;
        choice = None;
        scroll_left_up = false;
        scroll_right_down = false;
        next_action = None;
    }

    stdout()
        .execute(DisableMouseCapture)?
        .execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;

    Ok(())
}

fn cleanup_stack(
    db: &mut Database,
    all_players: &mut AllPlayers,
    turn: &Turn,
    state: &mut UiState,
) {
    let mut pending = Stack::resolve_1(db);
    while pending.only_immediate_results(db, all_players) {
        let result = pending.resolve(db, all_players, None);
        if result == ResolutionResult::Complete {
            break;
        }
    }

    if pending.is_empty() {
        pending = Battlefield::check_sba(db);
        while pending.only_immediate_results(db, all_players) {
            let result = pending.resolve(db, all_players, None);
            if result == ResolutionResult::Complete {
                break;
            }
        }
    }

    maybe_organize_stack(db, turn, pending, state);
}

fn maybe_organize_stack(
    db: &mut Database,
    turn: &Turn,
    mut pending: PendingResults,
    state: &mut UiState,
) {
    if !pending.is_empty() {
        *state = UiState::SelectingOptions {
            to_resolve: Box::new(pending),
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
            pending.set_organize_stack(db, entries, turn);
            *state = UiState::SelectingOptions {
                to_resolve: Box::new(pending),
                selection_list_state: ListState::default(),
                organizing_stack: true,
                selection_list_offset: 0,
            };
        }
    }
}
