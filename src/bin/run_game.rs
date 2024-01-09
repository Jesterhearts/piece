#[macro_use]
extern crate tracing;

use std::{borrow::Cow, fs::OpenOptions};

use approx::ulps_eq;
use egui::{Color32, Label, Layout, Sense, TextEdit};
use itertools::Itertools;
use macroquad::window::next_frame;
use piece::{
    ai::AI,
    battlefield::Battlefields,
    card::{replace_symbols, Card},
    in_play::{CardId, Database},
    library::DeckDefinition,
    load_cards,
    pending_results::{PendingResults, ResolutionResult},
    player::{AllPlayers, Player},
    stack::Stack,
    targets::Location,
    turns::Turn,
    ui::{self, ManaDisplay},
    FONT_DATA,
};
use probly_search::{score::bm25, Index};
use taffy::prelude::*;

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
        .with_writer(non_blocking)
        .init();

    let cards = load_cards()?;

    let mut all_players = AllPlayers::default();

    let player1 = all_players.new_player("Player 1".to_string(), 20);
    let player2 = all_players.new_player("Player 2".to_string(), 20);
    all_players[player1].infinite_mana();

    let mut db = Database::new(all_players);
    let ai = AI::new(player2);

    let mut index = Index::<usize>::new(6);
    for (idx, card) in cards.values().enumerate() {
        index.add_document(
            &[
                |card: &Card| vec![card.name.as_str()],
                |card: &Card| vec![card.cost.cost_string.as_str()],
                |card: &Card| card.keywords.keys().map(|k| k.as_ref()).collect_vec(),
                |card: &Card| card.types.iter().map(|t| t.as_ref()).collect_vec(),
                |card: &Card| card.subtypes.iter().map(|t| t.as_ref()).collect_vec(),
                |card: &Card| vec![card.full_text.as_str()],
            ],
            |title| {
                title
                    .split_whitespace()
                    .map(str::to_ascii_lowercase)
                    .map(Cow::from)
                    .collect_vec()
            },
            idx,
            card,
        );
    }

    let land1 = CardId::upload(&mut db, &cards, player1, "Forest");
    let land2 = CardId::upload(&mut db, &cards, player1, "Forest");
    let land3 = CardId::upload(&mut db, &cards, player1, "Forest");
    let _ = Battlefields::add_from_stack_or_hand(&mut db, land1, None);
    let _ = Battlefields::add_from_stack_or_hand(&mut db, land2, None);
    let _ = Battlefields::add_from_stack_or_hand(&mut db, land3, None);

    let card2 = CardId::upload(&mut db, &cards, player1, "Alpine Grizzly");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, card2, None);
    assert_eq!(results.resolve(&mut db, None), ResolutionResult::Complete);

    let card3 = CardId::upload(&mut db, &cards, player1, "Elesh Norn, Grand Cenobite");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, card3, None);
    assert_eq!(results.resolve(&mut db, None), ResolutionResult::Complete);

    let card8 = CardId::upload(&mut db, &cards, player1, "Might of the Ancestors");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, card8, None);
    assert_eq!(results.resolve(&mut db, None), ResolutionResult::Complete);

    let card9 = CardId::upload(&mut db, &cards, player1, "Bat Colony");
    card9.move_to_hand(&mut db);

    let card10 = CardId::upload(&mut db, &cards, player1, "Ojer Taq, Deepest Foundation");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, card10, None);
    assert_eq!(results.resolve(&mut db, None), ResolutionResult::Complete);

    let card11 = CardId::upload(&mut db, &cards, player1, "Abzan Banner");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, card11, None);
    assert_eq!(results.resolve(&mut db, None), ResolutionResult::Complete);

    let card12 = CardId::upload(&mut db, &cards, player1, "Resplendent Angel");
    let mut results = Battlefields::add_from_stack_or_hand(&mut db, card12, None);
    assert_eq!(results.resolve(&mut db, None), ResolutionResult::Complete);

    let card13 = CardId::upload(&mut db, &cards, player1, "Get Lost");
    card13.move_to_hand(&mut db);

    let card14 = CardId::upload(&mut db, &cards, player1, "Kutzil's Flanker");
    card14.move_to_hand(&mut db);

    while !db.stack.is_empty() {
        let mut results = Stack::resolve_1(&mut db);
        while results.resolve(&mut db, None) != ResolutionResult::Complete {}
    }

    let mut def = DeckDefinition::default();
    for card in cards.keys() {
        def.add_card(card.clone(), 1);
    }
    db.all_players[player1].library = def.build_deck(&mut db, &cards, player1);

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

    egui_macroquad::ui(|ctx| {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "symbols".to_string(),
            egui::FontData::from_static(FONT_DATA),
        );

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(1, "symbols".to_string());

        ctx.set_fonts(fonts)
    });

    loop {
        if db.turn.priority_player() == player2 {
            debug!("Giving ai priority");
            let mut pending = ai.priority(&mut db, &mut PendingResults::default());

            while pending.only_immediate_results(&db) {
                let result = pending.resolve(&mut db, None);
                if result == ResolutionResult::Complete {
                    break;
                }
            }

            maybe_organize_stack(&mut db, pending, &mut to_resolve, &mut organizing_stack);
        }

        egui_macroquad::ui(|ctx| {
            egui::TopBottomPanel::top("Menu").show(ctx, |ui| {
                ui.set_enabled(to_resolve.is_none() && adding_card.is_none());
                ui.with_layout(Layout::left_to_right(egui::Align::Min), |ui| {
                    if ui.button("Pass").clicked()
                        || (ui.is_enabled()
                            && ctx.input(|input| input.key_released(egui::Key::Num1)))
                    {
                        debug!("Passing priority");
                        assert_eq!(db.turn.priority_player(), player1);
                        db.turn.pass_priority();

                        if db.turn.passed_full_round() {
                            let mut pending = Turn::step(&mut db);
                            while pending.only_immediate_results(&db) {
                                let result = pending.resolve(&mut db, None);
                                if result == ResolutionResult::Complete {
                                    break;
                                }
                            }

                            maybe_organize_stack(
                                &mut db,
                                pending,
                                &mut to_resolve,
                                &mut organizing_stack,
                            );
                        }
                    }

                    if ui.button("(Debug) Untap all").clicked()
                        || (ui.is_enabled()
                            && ctx.input(|input| input.key_released(egui::Key::Num2)))
                    {
                        for card in db
                            .battlefield
                            .battlefields
                            .values()
                            .flat_map(|b| b.iter())
                            .copied()
                            .collect_vec()
                        {
                            card.untap(&mut db);
                        }
                    }

                    if ui.button("(Debug) Infinite mana").clicked()
                        || (ui.is_enabled()
                            && ctx.input(|input| input.key_released(egui::Key::Num3)))
                    {
                        db.all_players[player1].infinite_mana();
                    }

                    if ui.button("(Debug) Draw").clicked()
                        || (ui.is_enabled()
                            && ctx.input(|input| input.key_released(egui::Key::Num4)))
                    {
                        let mut pending = Player::draw(&mut db, player1, 1);
                        while pending.only_immediate_results(&db) {
                            let result = pending.resolve(&mut db, None);
                            if result == ResolutionResult::Complete {
                                break;
                            }
                        }

                        maybe_organize_stack(
                            &mut db,
                            pending,
                            &mut to_resolve,
                            &mut organizing_stack,
                        );
                    }

                    if ui.button("(Debug) Add Card to Hand").clicked()
                        || (ui.is_enabled()
                            && ctx.input(|input| input.key_released(egui::Key::Num5)))
                    {
                        adding_card = Some(String::default());
                    }
                });

                ui.with_layout(Layout::left_to_right(egui::Align::Min), |ui| {
                    ui.label(format!(
                        "{} {}",
                        db.all_players[db.turn.active_player()].name,
                        db.turn.phase.as_ref()
                    ));

                    ui.separator();
                    ui.label(format!(
                        "{} ({})",
                        db.all_players[player1].name, db.all_players[player1].life_total
                    ));

                    ui.separator();
                    ui.label(format!(
                        "{} ({})",
                        db.all_players[player2].name, db.all_players[player2].life_total
                    ));
                })
            });

            egui::CentralPanel::default().show(ctx, |ui| {
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
                        items: db.all_players[player2].mana_pool.pools_display(),
                    },
                );

                let pos = tree.layout(stack).unwrap();
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(pos.location.x, row_offset + pos.location.y),
                        egui::vec2(pos.size.width, pos.size.height),
                    ),
                    ui::Stack {
                        items: db
                            .stack
                            .entries()
                            .iter()
                            .rev()
                            .enumerate()
                            .map(|(idx, e)| format!("({}) {}", idx, e.display(&db)))
                            .collect_vec(),
                        left_clicked: &mut left_clicked,
                    },
                );

                if to_resolve.is_none()
                    && (left_clicked.take().is_some()
                        || ctx.input(|input| input.key_released(egui::Key::Enter)))
                {
                    cleanup_stack(&mut db, &mut to_resolve, &mut organizing_stack);
                }

                let pos = tree.layout(player1_mana).unwrap();
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(pos.location.x, row_offset + pos.location.y),
                        egui::vec2(pos.size.width, pos.size.height),
                    ),
                    ManaDisplay {
                        player: player1,
                        items: db.all_players[player1].mana_pool.pools_display(),
                    },
                );

                let mut col_offset = tree.layout(lhs_column).unwrap().size.width;

                let cards = db.battlefield[player2]
                    .iter()
                    .copied()
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
                    inspecting_card = Some(db.battlefield[player2][clicked]);
                }

                let cards = db.battlefield[player1]
                    .iter()
                    .copied()
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
                    selected_card = Some(db.battlefield[player1][clicked]);
                } else if let Some(clicked) = right_clicked.take() {
                    inspecting_card = Some(db.battlefield[player1][clicked]);
                }

                let pos = tree.layout(player1_options).unwrap();
                ui.put(
                    egui::Rect::from_min_size(
                        egui::pos2(col_offset + pos.location.x, row_offset + pos.location.y),
                        egui::vec2(pos.size.width, pos.size.height),
                    ),
                    ui::Actions {
                        db: &mut db,
                        player: player1,
                        card: selected_card,
                        pending: &to_resolve,
                        left_clicked: &mut left_clicked,
                    },
                );

                if let Some(clicked) = left_clicked.take() {
                    let card = selected_card.unwrap();
                    let mut selected_ability = None;
                    if card.is_in_location(&db, Location::Hand)
                        && clicked == 0
                        && Turn::can_cast(&db, card)
                    {
                        let mut pending = Player::play_card(&mut db, player1, card);
                        while pending.only_immediate_results(&db) {
                            let result = pending.resolve(&mut db, None);
                            if result == ResolutionResult::Complete {
                                break;
                            }
                        }

                        maybe_organize_stack(
                            &mut db,
                            pending,
                            &mut to_resolve,
                            &mut organizing_stack,
                        );
                    } else if card.is_in_location(&db, Location::Hand) && Turn::can_cast(&db, card)
                    {
                        selected_ability = Some(clicked - 1);
                    } else {
                        selected_ability = Some(clicked);
                    }

                    if let Some(selected) = selected_ability {
                        if selected < db[card].abilities(&db).len() {
                            let mut pending = Battlefields::activate_ability(
                                &mut db,
                                &to_resolve,
                                player1,
                                card,
                                selected,
                            );

                            while pending.only_immediate_results(&db) {
                                let result = pending.resolve(&mut db, None);
                                if result == ResolutionResult::Complete {
                                    break;
                                }
                            }

                            if let Some(to_resolve) = to_resolve.take() {
                                pending.extend(to_resolve);
                            }

                            maybe_organize_stack(
                                &mut db,
                                pending,
                                &mut to_resolve,
                                &mut organizing_stack,
                            );
                        }
                    }
                }

                let cards = db.hand[player1].iter().copied().collect_vec();
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
                    selected_card = Some(db.hand[player1][clicked]);
                } else if let Some(clicked) = right_clicked.take() {
                    inspecting_card = Some(db.hand[player1][clicked]);
                }

                col_offset += tree.layout(center_column).unwrap().size.width;

                let cards = db.exile[player2]
                    .iter()
                    .map(|card| card.name(&db))
                    .cloned()
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

                if let Some(clicked) = right_clicked.take() {
                    inspecting_card = Some(db.exile[player2][clicked]);
                }

                let cards = db.graveyard[player2]
                    .iter()
                    .map(|card| card.name(&db))
                    .cloned()
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

                if let Some(clicked) = right_clicked.take() {
                    inspecting_card = Some(db.graveyard[player2][clicked]);
                }

                let cards = db.graveyard[player1]
                    .iter()
                    .map(|card| card.name(&db))
                    .cloned()
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

                if let Some(clicked) = right_clicked.take() {
                    inspecting_card = Some(db.graveyard[player1][clicked]);
                }

                let cards = db.exile[player1]
                    .iter()
                    .map(|card| card.name(&db))
                    .cloned()
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

                if let Some(clicked) = right_clicked.take() {
                    inspecting_card = Some(db.exile[player1][clicked]);
                }
            });

            let mut choice: Option<Option<usize>> = None;
            if let Some(resolving) = to_resolve.as_mut() {
                if resolving.priority(&db) == player2 {
                    let mut pending = ai.priority(&mut db, resolving);

                    while pending.only_immediate_results(&db) {
                        let result = pending.resolve(&mut db, None);
                        if result == ResolutionResult::Complete {
                            break;
                        }
                    }

                    maybe_organize_stack(&mut db, pending, &mut to_resolve, &mut organizing_stack);
                } else {
                    let mut open = true;

                    egui::Window::new(resolving.description(&db))
                        .open(&mut open)
                        .show(ctx, |ui| {
                            ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
                                if resolving.choices_optional(&db) && ui.button("None").clicked() {
                                    choice = Some(None);
                                }

                                for (idx, option) in resolving.options(&mut db) {
                                    if ui.button(option).clicked() {
                                        choice = Some(Some(idx));
                                    }
                                }
                            })
                        });

                    if !open || ctx.input(|input| input.key_released(egui::Key::Escape)) {
                        let can_cancel = resolving.can_cancel(&db);
                        debug!("Can cancel {:?} = {}", resolving, can_cancel);
                        if can_cancel {
                            to_resolve = None;
                        }
                    } else if let Some(choice) = choice {
                        loop {
                            match resolving.resolve(&mut db, choice) {
                                ResolutionResult::Complete => {
                                    let mut pending = Battlefields::check_sba(&mut db);
                                    while pending.only_immediate_results(&db) {
                                        let result = pending.resolve(&mut db, None);
                                        if result == ResolutionResult::Complete {
                                            break;
                                        }
                                    }

                                    if pending.is_empty() {
                                        let entries = db.stack.entries_unsettled();
                                        if !organizing_stack && entries.len() > 1 {
                                            resolving.set_organize_stack(&db, entries);
                                            organizing_stack = true;
                                        } else {
                                            debug!("Stepping priority");
                                            db.turn.step_priority();
                                            assert_eq!(db.turn.priority_player(), player2);
                                            debug!("Giving ai priority",);
                                            let pending = ai
                                                .priority(&mut db, &mut PendingResults::default());
                                            maybe_organize_stack(
                                                &mut db,
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
                                    if !resolving.only_immediate_results(&db) {
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
                egui::Window::new(format!(
                    "{} - {}",
                    inspecting.name(&db),
                    inspecting.faceup_face(&db).cost.cost_string
                ))
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.add(ui::Card {
                        db: &mut db,
                        card: inspecting,
                        title: None,
                    });
                });

                if !open || ctx.input(|input| input.key_released(egui::Key::Escape)) {
                    inspecting_card = None;
                }
            }

            if adding_card.is_some() {
                let mut open = true;

                egui::Window::new("Add card to hand")
                    .open(&mut open)
                    .show(ctx, |ui| {
                        let adding = adding_card.as_mut().unwrap();

                        let is_valid = cards.contains_key(adding);
                        let edit = ui.add(
                            TextEdit::singleline(adding)
                                .hint_text("Card name")
                                .text_color(if is_valid {
                                    Color32::GREEN
                                } else {
                                    Color32::WHITE
                                }),
                        );
                        if edit.changed() {
                            *adding = replace_symbols(adding);
                        }

                        let search = index.query(
                            adding,
                            &mut bm25::new(),
                            |title| {
                                title
                                    .split_whitespace()
                                    .map(str::to_ascii_lowercase)
                                    .map(Cow::from)
                                    .collect_vec()
                            },
                            &[1., 1., 0.25, 0.5, 0.5, 0.75],
                        );
                        let top = search.get(0).map(|result| result.key);

                        let mut inspecting = None;
                        let mut clicked = None;
                        for result in search
                            .into_iter()
                            .take(10)
                            .sorted_by(|l, r| {
                                if ulps_eq!(l.score, r.score) {
                                    l.key.cmp(&r.key)
                                } else {
                                    r.score.partial_cmp(&l.score).unwrap()
                                }
                            })
                            .map(|result| result.key)
                        {
                            let label = ui.add(
                                Label::new(format!("•\t{}", cards.get_index(result).unwrap().0))
                                    .sense(Sense::click()),
                            );
                            if label.clicked() {
                                clicked = Some(result);
                            } else if label.clicked_by(egui::PointerButton::Secondary) {
                                inspecting = Some(result);
                            }
                        }

                        if clicked.is_some()
                            || (ui.input(|input| input.key_released(egui::Key::Enter))
                                && (is_valid || top.is_some()))
                        {
                            let adding = if is_valid {
                                &*adding
                            } else if let Some(clicked) = clicked {
                                cards.get_index(clicked).unwrap().0
                            } else {
                                cards.get_index(top.unwrap()).unwrap().0
                            };

                            let card = CardId::upload(&mut db, &cards, player1, adding);
                            card.move_to_hand(&mut db);
                            adding_card = None;
                        } else if let Some(inspecting) = inspecting {
                            let card = CardId::upload(
                                &mut db,
                                &cards,
                                player1,
                                cards.get_index(inspecting).unwrap().0,
                            );
                            inspecting_card = Some(card);
                        }
                        edit.request_focus();
                    });

                if !open || ctx.input(|input| input.key_released(egui::Key::Escape)) {
                    adding_card = None;
                }
            }
        });

        egui_macroquad::draw();

        next_frame().await
    }
}

fn cleanup_stack(
    db: &mut Database,
    to_resolve: &mut Option<PendingResults>,
    organizing_stack: &mut bool,
) {
    let mut pending = Stack::resolve_1(db);
    while pending.only_immediate_results(db) {
        let result = pending.resolve(db, None);
        if result == ResolutionResult::Complete {
            break;
        }
    }

    if pending.is_empty() {
        pending = Battlefields::check_sba(db);
        while pending.only_immediate_results(db) {
            let result = pending.resolve(db, None);
            if result == ResolutionResult::Complete {
                break;
            }
        }
    }

    maybe_organize_stack(db, pending, to_resolve, organizing_stack);
}

fn maybe_organize_stack(
    db: &mut Database,
    mut pending: PendingResults,
    to_resolve: &mut Option<PendingResults>,
    organizing_stack: &mut bool,
) {
    if !pending.is_empty() {
        *to_resolve = Some(pending);
        *organizing_stack = false;
    } else {
        let entries = db.stack.entries_unsettled();
        debug!("Stack entries: {:?}", entries);
        if entries.len() > 1 {
            pending.set_organize_stack(db, entries);
            *to_resolve = Some(pending);
            *organizing_stack = true;
        } else {
            *to_resolve = None;
        }
    }
}
