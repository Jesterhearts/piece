#[macro_use]
extern crate tracing;

use std::fs::OpenOptions;

use egui::{Color32, Layout, TextEdit};
use itertools::Itertools;
use macroquad::window::next_frame;
use piece::{
    ai::AI,
    battlefield::{Battlefield, PendingResults, ResolutionResult},
    deck::DeckDefinition,
    in_play::{self, CardId, Database, InExile, InGraveyard, InHand, OnBattlefield},
    load_cards,
    player::AllPlayers,
    stack::Stack,
    turns::Turn,
    ui::{self, ManaDisplay},
};
use taffy::prelude::*;
use tracing_subscriber::fmt::format::FmtSpan;

#[macroquad::main("Piece MTG")]
async fn main() -> anyhow::Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("logs.log")?;

    let (non_blocking, _guard) = tracing_appender::non_blocking(file);
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .pretty()
        .with_ansi(false)
        .with_line_number(true)
        .with_file(true)
        .with_target(false)
        .with_span_events(FmtSpan::ENTER)
        .with_writer(non_blocking)
        .init();

    let cards = load_cards()?;
    let mut db = Database::default();

    let mut all_players = AllPlayers::default();

    let player1 = all_players.new_player("Player 1".to_string(), 20);
    let player2 = all_players.new_player("Player 2".to_string(), 20);
    all_players[player1].infinite_mana();

    let ai = AI::new(player2);

    let mut turn = Turn::new(&mut db, &all_players);

    let land1 = CardId::upload(&mut db, &cards, player1, "Forest");
    let land2 = CardId::upload(&mut db, &cards, player1, "Forest");
    let land3 = CardId::upload(&mut db, &cards, player1, "Forest");
    let _ = Battlefield::add_from_stack_or_hand(&mut db, land1, None);
    let _ = Battlefield::add_from_stack_or_hand(&mut db, land2, None);
    let _ = Battlefield::add_from_stack_or_hand(&mut db, land3, None);

    let card2 = CardId::upload(&mut db, &cards, player1, "Alpine Grizzly");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card2, None);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, &turn, None),
        ResolutionResult::Complete
    );

    let card3 = CardId::upload(&mut db, &cards, player1, "Elesh Norn, Grand Cenobite");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card3, None);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, &turn, None),
        ResolutionResult::Complete
    );

    let card8 = CardId::upload(&mut db, &cards, player1, "Might of the Ancestors");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card8, None);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, &turn, None),
        ResolutionResult::Complete
    );

    let card9 = CardId::upload(&mut db, &cards, player1, "Bat Colony");
    card9.move_to_hand(&mut db);

    let card10 = CardId::upload(&mut db, &cards, player1, "Ojer Taq, Deepest Foundation");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card10, None);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, &turn, None),
        ResolutionResult::Complete
    );

    let card11 = CardId::upload(&mut db, &cards, player1, "Abzan Banner");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card11, None);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, &turn, None),
        ResolutionResult::Complete
    );

    let card12 = CardId::upload(&mut db, &cards, player1, "Resplendent Angel");
    let mut results = Battlefield::add_from_stack_or_hand(&mut db, card12, None);
    assert_eq!(
        results.resolve(&mut db, &mut all_players, &turn, None),
        ResolutionResult::Complete
    );

    let card13 = CardId::upload(&mut db, &cards, player1, "Get Lost");
    card13.move_to_hand(&mut db);

    let card14 = CardId::upload(&mut db, &cards, player1, "Kutzil's Flanker");
    card14.move_to_hand(&mut db);

    while !Stack::is_empty(&mut db) {
        let mut results = Stack::resolve_1(&mut db);
        while results.resolve(&mut db, &mut all_players, &turn, None) != ResolutionResult::Complete
        {
        }
    }

    let mut def = DeckDefinition::default();
    for card in cards.keys() {
        def.add_card(card.clone(), 1);
    }
    all_players[player1].deck = def.build_deck(&mut db, &cards, player1);

    let mut tree = Taffy::default();

    let player2_mana = tree.new_leaf(Style {
        display: Display::Grid,
        size: Size::percent(1.0),
        grid_column: Line::from_span(1),
        grid_row: Line::from_span(1),
        ..Default::default()
    })?;

    let stack = tree.new_leaf(Style {
        display: Display::Grid,
        size: Size::percent(1.0),
        grid_column: Line::from_span(1),
        grid_row: Line::from_span(1),
        ..Default::default()
    })?;

    let player1_mana = tree.new_leaf(Style {
        display: Display::Grid,
        size: Size::percent(1.0),
        grid_column: Line::from_span(1),
        grid_row: Line::from_span(1),
        ..Default::default()
    })?;

    let lhs_column = tree.new_with_children(
        Style {
            display: Display::Grid,
            size: Size::percent(1.0),
            grid_column: Line::from_span(1),
            grid_row: Line::from_span(1),
            grid_template_rows: vec![
                TrackSizingFunction::from_percent(0.20),
                TrackSizingFunction::from_percent(0.50),
                TrackSizingFunction::from_percent(0.30),
            ],
            grid_template_columns: vec![TrackSizingFunction::from_percent(1.0)],
            ..Default::default()
        },
        &[player2_mana, stack, player1_mana],
    )?;

    let player2_battlefield = tree.new_leaf(Style {
        display: Display::Grid,
        size: Size::percent(1.0),
        grid_column: Line::from_span(1),
        grid_row: Line::from_span(1),
        ..Default::default()
    })?;

    let player1_battlefield = tree.new_leaf(Style {
        display: Display::Grid,
        size: Size::percent(1.0),
        grid_column: Line::from_span(1),
        grid_row: Line::from_span(1),
        ..Default::default()
    })?;

    let player1_options = tree.new_leaf(Style {
        display: Display::Grid,
        size: Size::percent(1.0),
        grid_column: Line::from_span(1),
        grid_row: Line::from_span(1),
        ..Default::default()
    })?;

    let player1_hand = tree.new_leaf(Style {
        display: Display::Grid,
        size: Size::percent(1.0),
        grid_column: Line::from_span(1),
        grid_row: Line::from_span(1),
        ..Default::default()
    })?;

    let center_column = tree.new_with_children(
        Style {
            display: Display::Grid,
            size: Size::percent(1.0),
            grid_column: Line::from_span(1),
            grid_row: Line::from_span(1),
            grid_template_rows: vec![
                TrackSizingFunction::from_percent(0.40),
                TrackSizingFunction::from_percent(0.50),
                TrackSizingFunction::from_percent(0.05),
                TrackSizingFunction::from_percent(0.05),
            ],
            grid_template_columns: vec![TrackSizingFunction::from_percent(1.0)],
            ..Default::default()
        },
        &[
            player2_battlefield,
            player1_battlefield,
            player1_options,
            player1_hand,
        ],
    )?;

    let player2_exile = tree.new_leaf(Style {
        display: Display::Grid,
        size: Size::percent(1.0),
        grid_column: Line::from_span(1),
        grid_row: Line::from_span(1),
        ..Default::default()
    })?;

    let player2_graveyard = tree.new_leaf(Style {
        display: Display::Grid,
        size: Size::percent(1.0),
        grid_column: Line::from_span(1),
        grid_row: Line::from_span(1),
        ..Default::default()
    })?;

    let player1_graveyard = tree.new_leaf(Style {
        display: Display::Grid,
        size: Size::percent(1.0),
        grid_column: Line::from_span(1),
        grid_row: Line::from_span(1),
        ..Default::default()
    })?;

    let player1_exile = tree.new_leaf(Style {
        display: Display::Grid,
        size: Size::percent(1.0),
        grid_column: Line::from_span(1),
        grid_row: Line::from_span(1),
        ..Default::default()
    })?;

    let rhs_column = tree.new_with_children(
        Style {
            display: Display::Grid,
            size: Size::percent(1.0),
            grid_column: Line::from_span(1),
            grid_row: Line::from_span(1),
            grid_template_rows: vec![
                TrackSizingFunction::from_percent(0.15),
                TrackSizingFunction::from_percent(0.25),
                TrackSizingFunction::from_percent(0.40),
                TrackSizingFunction::from_percent(0.20),
            ],
            grid_template_columns: vec![TrackSizingFunction::from_percent(1.0)],
            ..Default::default()
        },
        &[
            player2_exile,
            player2_graveyard,
            player1_graveyard,
            player1_exile,
        ],
    )?;

    let root = tree.new_with_children(
        Style {
            display: Display::Grid,
            size: Size::percent(1.0),
            grid_template_rows: vec![TrackSizingFunction::from_percent(1.0)],
            grid_template_columns: vec![
                TrackSizingFunction::from_percent(0.15),
                TrackSizingFunction::from_percent(0.70),
                TrackSizingFunction::from_percent(0.15),
            ],
            ..Default::default()
        },
        &[lhs_column, center_column, rhs_column],
    )?;

    let mut adding_card = None;
    let mut to_resolve = None;
    let mut organizing_stack = false;

    let mut left_clicked = None;
    let mut right_clicked = None;
    let mut selected_card: Option<CardId> = None;
    let mut inspecting_card = None;

    loop {
        if turn.priority_player() == player2 {
            debug!("Giving ai priority");
            let mut pending = ai.priority(
                &mut db,
                &mut all_players,
                &mut turn,
                &mut PendingResults::default(),
            );

            while pending.only_immediate_results(&db, &all_players) {
                let result = pending.resolve(&mut db, &mut all_players, &turn, None);
                if result == ResolutionResult::Complete {
                    break;
                }
            }

            maybe_organize_stack(
                &mut db,
                &turn,
                pending,
                &mut to_resolve,
                &mut organizing_stack,
            );
        }

        egui_macroquad::ui(|ctx| {
            egui::TopBottomPanel::top("Menu").show(ctx, |ui| {
                ui.set_enabled(to_resolve.is_none());
                ui.with_layout(Layout::left_to_right(egui::Align::Min), |ui| {
                    if ui.button("Pass").clicked() {
                        debug!("Passing priority");
                        assert_eq!(turn.priority_player(), player1);
                        turn.pass_priority();

                        if turn.passed_full_round() {
                            let mut pending = turn.step(&mut db, &mut all_players);
                            while pending.only_immediate_results(&db, &all_players) {
                                let result =
                                    pending.resolve(&mut db, &mut all_players, &turn, None);
                                if result == ResolutionResult::Complete {
                                    break;
                                }
                            }

                            maybe_organize_stack(
                                &mut db,
                                &turn,
                                pending,
                                &mut to_resolve,
                                &mut organizing_stack,
                            );
                        }
                    }

                    if ui.button("(Debug) Untap all").clicked() {
                        for card in in_play::cards::<OnBattlefield>(&mut db) {
                            card.untap(&mut db);
                        }
                    }

                    if ui.button("(Debug) Infinite mana").clicked() {
                        all_players[player1].infinite_mana();
                    }

                    if ui.button("(Debug) Draw").clicked() {
                        let mut pending = all_players[player1].draw(&mut db, 1);
                        while pending.only_immediate_results(&db, &all_players) {
                            let result = pending.resolve(&mut db, &mut all_players, &turn, None);
                            if result == ResolutionResult::Complete {
                                break;
                            }
                        }

                        maybe_organize_stack(
                            &mut db,
                            &turn,
                            pending,
                            &mut to_resolve,
                            &mut organizing_stack,
                        );
                    }

                    if ui.button("(Debug) Add Card to Hand").clicked() {
                        adding_card = Some(String::default());
                    }
                });

                ui.with_layout(Layout::left_to_right(egui::Align::Min), |ui| {
                    ui.label(format!(
                        "{} {}",
                        all_players[turn.active_player()].name,
                        turn.phase.as_ref()
                    ));

                    ui.separator();
                    ui.label(format!(
                        "{} ({})",
                        all_players[player1].name, all_players[player1].life_total
                    ));

                    ui.separator();
                    ui.label(format!(
                        "{} ({})",
                        all_players[player2].name, all_players[player2].life_total
                    ));
                })
            });

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.set_enabled(to_resolve.is_none());
                let size = ui.max_rect();
                tree.compute_layout(
                    root,
                    Size {
                        width: AvailableSpace::from_points(size.width()),
                        height: AvailableSpace::from_points(size.height()),
                    },
                )
                .unwrap();

                let row_offset = ui.next_widget_position().y;

                let pos = tree.layout(player2_mana).unwrap();
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(pos.location.x, row_offset + pos.location.y),
                        egui::vec2(pos.size.width, pos.size.height),
                    ),
                    ManaDisplay {
                        player: player2,
                        items: all_players[player2].mana_pool.pools_display(),
                    },
                );

                let pos = tree.layout(stack).unwrap();
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(pos.location.x, row_offset + pos.location.y),
                        egui::vec2(pos.size.width, pos.size.height),
                    ),
                    ui::Stack {
                        items: Stack::entries(&mut db)
                            .into_iter()
                            .map(|e| format!("({}) {}", e.0, e.1.display(&mut db)))
                            .collect_vec(),
                        left_clicked: &mut left_clicked,
                    },
                );

                if left_clicked.take().is_some() {
                    cleanup_stack(
                        &mut db,
                        &mut all_players,
                        &turn,
                        &mut to_resolve,
                        &mut organizing_stack,
                    );
                }

                let pos = tree.layout(player1_mana).unwrap();
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(pos.location.x, row_offset + pos.location.y),
                        egui::vec2(pos.size.width, pos.size.height),
                    ),
                    ManaDisplay {
                        player: player1,
                        items: all_players[player1].mana_pool.pools_display(),
                    },
                );

                let mut col_offset = tree.layout(lhs_column).unwrap().size.width;

                let cards = player2
                    .get_cards::<OnBattlefield>(&mut db)
                    .into_iter()
                    .enumerate()
                    .collect_vec();
                let pos = tree.layout(player2_battlefield).unwrap();
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(col_offset + pos.location.x, row_offset + pos.location.y),
                        egui::vec2(pos.size.width, pos.size.height),
                    ),
                    ui::Battlefield {
                        db: &mut db,
                        player: player2,
                        cards,
                        left_clicked: &mut None,
                        right_clicked: &mut right_clicked,
                    },
                );

                if let Some(clicked) = right_clicked.take() {
                    inspecting_card = Some(player2.get_cards::<OnBattlefield>(&mut db)[clicked]);
                }

                let cards = player1
                    .get_cards::<OnBattlefield>(&mut db)
                    .into_iter()
                    .enumerate()
                    .collect_vec();
                let pos = tree.layout(player1_battlefield).unwrap();
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(col_offset + pos.location.x, row_offset + pos.location.y),
                        egui::vec2(pos.size.width, pos.size.height),
                    ),
                    ui::Battlefield {
                        db: &mut db,
                        player: player1,
                        cards,
                        left_clicked: &mut left_clicked,
                        right_clicked: &mut right_clicked,
                    },
                );

                if let Some(clicked) = left_clicked.take() {
                    selected_card = Some(player1.get_cards::<OnBattlefield>(&mut db)[clicked]);
                } else if let Some(clicked) = right_clicked.take() {
                    inspecting_card = Some(player1.get_cards::<OnBattlefield>(&mut db)[clicked]);
                }

                let pos = tree.layout(player1_options).unwrap();
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(col_offset + pos.location.x, row_offset + pos.location.y),
                        egui::vec2(pos.size.width, pos.size.height),
                    ),
                    ui::Actions {
                        db: &mut db,
                        all_players: &all_players,
                        turn: &turn,
                        player: player1,
                        card: selected_card,
                        left_clicked: &mut left_clicked,
                    },
                );

                if let Some(clicked) = left_clicked.take() {
                    let card = selected_card.unwrap();
                    let mut selected_ability = None;
                    if card.is_in_location::<InHand>(&db)
                        && clicked == 0
                        && turn.can_cast(&mut db, card)
                    {
                        let mut pending = all_players[player1].play_card(&mut db, card);
                        while pending.only_immediate_results(&db, &all_players) {
                            let result = pending.resolve(&mut db, &mut all_players, &turn, None);
                            if result == ResolutionResult::Complete {
                                break;
                            }
                        }

                        maybe_organize_stack(
                            &mut db,
                            &turn,
                            pending,
                            &mut to_resolve,
                            &mut organizing_stack,
                        );
                    } else if card.is_in_location::<InHand>(&db) && turn.can_cast(&mut db, card) {
                        selected_ability = Some(clicked - 1);
                    } else {
                        selected_ability = Some(clicked);
                    }

                    if let Some(selected) = selected_ability {
                        let abilities = card.activated_abilities(&db);

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
                                let result =
                                    pending.resolve(&mut db, &mut all_players, &turn, None);
                                if result == ResolutionResult::Complete {
                                    break;
                                }
                            }

                            maybe_organize_stack(
                                &mut db,
                                &turn,
                                pending,
                                &mut to_resolve,
                                &mut organizing_stack,
                            );
                        }
                    }
                }

                let cards = player1.get_cards::<InHand>(&mut db);
                let pos = tree.layout(player1_hand).unwrap();
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(col_offset + pos.location.x, row_offset + pos.location.y),
                        egui::vec2(pos.size.width, pos.size.height),
                    ),
                    ui::Hand {
                        db: &mut db,
                        owner: player1,
                        cards,
                        left_clicked: &mut left_clicked,
                        right_clicked: &mut right_clicked,
                    },
                );

                if let Some(clicked) = left_clicked.take() {
                    selected_card = Some(player1.get_cards::<InHand>(&mut db)[clicked]);
                } else if let Some(clicked) = right_clicked.take() {
                    inspecting_card = Some(player1.get_cards::<InHand>(&mut db)[clicked]);
                }

                col_offset += tree.layout(center_column).unwrap().size.width;

                let cards = player2
                    .get_cards::<InExile>(&mut db)
                    .into_iter()
                    .map(|card| card.name(&db))
                    .collect_vec();
                let pos = tree.layout(player2_exile).unwrap();
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(col_offset + pos.location.x, row_offset + pos.location.y),
                        egui::vec2(pos.size.width, pos.size.height),
                    ),
                    ui::Exile {
                        player: player2,
                        cards,
                        right_clicked: &mut right_clicked,
                    },
                );

                if let Some(clicked) = right_clicked {
                    inspecting_card = Some(player2.get_cards::<InExile>(&mut db)[clicked]);
                }

                let cards = player2
                    .get_cards::<InGraveyard>(&mut db)
                    .into_iter()
                    .map(|card| card.name(&db))
                    .collect_vec();
                let pos = tree.layout(player2_graveyard).unwrap();
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(col_offset + pos.location.x, row_offset + pos.location.y),
                        egui::vec2(pos.size.width, pos.size.height),
                    ),
                    ui::Graveyard {
                        player: player2,
                        cards,
                        right_clicked: &mut right_clicked,
                    },
                );

                if let Some(clicked) = right_clicked {
                    inspecting_card = Some(player2.get_cards::<InGraveyard>(&mut db)[clicked]);
                }

                let cards = player1
                    .get_cards::<InGraveyard>(&mut db)
                    .into_iter()
                    .map(|card| card.name(&db))
                    .collect_vec();
                let pos = tree.layout(player1_graveyard).unwrap();
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(col_offset + pos.location.x, row_offset + pos.location.y),
                        egui::vec2(pos.size.width, pos.size.height),
                    ),
                    ui::Graveyard {
                        player: player1,
                        cards,
                        right_clicked: &mut right_clicked,
                    },
                );

                if let Some(clicked) = right_clicked {
                    inspecting_card = Some(player1.get_cards::<InGraveyard>(&mut db)[clicked]);
                }

                let cards = player1
                    .get_cards::<InExile>(&mut db)
                    .into_iter()
                    .map(|card| card.name(&db))
                    .collect_vec();
                let pos = tree.layout(player1_exile).unwrap();
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(col_offset + pos.location.x, row_offset + pos.location.y),
                        egui::vec2(pos.size.width, pos.size.height),
                    ),
                    ui::Exile {
                        player: player1,
                        cards,
                        right_clicked: &mut right_clicked,
                    },
                );

                if let Some(clicked) = right_clicked {
                    inspecting_card = Some(player1.get_cards::<InExile>(&mut db)[clicked]);
                }
            });

            let mut choice: Option<Option<usize>> = None;
            if let Some(resolving) = to_resolve.as_mut() {
                if resolving.priority(&db, &all_players, &turn) == player2 {
                    let mut pending = ai.priority(&mut db, &mut all_players, &mut turn, resolving);

                    while pending.only_immediate_results(&db, &all_players) {
                        let result = pending.resolve(&mut db, &mut all_players, &turn, None);
                        if result == ResolutionResult::Complete {
                            break;
                        }
                    }

                    maybe_organize_stack(
                        &mut db,
                        &turn,
                        pending,
                        &mut to_resolve,
                        &mut organizing_stack,
                    );
                } else {
                    let mut open = true;

                    egui::Window::new(resolving.description(&db))
                        .open(&mut open)
                        .show(ctx, |ui| {
                            ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
                                if resolving.choices_optional(&db, &all_players)
                                    && ui.button("None").clicked()
                                {
                                    choice = Some(None);
                                }

                                for (idx, option) in resolving.options(&mut db, &all_players) {
                                    if ui.button(option).clicked() {
                                        choice = Some(Some(idx));
                                    }
                                }
                            })
                        });

                    if !open && resolving.can_cancel() {
                        to_resolve = None;
                    } else if let Some(choice) = choice {
                        loop {
                            match resolving.resolve(&mut db, &mut all_players, &turn, choice) {
                                ResolutionResult::Complete => {
                                    let mut pending = Battlefield::check_sba(&mut db);
                                    while pending.only_immediate_results(&db, &all_players) {
                                        let result =
                                            pending.resolve(&mut db, &mut all_players, &turn, None);
                                        if result == ResolutionResult::Complete {
                                            break;
                                        }
                                    }

                                    if pending.is_empty() {
                                        let entries = Stack::entries_unsettled(&mut db)
                                            .into_iter()
                                            .map(|(_, entry)| entry)
                                            .collect_vec();
                                        if !organizing_stack && entries.len() > 1 {
                                            resolving.set_organize_stack(&db, entries, &turn);
                                            organizing_stack = true;
                                        } else {
                                            debug!("Stepping priority");
                                            turn.step_priority();
                                            assert_eq!(turn.priority_player(), player2);
                                            debug!("Giving ai priority",);
                                            let pending = ai.priority(
                                                &mut db,
                                                &mut all_players,
                                                &mut turn,
                                                &mut PendingResults::default(),
                                            );
                                            maybe_organize_stack(
                                                &mut db,
                                                &turn,
                                                pending,
                                                &mut to_resolve,
                                                &mut organizing_stack,
                                            );
                                        }
                                    } else {
                                        to_resolve = Some(pending);
                                    }

                                    break;
                                }
                                ResolutionResult::TryAgain => {
                                    debug!("Trying again for {:#?}", resolving);
                                    if !resolving.only_immediate_results(&db, &all_players) {
                                        break;
                                    }
                                }
                                ResolutionResult::PendingChoice => {
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            if let Some(inspecting) = inspecting_card {
                let mut open = true;
                egui::Window::new(inspecting.name(&db))
                    .open(&mut open)
                    .show(ctx, |ui| {
                        ui.add(ui::Card {
                            db: &mut db,
                            card: inspecting,
                            title: None,
                        });
                    });

                if !open {
                    inspecting_card = None;
                }
            }

            if adding_card.is_some() {
                egui::Window::new("Add card to hand").show(ctx, |ui| {
                    let adding = adding_card.as_mut().unwrap();
                    let is_valid = cards.contains_key(adding);
                    let edit = ui.add(
                        TextEdit::singleline(adding)
                            .hint_text("Card name")
                            .text_color(if is_valid {
                                Color32::GREEN
                            } else {
                                Color32::RED
                            }),
                    );

                    if edit.lost_focus() {
                        if is_valid {
                            let card = CardId::upload(&mut db, &cards, player1, adding);
                            card.move_to_hand(&mut db);
                        }
                        adding_card = None;
                    }
                    edit.request_focus();
                });
            }
        });

        egui_macroquad::draw();

        next_frame().await
    }
}

