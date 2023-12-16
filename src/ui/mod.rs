pub mod horizontal_list;
pub mod linewrap;
pub mod list;

use itertools::Itertools;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Stylize,
    text::Span,
    widgets::{
        block::{Position, Title},
        Block, BorderType, Borders, Paragraph, StatefulWidget, Widget, Wrap,
    },
};

use crate::{
    in_play::{CardId, CounterId, Database, OnBattlefield},
    player::{AllPlayers, Owner},
    turns::Turn,
    ui::horizontal_list::{HorizontalList, HorizontalListState},
};

#[derive(Debug, Default)]
pub struct CardSelectionState {
    pub selected: Option<CardId>,
    pub hovered: Option<CardId>,
}

pub struct Card<'db> {
    pub db: &'db mut Database,
    pub card: CardId,
    pub title: String,
    pub pt: Option<String>,
    pub last_hover: Option<(u16, u16)>,
    pub last_click: Option<(u16, u16)>,
}

impl<'db> StatefulWidget for Card<'db> {
    type State = CardSelectionState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let hovered = if let Some(last_hover) = self.last_hover {
            area.intersects(Rect {
                x: last_hover.1,
                y: last_hover.0,
                width: 1,
                height: 1,
            })
        } else {
            false
        };

        let clicked = if let Some(last_click) = self.last_click {
            area.intersects(Rect {
                x: last_click.1,
                y: last_click.0,
                width: 1,
                height: 1,
            })
        } else {
            false
        };

        let mut block = Block::default()
            .title(Title::from(self.title).position(Position::Top))
            .borders(Borders::ALL);

        let types = self.card.types(self.db);
        let subtypes = self.card.subtypes(self.db);

        let typeline = std::iter::once(types.iter().map(|ty| ty.as_ref()).join(" "))
            .chain(
                std::iter::once(subtypes.iter().map(|ty| ty.as_ref()).join(" "))
                    .filter(|s| !s.is_empty()),
            )
            .join(" - ");

        block = block.title(
            Title::from(format!(" {} ", typeline))
                .position(Position::Bottom)
                .alignment(Alignment::Left),
        );

        if let Some(pt) = self.pt {
            block = block.title(
                Title::from(format!(" {} ", pt))
                    .position(Position::Bottom)
                    .alignment(Alignment::Right),
            )
        }

        if hovered {
            state.hovered = Some(self.card);
            block = block.on_dark_gray();
        }

        if clicked {
            state.selected = Some(self.card);
            block = block.white().bold();
        }

        if self.card.tapped(self.db) {
            block = block.italic();
        }

        let inner_area = block.inner(area);
        block.render(area, buf);
        let area = inner_area;

        if self.card.manifested(self.db) {
            return;
        }

        let source = self
            .card
            .cloning(self.db)
            .map(CardId::from)
            .unwrap_or(self.card);

        let oracle_text = source.oracle_text(self.db);
        let has_oracle_text = !oracle_text.is_empty();
        let triggers = source.triggers_text(self.db);
        let has_triggers = !triggers.is_empty();
        let abilities = source.abilities_text(self.db);
        let has_abilities = !abilities.is_empty();
        let modified_by = source.modified_by(self.db);
        let is_modified = !modified_by.is_empty();
        let counters = CounterId::counter_text_on(self.db, source);
        let has_counters = !counters.is_empty();

        let paragraph = std::iter::once(oracle_text)
            .chain(std::iter::once(String::default()).filter(|_| has_oracle_text))
            .chain(triggers)
            .chain(std::iter::once(String::default()).filter(|_| has_triggers))
            .chain(std::iter::once(abilities))
            .chain(std::iter::once(String::default()).filter(|_| has_abilities))
            .chain(std::iter::once("Modified by:".to_string()).filter(|_| is_modified))
            .chain(modified_by)
            .chain(std::iter::once(String::default()).filter(|_| is_modified))
            .chain(std::iter::once("Counters:".to_string()).filter(|_| has_counters))
            .chain(counters.into_iter().map(|counter| format!("  {}", counter)))
            .join("\n");

