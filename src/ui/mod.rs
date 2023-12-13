pub mod horizontal_list;
pub mod linewrap;

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
    player::Owner,
    ui::horizontal_list::{HorizontalList, HorizontalListState},
};

#[derive(Debug, Default)]
pub struct CardSelectionState {
    pub selected: Option<CardId>,
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

        if let Some(pt) = self.pt {
            block = block.title(
                Title::from(pt)
                    .position(Position::Bottom)
                    .alignment(Alignment::Right),
            )
        }

        if hovered {
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

        let mana_abilities = self.card.abilities_text(self.db);
        let modified_by = self.card.modified_by(self.db);
        let is_modified = !modified_by.is_empty();
        let counters = CounterId::counter_text_on(self.db, self.card);
        let has_counters = !counters.is_empty();

        let paragraph = std::iter::once(self.card.oracle_text(self.db))
            .chain(std::iter::once(mana_abilities))
            .chain(std::iter::once("Modified by:".to_string()).filter(|_| is_modified))
            .chain(modified_by)
            .chain(std::iter::once("Counters:".to_string()).filter(|_| has_counters))
            .chain(counters)
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
                let index = card.id(self.db);
                let name = card.name(self.db);
                let cost = card.cost(self.db);
                if cost.mana_cost.is_empty() {
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

pub struct SelectedAbilities<'db> {
    pub db: &'db mut Database,
    pub card: Option<CardId>,
}

impl<'db> StatefulWidget for SelectedAbilities<'db> {
    type State = HorizontalListState;
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &mut Self::State) {
        let block = Block::default().borders(Borders::ALL);
        let inner_area = block.inner(area);
        block.render(area, buf);
        let area = inner_area;

        if let Some(card) = self.card {
            let abilites = card.activated_abilities(self.db);

            HorizontalList::new(
                abilites
                    .iter()
                    .map(|ability| ability.text(self.db))
                    .map(Span::from)
                    .collect_vec(),
            )
            .render(area, buf, state);
        }
    }
}
