#[macro_use]
extern crate tracing;

use std::{fs::OpenOptions, time::Instant};

use convert_case::{Case, Casing};
use egui::{Color32, Label, Layout, Sense, TextEdit};
use itertools::Itertools;
use piece::{
    ai::AI,
    battlefield::Battlefields,
    card::replace_symbols,
    in_play::{CardId, Database},
    library::DeckDefinition,
    load_cards,
    pending_results::{PendingResults, ResolutionResult},
    player::{AllPlayers, Owner, Player},
    protogen::targets::Location,
    stack::Stack,
    turns::Turn,
    ui::{self, ManaDisplay},
    Cards, FONT_DATA,
};
use taffy::prelude::*;
use tantivy::{
    collector::TopDocs,
    doc,
    query::QueryParser,
    schema::{Field, SchemaBuilder, TextFieldIndexing, TextOptions, STORED, TEXT},
    tokenizer::RegexTokenizer,
    Index, Searcher,
};

struct App {
    cards: Cards,
    database: Database,
    ai: AI,

    player1: Owner,
    player2: Owner,

    searcher: Searcher,
    parser: QueryParser,
    name: Field,

    adding_card: Option<String>,
    to_resolve: Option<PendingResults>,
    organizing_stack: bool,

    left_clicked: Option<usize>,
    right_clicked: Option<usize>,
    selected_card: Option<CardId>,
    inspecting_card: Option<CardId>,
}

impl App {
    #[allow(clippy::too_many_arguments)]
    fn new(
        cc: &eframe::CreationContext,
        cards: Cards,
        database: Database,
        ai: AI,
        player1: Owner,
        player2: Owner,
        searcher: Searcher,
        parser: QueryParser,
        name: Field,
    ) -> Self {
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

        cc.egui_ctx.set_fonts(fonts);

        Self {
            cards,
            database,
            ai,
            player1,
            player2,
            searcher,
            parser,
            name,
            adding_card: None,
            to_resolve: None,
            organizing_stack: false,
            left_clicked: None,
            right_clicked: None,
            selected_card: None,
            inspecting_card: None,
        }
    }
}