fn cleanup_stack(
    db: &mut Database,
    all_players: &mut AllPlayers,
    turn: &Turn,
    to_resolve: &mut Option<PendingResults>,
    organizing_stack: &mut bool,
) {
    let mut pending = Stack::resolve_1(db);
    while pending.only_immediate_results(db, all_players) {
        let result = pending.resolve(db, all_players, turn, None);
        if result == ResolutionResult::Complete {
            break;
        }
    }

    if pending.is_empty() {
        pending = Battlefield::check_sba(db);
        while pending.only_immediate_results(db, all_players) {
            let result = pending.resolve(db, all_players, turn, None);
            if result == ResolutionResult::Complete {
                break;
            }
        }
    }

    maybe_organize_stack(db, turn, pending, to_resolve, organizing_stack);
}

fn maybe_organize_stack(
    db: &mut Database,
    turn: &Turn,
    mut pending: PendingResults,
    to_resolve: &mut Option<PendingResults>,
    organizing_stack: &mut bool,
) {
    if !pending.is_empty() {
        *to_resolve = Some(pending);
        *organizing_stack = false;
    } else {
        let entries = Stack::entries_unsettled(db)
            .into_iter()
            .map(|(_, entry)| entry)
            .collect_vec();
        debug!("Stack entries: {:?}", entries);
        if entries.len() > 1 {
            pending.set_organize_stack(db, entries, turn);
            *to_resolve = Some(pending);
            *organizing_stack = true;
        } else {
            *to_resolve = None;
        }
    }
}
