use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style, Stylize},
    text::Span,
    widgets::{Block, StatefulWidget, Widget},
};

use crate::ui::linewrap::{LineComposer, WordWrapper};

const ITEMS_PER_PAGE: usize = 9;

#[derive(Debug, Default)]
pub struct HorizontalListState {
    pub hovered: Option<usize>,
    pub start_index: usize,
    pub count: Option<usize>,
    pub has_overflow: bool,
    pub left_clicked: bool,
    pub right_clicked: bool,
}

/// A horizontal list, numbers the items based on their displayed position.
#[derive(Debug)]
pub struct HorizontalList<'a> {
    block: Option<Block<'a>>,
    items: Vec<(usize, Span<'a>)>,
    style: Style,
    page: u16,
    last_hover: Option<(u16, u16)>,
    last_click: Option<(u16, u16)>,
}

impl<'a> HorizontalList<'a> {
    pub fn new(
        items: Vec<(usize, Span<'a>)>,
        last_hover: Option<(u16, u16)>,
        last_click: Option<(u16, u16)>,
    ) -> Self {
        Self {
            block: None,
            items,
            style: Default::default(),
            page: 0,
            last_hover,
            last_click,
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
        let mut remaining_width = list_area.width - 5 /* 3 for ... 2 for < and > */;
        let mut item_index = 1;

        buf.set_string(x, y, "🡄", Style::default());
        buf.set_string(list_area.right() - 1, y, "🡆", Style::default());
        x += 1;

        if let Some(click) = self.last_click {
            if click.0 >= list_area.top()
                && click.0 < list_area.bottom()
                && click.1 >= list_area.left()
                && click.1 < list_area.left() + 1
            {
                state.left_clicked = true;
            }
            if click.0 >= list_area.top()
                && click.0 < list_area.bottom()
                && click.1 >= list_area.right() - 1
                && click.1 < list_area.right()
            {
                state.right_clicked = true;
            }
        }

        let mut current_page = 0;

        for (index, (outer_index, item)) in self.items.iter().enumerate() {
            // We always do "(#) " for 1-9
            const NUMBER_WIDTH: u16 = 4;
            // We separate with " "
            const ITEM_SPACING: u16 = 1;
            let mut item_width = (item.width() as u16)
                .saturating_add(NUMBER_WIDTH)
                .saturating_add(ITEM_SPACING)
                .min(list_area.width - 3);

            if item_index > ITEMS_PER_PAGE {
                break;
            }

            if remaining_width == list_area.width - 5 && item_width > remaining_width {
                let mut graphemes = item.styled_graphemes(Style::default());
                let mut lines = WordWrapper::new(&mut graphemes, list_area.right() - 1 - x, true);
                let mut max = 0;
                while let Some(line) = lines.next_line() {
                    max = line.1.max(max);
                }
                item_width = max;
            }

            if item_width > remaining_width {
                current_page += 1;
                remaining_width = list_area.width - 5;
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
                let mut lines = WordWrapper::new(&mut graphemes, list_area.right() - 1 - x, true);

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

                if let Some(hover) = self.last_hover {
                    if hover.0 >= y
                        && hover.0 < list_area.bottom()
                        && hover.1 >= initial_x
                        && hover.1 < x
                    {
                        state.hovered = Some(*outer_index);
                        buf.set_style(
                            Rect {
                                x: initial_x,
                                y,
                                width: max_width,
                                height: y_offset,
                            },
                            Style::default().on_dark_gray(),
                        );
                    }
                }

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