        let mut paragraph = Paragraph::new(paragraph).wrap(Wrap { trim: false });

        if self.card.tapped(self.db) {
            paragraph = paragraph.italic();
        }

        paragraph.render(area, buf)
    }
}

pub struct Battlefield<'db> {
    pub db: &'db mut Database,
    pub owner: Owner,
    pub player_name: String,
    pub last_hover: Option<(u16, u16)>,
    pub last_click: Option<(u16, u16)>,
}

impl<'db> StatefulWidget for Battlefield<'db> {
    type State = CardSelectionState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        state.hovered = None;

        let block = Block::default()
            .title(self.player_name)
            .border_type(BorderType::Double)
            .borders(Borders::ALL);

        let inner_area = block.inner(area);
        block.render(area, buf);
        let area = inner_area;

        let cards = self.owner.get_cards::<OnBattlefield>(self.db);
        if cards.is_empty() {
            return;
        }

        let card_titles = cards
            .iter()
            .map(|card| {
                let manifested = card.manifested(self.db);

                let index = card.id(self.db);
                let name = if manifested {
                    "Manifested".to_string()
                } else {
                    card.name(self.db)
                };

                let cost = card.cost(self.db);
                if cost.mana_cost.is_empty() || manifested {
                    format!("({}) {}", index, name)
                } else {
                    format!("({}) {} - {}", index, name, cost.text())
                }
            })
            .collect_vec();

        let max_len = card_titles.iter().map(|t| t.len()).max().unwrap();
        let cards_wide = area.width as usize / max_len;
        let wide_percentage = (1.0 / cards_wide as f32 * 100.0).floor() as u16;
        let cards_tall = (cards.len() as f32 / cards_wide as f32).ceil();
        let tall_percentage = (1.0 / cards_tall * 100.0).floor() as u16;

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                (0..cards_tall as usize)
                    .map(|_| Constraint::Percentage(tall_percentage))
                    .collect_vec(),
            )
            .split(area);

        let mut card_and_title = cards.into_iter().zip(card_titles);

        for cell in layout.iter() {
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    (0..cards_wide)
                        .map(|_| Constraint::Percentage(wide_percentage))
                        .collect_vec(),
                )
                .split(*cell);

            for cell in layout.iter() {
                if let Some((card, title)) = card_and_title.next() {
                    let pt = card.pt_text(self.db);
                    Card {
                        db: self.db,
                        card,
                        title,
                        pt,
                        last_hover: self.last_hover,
                        last_click: self.last_click,
                    }
                    .render(*cell, buf, state);
                } else {
                    return;
                }
            }
        }
    }
}

pub struct SelectedAbilities<'db, 'ap, 't> {
    pub db: &'db mut Database,
    pub all_players: &'ap AllPlayers,
    pub turn: &'t Turn,
    pub card: Option<CardId>,
    pub page: u16,
    pub last_hover: Option<(u16, u16)>,
    pub last_click: Option<(u16, u16)>,
}

impl<'db, 'ap, 't> StatefulWidget for SelectedAbilities<'db, 'ap, 't> {
    type State = HorizontalListState;
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &mut Self::State) {
        let block = Block::default()
            .title(" Select an option ")
            .borders(Borders::ALL);
        let inner_area = block.inner(area);
        block.render(area, buf);
        let area = inner_area;

        if let Some(card) = self.card {
            let abilites = card
                .activated_abilities(self.db)
                .into_iter()
                .filter(|ability| ability.can_be_activated(self.db, self.all_players, self.turn))
                .collect_vec();

            HorizontalList::new(
                abilites
                    .iter()
                    .map(|ability| ability.text(self.db))
                    .map(Span::from)
                    .collect_vec(),
                self.last_hover,
                self.last_click,
            )
            .page(self.page)
            .render(area, buf, state);
        }
    }
}
