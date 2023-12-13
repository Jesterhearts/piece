use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, StatefulWidget, Widget},
};

use crate::ui::linewrap::{LineComposer, WordWrapper};

const ITEMS_PER_PAGE: usize = 9;

#[derive(Debug, Default)]
pub struct HorizontalListState {
    pub start_index: usize,
    pub count: Option<usize>,
    pub has_overflow: bool,
}

/// A horizontal list, numbers the items based on their displayed position.
#[derive(Debug)]
pub struct HorizontalList<'a> {
    block: Option<Block<'a>>,
    items: Vec<Span<'a>>,
    style: Style,
    page: u16,
}

impl<'a> HorizontalList<'a> {
    pub fn new(items: Vec<Span<'a>>) -> Self {
        Self {
            block: None,
            items,
            style: Default::default(),
            page: 0,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn page(mut self, page: u16) -> Self {
        self.page = page;
        self
    }
}

impl StatefulWidget for HorizontalList<'_> {
    type State = HorizontalListState;

    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        *state = HorizontalListState::default();
        buf.set_style(area, self.style);
        let list_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };

        if list_area.width == 0 || list_area.height == 0 {
            return;
        }

        if self.items.is_empty() {
            return;
        }

        let y = list_area.top();
        let mut x = list_area.left();
        let mut remaining_width = list_area.width - 3;
        let mut item_index = 1;

        let mut current_page = 0;

        for (index, item) in self.items.iter().enumerate() {
            // We always do "(#) " for 1-9
            const NUMBER_WIDTH: u16 = 4;
            // We separate with " "
            const ITEM_SPACING: u16 = 1;
            let item_width = (item.width() as u16)
                .saturating_add(NUMBER_WIDTH)
                .saturating_add(ITEM_SPACING)
                .min(list_area.width - 3);

            if item_index > ITEMS_PER_PAGE {
                break;
            }

            if item_width > remaining_width {
                current_page += 1;
                remaining_width = list_area.width - 3;
                if current_page == self.page {
                    state.start_index = index;
                }
            }

            remaining_width = remaining_width.saturating_sub(item_width);
            if current_page > self.page {
                break;
            }

            if current_page == self.page {
                state.count = Some(index);
                let list_number = Span::styled(
                    format!("({}) ", item_index),
                    Style::default().add_modifier(Modifier::BOLD),
                );
                item_index += 1;

                let pos = buf.set_span(x, y, &list_number, list_area.right() - x);
                x = pos.0;

                let mut graphemes = item.styled_graphemes(Style::default());
                let mut lines = WordWrapper::new(&mut graphemes, list_area.right() - x, true);

                let initial_x = x;
                let mut max_width = 0;
                let mut y_offset = 0;
                while let Some(line) = lines.next_line() {
                    if y + y_offset < list_area.bottom() {
                        for chunk in line.0 {
                            let pos = buf.set_span(
                                x,
                                y + y_offset,
                                &Span::from(chunk.symbol),
                                list_area.right() - x,
                            );
                            x = pos.0;
                        }
                        x = initial_x;
                        max_width = max_width.max(line.1);
                        y_offset += 1;
                    }
                }

                x = initial_x + max_width;

                let pos = buf.set_span(x, y, &Span::from(" "), list_area.right() - x);
                x = pos.0;
            }
        }
        if state.count.is_some() && state.count.unwrap() < self.items.len() - 1 {
            state.has_overflow = true;
            buf.set_span(x, y, &Span::from("..."), list_area.right() - x);
        } else {
            state.has_overflow = false;
        }
    }
}
