use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

pub struct AccountListItem {
    pub issuer: String,
    pub account_name: String,
    pub selected: bool,
}

impl Widget for AccountListItem {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let selected_style = Style::default().fg(Color::Black).bg(Color::Cyan);

        let issuer_style = if self.selected {
            selected_style
        } else {
            Style::default().fg(Color::White)
        };

        let account_style = if self.selected {
            selected_style
        } else {
            Style::default().fg(Color::DarkGray)
        };

        if self.selected {
            buf.set_style(area, selected_style);
        }

        let marker = if self.selected { "›" } else { " " };

        buf.set_string(area.left(), area.top(), marker, issuer_style);

        buf.set_string(area.left() + 2, area.top(), &self.issuer, issuer_style);

        buf.set_string(
            area.left() + 2,
            area.top() + 1,
            &self.account_name,
            account_style,
        );
    }
}