fn main() -> anyhow::Result<()> {
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

    let mut database = Database::new(all_players);
    let ai = AI::new(player2);

    let timer = Instant::now();

    let cost_tokenizer = TextFieldIndexing::default().set_tokenizer("cost");
    let cost_options = TextOptions::default().set_indexing_options(cost_tokenizer);

    let oracle_tokenizer = TextFieldIndexing::default().set_tokenizer("oracle_text");
    let oracle_options = TextOptions::default().set_indexing_options(oracle_tokenizer);

    let mut schema = SchemaBuilder::new();
    let name = schema.add_text_field("name", TEXT | STORED);
    let cost = schema.add_text_field("cost", cost_options);
    let keywords = schema.add_text_field("keywords", TEXT);
    let types = schema.add_text_field("types", TEXT);
    let subtypes = schema.add_text_field("subtypes", TEXT);
    let oracle_text = schema.add_text_field("oracle_text", oracle_options);

    let schema = schema.build();

    let index = Index::create_in_ram(schema);
    index
        .tokenizers()
        .register("cost", RegexTokenizer::new(r"[^\w\s]+")?);
    index
        .tokenizers()
        .register("oracle_text", RegexTokenizer::new(r"[^\w\s]+|\w+")?);

    let mut index_writer = index.writer(15_000_000)?;

    for card in cards.values() {
        index_writer.add_document(doc!(
            name => card.name.as_str(),
            cost => card.cost.text(),
            keywords => card.keywords.keys().map(|k| k.to_case(Case::Lower)).join(", "),
            types => card.types.iter().map(|t| t.enum_value().unwrap().as_ref().to_case(Case::Lower)).join(", "),
            subtypes => card.subtypes.iter().map(|t| t.enum_value().unwrap().as_ref().to_case(Case::Lower)).join(", "),
            oracle_text => card.document(),
        ))?;
    }

    index_writer.commit()?;

    info!("Indexed cards in {}ms", timer.elapsed().as_millis());

    let mut def = DeckDefinition::default();
    for card in cards.keys() {
        def.add_card(card.clone(), 1);
    }
    database.all_players[player1].library = def.build_deck(&mut database, &cards, player1);

    let reader = index.reader()?;
    let searcher = reader.searcher();
    let mut parser = QueryParser::for_index(
        &index,
        vec![name, cost, keywords, types, subtypes, oracle_text],
    );
    parser.set_field_boost(name, 10.0);
    parser.set_field_boost(cost, 10.0);
    parser.set_field_fuzzy(name, true, 1, false);
    parser.set_field_fuzzy(cost, true, 0, false);
    parser.set_field_fuzzy(oracle_text, true, 1, false);

    eframe::run_native(
        "Piece MTG",
        eframe::NativeOptions::default(),
        Box::new(move |cc| {
            Box::new(App::new(
                cc, cards, database, ai, player1, player2, searcher, parser, name,
            ))
        }),
    )
    .unwrap();

    Ok(())
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut tree = Taffy::default();

        let player2_mana = tree
            .new_leaf(Style {
                display: Display::Grid,
                size: Size::percent(1.0),
                grid_column: Line::from_span(1),
                grid_row: Line::from_span(1),
                ..Default::default()
            })
            .unwrap();

        let stack = tree
            .new_leaf(Style {
                display: Display::Grid,
                size: Size::percent(1.0),
                grid_column: Line::from_span(1),
                grid_row: Line::from_span(1),
                ..Default::default()
            })
            .unwrap();

        let player1_mana = tree
            .new_leaf(Style {
                display: Display::Grid,
                size: Size::percent(1.0),
                grid_column: Line::from_span(1),
                grid_row: Line::from_span(1),
                ..Default::default()
            })
            .unwrap();

        let lhs_column = tree
            .new_with_children(
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
            )
            .unwrap();

        let player2_battlefield = tree
            .new_leaf(Style {
                display: Display::Grid,
                size: Size::percent(1.0),
                grid_column: Line::from_span(1),
                grid_row: Line::from_span(1),
                ..Default::default()
            })
            .unwrap();

        let player1_battlefield = tree
            .new_leaf(Style {
                display: Display::Grid,
                size: Size::percent(1.0),
                grid_column: Line::from_span(1),
                grid_row: Line::from_span(1),
                ..Default::default()
            })
            .unwrap();

        let player1_options = tree
            .new_leaf(Style {
                display: Display::Grid,
                size: Size::percent(1.0),
                grid_column: Line::from_span(1),
                grid_row: Line::from_span(1),
                ..Default::default()
            })
            .unwrap();

        let player1_hand = tree
            .new_leaf(Style {
                display: Display::Grid,
                size: Size::percent(1.0),
                grid_column: Line::from_span(1),
                grid_row: Line::from_span(1),
                ..Default::default()
            })
            .unwrap();

        let center_column = tree
            .new_with_children(
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
            )
            .unwrap();

        let player2_exile = tree
            .new_leaf(Style {
                display: Display::Grid,
                size: Size::percent(1.0),
                grid_column: Line::from_span(1),
                grid_row: Line::from_span(1),
                ..Default::default()
            })
            .unwrap();

        let player2_graveyard = tree
            .new_leaf(Style {
                display: Display::Grid,
                size: Size::percent(1.0),
                grid_column: Line::from_span(1),
                grid_row: Line::from_span(1),
                ..Default::default()
            })
            .unwrap();

        let player1_graveyard = tree
            .new_leaf(Style {
                display: Display::Grid,
                size: Size::percent(1.0),
                grid_column: Line::from_span(1),
                grid_row: Line::from_span(1),
                ..Default::default()
            })
            .unwrap();

        let player1_exile = tree
            .new_leaf(Style {
                display: Display::Grid,
                size: Size::percent(1.0),
                grid_column: Line::from_span(1),
                grid_row: Line::from_span(1),
                ..Default::default()
            })
            .unwrap();

        let rhs_column = tree
            .new_with_children(
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
            )
            .unwrap();

        let root = tree
            .new_with_children(
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
            )
            .unwrap();

        if self.database.turn.priority_player() == self.player2 {
            debug!("Giving ai priority");
            let mut pending = self
                .ai
                .priority(&mut self.database, &mut PendingResults::default());

            while pending.only_immediate_results(&self.database) {
                let result = pending.resolve(&mut self.database, None);
                if result == ResolutionResult::Complete {
                    break;
                }
            }

            maybe_organize_stack(
                &mut self.database,
                pending,
                &mut self.to_resolve,
                &mut self.organizing_stack,
            );
        }

        egui::TopBottomPanel::top("Menu").show(ctx, |ui| {
            ui.set_enabled(self.to_resolve.is_none() && self.adding_card.is_none());
            ui.with_layout(Layout::left_to_right(egui::Align::Min), |ui| {
                if ui.button("Pass").clicked()
                    || (ui.is_enabled() && ctx.input(|input| input.key_released(egui::Key::Num1)))
                {
                    debug!("Passing priority");
                    assert_eq!(self.database.turn.priority_player(), self.player1);
                    self.database.turn.pass_priority();

                    if self.database.turn.passed_full_round() {
                        let mut pending = Turn::step(&mut self.database);
                        while pending.only_immediate_results(&self.database) {
                            let result = pending.resolve(&mut self.database, None);
                            if result == ResolutionResult::Complete {
                                break;
                            }
                        }

                        maybe_organize_stack(
                            &mut self.database,
                            pending,
                            &mut self.to_resolve,
                            &mut self.organizing_stack,
                        );
                    }
                }

                if ui.button("(Debug) Untap all").clicked()
                    || (ui.is_enabled() && ctx.input(|input| input.key_released(egui::Key::Num2)))
                {
                    for card in self
                        .database
                        .battlefield
                        .battlefields
                        .values()
                        .flat_map(|b| b.iter())
                        .copied()
                        .collect_vec()
                    {
                        card.untap(&mut self.database);
                    }
                }

                if ui.button("(Debug) Infinite mana").clicked()
                    || (ui.is_enabled() && ctx.input(|input| input.key_released(egui::Key::Num3)))
                {
                    self.database.all_players[self.player1].infinite_mana();
                }

                if ui.button("(Debug) Draw").clicked()
                    || (ui.is_enabled() && ctx.input(|input| input.key_released(egui::Key::Num4)))
                {
                    let mut pending = Player::draw(&mut self.database, self.player1, 1);
                    while pending.only_immediate_results(&self.database) {
                        let result = pending.resolve(&mut self.database, None);
                        if result == ResolutionResult::Complete {
                            break;
                        }
                    }

                    maybe_organize_stack(
                        &mut self.database,
                        pending,
                        &mut self.to_resolve,
                        &mut self.organizing_stack,
                    );
                }

                if ui.button("(Debug) Add Card to Hand").clicked()
                    || (ui.is_enabled() && ctx.input(|input| input.key_released(egui::Key::Num5)))
                {
                    self.adding_card = Some(String::default());
                }
            });

            ui.with_layout(Layout::left_to_right(egui::Align::Min), |ui| {
                ui.label(format!(
                    "{} {}",
                    self.database.all_players[self.database.turn.active_player()].name,
                    self.database.turn.phase.as_ref()
                ));

                ui.separator();
                ui.label(format!(
                    "{} ({})",
                    self.database.all_players[self.player1].name,
                    self.database.all_players[self.player1].life_total
                ));

                ui.separator();
                ui.label(format!(
                    "{} ({})",
                    self.database.all_players[self.player2].name,
                    self.database.all_players[self.player2].life_total
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
                    player: self.player2,
                    items: self.database.all_players[self.player2]
                        .mana_pool
                        .pools_display(),
                },
            );

            let pos = tree.layout(stack).unwrap();
            ui.put(
                egui::Rect::from_min_size(
                    egui::pos2(pos.location.x, row_offset + pos.location.y),
                    egui::vec2(pos.size.width, pos.size.height),
                ),
                ui::Stack {
                    items: self
                        .database
                        .stack
                        .entries()
                        .iter()
                        .rev()
                        .enumerate()
                        .map(|(idx, e)| format!("({}) {}", idx, e.display(&self.database)))
                        .collect_vec(),
                    left_clicked: &mut self.left_clicked,
                },
            );

            if self.to_resolve.is_none()
                && (self.left_clicked.take().is_some()
                    || ctx.input(|input| input.key_released(egui::Key::Enter)))
            {
                cleanup_stack(
                    &mut self.database,
                    &mut self.to_resolve,
                    &mut self.organizing_stack,
                );
            }

            let pos = tree.layout(player1_mana).unwrap();
            ui.put(
                egui::Rect::from_min_size(
                    egui::pos2(pos.location.x, row_offset + pos.location.y),
                    egui::vec2(pos.size.width, pos.size.height),
                ),
                ManaDisplay {
                    player: self.player1,
                    items: self.database.all_players[self.player1]
                        .mana_pool
                        .pools_display(),
                },
            );

            let mut col_offset = tree.layout(lhs_column).unwrap().size.width;

            let cards = self.database.battlefield[self.player2]
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
                    db: &mut self.database,
                    player: self.player2,
                    cards,
                    left_clicked: &mut None,
                    right_clicked: &mut self.right_clicked,
                },
            );

            if let Some(clicked) = self.right_clicked.take() {
                self.inspecting_card = Some(self.database.battlefield[self.player2][clicked]);
            }

            let cards = self.database.battlefield[self.player1]
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
                    db: &mut self.database,
                    player: self.player1,
                    cards,
                    left_clicked: &mut self.left_clicked,
                    right_clicked: &mut self.right_clicked,
                },
            );

            if let Some(clicked) = self.left_clicked.take() {
                self.selected_card = Some(self.database.battlefield[self.player1][clicked]);
            } else if let Some(clicked) = self.right_clicked.take() {
                self.inspecting_card = Some(self.database.battlefield[self.player1][clicked]);
            }

            let pos = tree.layout(player1_options).unwrap();
            ui.put(
                egui::Rect::from_min_size(
                    egui::pos2(col_offset + pos.location.x, row_offset + pos.location.y),
                    egui::vec2(pos.size.width, pos.size.height),
                ),
                ui::Actions {
                    db: &mut self.database,
                    player: self.player1,
                    card: self.selected_card,
                    pending: &self.to_resolve,
                    left_clicked: &mut self.left_clicked,
                },
            );

            if let Some(clicked) = self.left_clicked.take() {
                let card = self.selected_card.unwrap();
                let mut selected_ability = None;
                if card.is_in_location(&self.database, Location::IN_HAND)
                    && clicked == 0
                    && Turn::can_cast(&self.database, card)
                {
                    let mut pending = Player::play_card(&mut self.database, self.player1, card);
                    while pending.only_immediate_results(&self.database) {
                        let result = pending.resolve(&mut self.database, None);
                        if result == ResolutionResult::Complete {
                            break;
                        }
                    }

                    maybe_organize_stack(
                        &mut self.database,
                        pending,
                        &mut self.to_resolve,
                        &mut self.organizing_stack,
                    );
                } else if card.is_in_location(&self.database, Location::IN_HAND)
                    && Turn::can_cast(&self.database, card)
                {
                    selected_ability = Some(clicked - 1);
                } else {
                    selected_ability = Some(clicked);
                }

                if let Some(selected) = selected_ability {
                    if selected < self.database[card].abilities(&self.database).len() {
                        let mut pending = Battlefields::activate_ability(
                            &mut self.database,
                            &self.to_resolve,
                            self.player1,
                            card,
                            selected,
                        );

                        while pending.only_immediate_results(&self.database) {
                            let result = pending.resolve(&mut self.database, None);
                            if result == ResolutionResult::Complete {
                                break;
                            }
                        }

                        if let Some(to_resolve) = self.to_resolve.take() {
                            pending.extend(to_resolve);
                        }

                        maybe_organize_stack(
                            &mut self.database,
                            pending,
                            &mut self.to_resolve,
                            &mut self.organizing_stack,
                        );
                    }
                }
            }

            let cards = self.database.hand[self.player1]
                .iter()
                .copied()
                .collect_vec();
            let pos = tree.layout(player1_hand).unwrap();
            ui.put(
                egui::Rect::from_min_size(
                    egui::pos2(col_offset + pos.location.x, row_offset + pos.location.y),
                    egui::vec2(pos.size.width, pos.size.height),
                ),
                ui::Hand {
                    db: &mut self.database,
                    owner: self.player1,
                    cards,
                    left_clicked: &mut self.left_clicked,
                    right_clicked: &mut self.right_clicked,
                },
            );

            if let Some(clicked) = self.left_clicked.take() {
                self.selected_card = Some(self.database.hand[self.player1][clicked]);
            } else if let Some(clicked) = self.right_clicked.take() {
                self.inspecting_card = Some(self.database.hand[self.player1][clicked]);
            }

            col_offset += tree.layout(center_column).unwrap().size.width;

            let cards = self.database.exile[self.player2]
                .iter()
                .map(|card| card.name(&self.database))
                .cloned()
                .collect_vec();
            let pos = tree.layout(player2_exile).unwrap();
            ui.put(
                egui::Rect::from_min_size(
                    egui::pos2(col_offset + pos.location.x, row_offset + pos.location.y),
                    egui::vec2(pos.size.width, pos.size.height),
                ),
                ui::Exile {
                    player: self.player2,
                    cards,
                    right_clicked: &mut self.right_clicked,
                },
            );

            if let Some(clicked) = self.right_clicked.take() {
                self.inspecting_card = Some(self.database.exile[self.player2][clicked]);
            }

            let cards = self.database.graveyard[self.player2]
                .iter()
                .map(|card| card.name(&self.database))
                .cloned()
                .collect_vec();
            let pos = tree.layout(player2_graveyard).unwrap();
            ui.put(
                egui::Rect::from_min_size(
                    egui::pos2(col_offset + pos.location.x, row_offset + pos.location.y),
                    egui::vec2(pos.size.width, pos.size.height),
                ),
                ui::Graveyard {
                    player: self.player2,
                    cards,
                    right_clicked: &mut self.right_clicked,
                },
            );

            if let Some(clicked) = self.right_clicked.take() {
                self.inspecting_card = Some(self.database.graveyard[self.player2][clicked]);
            }

            let cards = self.database.graveyard[self.player1]
                .iter()
                .map(|card| card.name(&self.database))
                .cloned()
                .collect_vec();
            let pos = tree.layout(player1_graveyard).unwrap();
            ui.put(
                egui::Rect::from_min_size(
                    egui::pos2(col_offset + pos.location.x, row_offset + pos.location.y),
                    egui::vec2(pos.size.width, pos.size.height),
                ),
                ui::Graveyard {
                    player: self.player1,
                    cards,
                    right_clicked: &mut self.right_clicked,
                },
            );

            if let Some(clicked) = self.right_clicked.take() {
                self.inspecting_card = Some(self.database.graveyard[self.player1][clicked]);
            }

            let cards = self.database.exile[self.player1]
                .iter()
                .map(|card| card.name(&self.database))
                .cloned()
                .collect_vec();
            let pos = tree.layout(player1_exile).unwrap();
            ui.put(
                egui::Rect::from_min_size(
                    egui::pos2(col_offset + pos.location.x, row_offset + pos.location.y),
                    egui::vec2(pos.size.width, pos.size.height),
                ),
                ui::Exile {
                    player: self.player1,
                    cards,
                    right_clicked: &mut self.right_clicked,
                },
            );

            if let Some(clicked) = self.right_clicked.take() {
                self.inspecting_card = Some(self.database.exile[self.player1][clicked]);
            }
        });

        let mut choice: Option<Option<usize>> = None;
        if let Some(resolving) = self.to_resolve.as_mut() {
            if resolving.priority(&self.database) == self.player2 {
                let mut pending = self.ai.priority(&mut self.database, resolving);

                while pending.only_immediate_results(&self.database) {
                    let result = pending.resolve(&mut self.database, None);
                    if result == ResolutionResult::Complete {
                        break;
                    }
                }

                maybe_organize_stack(
                    &mut self.database,
                    pending,
                    &mut self.to_resolve,
                    &mut self.organizing_stack,
                );
            } else {
                let mut open = true;

                egui::Window::new(resolving.description(&self.database))
                    .open(&mut open)
                    .show(ctx, |ui| {
                        ui.with_layout(Layout::top_down(egui::Align::Min), |ui| {
                            if resolving.choices_optional(&self.database)
                                && ui.button("None").clicked()
                            {
                                choice = Some(None);
                            }

                            for (idx, option) in resolving.options(&mut self.database) {
                                if ui.button(option).clicked() {
                                    choice = Some(Some(idx));
                                }
                            }
                        })
                    });

                if !open || ctx.input(|input| input.key_released(egui::Key::Escape)) {
                    let can_cancel = resolving.can_cancel(&self.database);
                    debug!("Can cancel {:?} = {}", resolving, can_cancel);
                    if can_cancel {
                        self.to_resolve = None;
                    }
                } else if let Some(choice) = choice {
                    loop {
                        match resolving.resolve(&mut self.database, choice) {
                            ResolutionResult::Complete => {
                                let mut pending = Battlefields::check_sba(&mut self.database);
                                while pending.only_immediate_results(&self.database) {
                                    let result = pending.resolve(&mut self.database, None);
                                    if result == ResolutionResult::Complete {
                                        break;
                                    }
                                }

                                if pending.is_empty() {
                                    let entries = self.database.stack.entries_unsettled();
                                    if !self.organizing_stack && entries.len() > 1 {
                                        resolving.set_organize_stack(&self.database, entries);
                                        self.organizing_stack = true;
                                    } else {
                                        debug!("Stepping priority");
                                        self.database.turn.step_priority();
                                        assert_eq!(
                                            self.database.turn.priority_player(),
                                            self.player2
                                        );
                                        debug!("Giving ai priority",);
                                        let pending = self.ai.priority(
                                            &mut self.database,
                                            &mut PendingResults::default(),
                                        );
                                        maybe_organize_stack(
                                            &mut self.database,
                                            pending,
                                            &mut self.to_resolve,
                                            &mut self.organizing_stack,
                                        );
                                    }
                                } else {
                                    self.to_resolve = Some(pending);
                                }

                                break;
                            }
                            ResolutionResult::TryAgain => {
                                debug!("Trying again for {:#?}", resolving);
                                if !resolving.only_immediate_results(&self.database) {
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

        if let Some(inspecting) = self.inspecting_card {
            let mut open = true;
            egui::Window::new(format!(
                "{} - {}",
                inspecting.name(&self.database),
                inspecting.faceup_face(&self.database).cost.text()
            ))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.add(ui::Card {
                    db: &mut self.database,
                    card: inspecting,
                    title: None,
                });
            });

            if !open || ctx.input(|input| input.key_released(egui::Key::Escape)) {
                self.inspecting_card = None;
            }
        }

        if self.adding_card.is_some() {
            let mut open = true;

            egui::Window::new("Add card to hand")
                .open(&mut open)
                .show(ctx, |ui| {
                    let adding = self.adding_card.as_mut().unwrap();

                    let is_valid = self.cards.contains_key(adding);
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

                    let query = self.parser.parse_query_lenient(adding).0;
                    let top_docs = self
                        .searcher
                        .search(&query, &TopDocs::with_limit(10))
                        .unwrap();

                    let top = top_docs.get(0).map(|(_, addr)| {
                        self.searcher
                            .doc(*addr)
                            .unwrap()
                            .get_first(self.name)
                            .unwrap()
                            .as_text()
                            .unwrap()
                            .to_owned()
                    });

                    let mut inspecting = None;
                    let mut clicked = None;
                    for result in top_docs.into_iter().map(|(_, addr)| {
                        self.searcher
                            .doc(addr)
                            .unwrap()
                            .get_first(self.name)
                            .unwrap()
                            .as_text()
                            .unwrap()
                            .to_owned()
                    }) {
                        let label =
                            ui.add(Label::new(format!("•\t{}", result)).sense(Sense::click()));
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
                        } else if let Some(clicked) = clicked.as_ref() {
                            clicked
                        } else {
                            top.as_ref().unwrap()
                        };

                        let card =
                            CardId::upload(&mut self.database, &self.cards, self.player1, adding);
                        card.move_to_hand(&mut self.database);
                        self.adding_card = None;
                    } else if let Some(inspecting) = inspecting {
                        let card = CardId::upload(
                            &mut self.database,
                            &self.cards,
                            self.player1,
                            &inspecting,
                        );
                        self.inspecting_card = Some(card);
                    }
                    edit.request_focus();
                });

            if !open || ctx.input(|input| input.key_released(egui::Key::Escape)) {
                self.adding_card = None;
            }
        }
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
