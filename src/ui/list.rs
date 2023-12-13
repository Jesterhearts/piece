use ratatui::{
    layout::Alignment,
    style::{Style, Stylize},
    text::Span,
    widgets::{
        block::{Position, Title},
        Block, Borders, StatefulWidget, Widget,
    },
};

use crate::ui::linewrap::{LineComposer, WordWrapper};

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]

pub struct ListState {
    pub selected: Option<usize>,
    pub hovered: Option<usize>,
    pub selected_up: bool,
    pub selected_down: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct List<'a> {
    pub title: String,
    pub items: Vec<Span<'a>>,
    pub last_hover: Option<(u16, u16)>,
    pub last_click: Option<(u16, u16)>,
    pub offset: usize,
}

impl<'a> StatefulWidget for List<'a> {
    type State = ListState;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        state.hovered = None;
        state.selected_down = false;
        state.selected_up = false;

        let block = Block::default().title(self.title).borders(Borders::ALL);
        let inner_area = block.inner(area);
        block.render(area, buf);
        let area = inner_area;

        if self.items.is_empty() {
            return;
        }

        if area.is_empty() {
            return;
        }

        if let Some(clicked) = self.last_click {
            if clicked.0 >= area.top()
                && clicked.0 < area.top() + 1
                && clicked.1 >= area.left()
                && clicked.1 < area.right()
            {
                state.selected_up = true;
            } else if clicked.0 >= area.bottom() - 1
                && clicked.0 < area.bottom()
                && clicked.1 >= area.left()
                && clicked.1 < area.right()
            {
                state.selected_down = true;
            }
        }

        let block = Block::default()
            .title(
                Title::from(" 🡅 ")
                    .alignment(Alignment::Center)
                    .position(Position::Top),
            )
            .title(
                Title::from(" 🡇 ")
                    .alignment(Alignment::Center)
                    .position(Position::Bottom),
            )
            .borders(Borders::TOP | Borders::BOTTOM);

        let inner_area = block.inner(area);
        block.render(area, buf);
        let area = inner_area;

        let mut current_height = 0;
        for (i, item) in self.items.into_iter().enumerate().skip(self.offset) {
            if current_height >= area.height {
                return;
            }

            let mut graphemes = item.styled_graphemes(Style::default());
            let mut lines = WordWrapper::new(&mut graphemes, area.width - 1, false);
            let mut all_lines = vec![];
            while let Some(line) = lines.next_line() {
                all_lines.push(line.0.to_owned());
            }

            if let Some(hover) = self.last_hover {
                if hover.0 >= area.top() + current_height
                    && hover.0 < area.top() + current_height + all_lines.len() as u16
                    && hover.1 >= area.left()
                    && hover.1 < area.right()
                {
                    state.hovered = Some(i);
                }
            }

            if let Some(click) = self.last_click {
                if click.0 >= area.top() + current_height
                    && click.0 < area.top() + current_height + all_lines.len() as u16
                    && click.1 >= area.left()
                    && click.1 < area.right()
                {
                    state.selected = Some(i);
                }
            }

            let selected = state.selected.map(|s| s == i).unwrap_or_default();
            let hovered = state.hovered.map(|s| s == i).unwrap_or_default();

            for (line_num, line) in all_lines.into_iter().enumerate() {
                if current_height >= area.height {
                    return;
                }

                if line_num == 0 && selected {
                    buf.set_string(
                        area.left(),
                        area.top() + current_height,
                        ">",
                        Style::default(),
                    )
                } else {
                    buf.set_string(
                        area.left(),
                        area.top() + current_height,
                        " ",
                        Style::default(),
                    )
                }

                let mut x = area.left() + 1;
                for chunk in line {
                    let mut span = Span::from(chunk.symbol);
                    if hovered {
                        span.patch_style(Style::default().on_dark_gray());
                    }

                    let pos = buf.set_span(x, area.top() + current_height, &span, area.width - 1);
                    x = pos.0;
                }

                current_height += 1;
            }
        }
    }
}
