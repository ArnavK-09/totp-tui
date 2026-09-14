use ratatui::layout::{Constraint, Layout, Rect};

pub struct LayoutManager;

impl LayoutManager {
    pub fn get_base_layout(&self, area: Rect) -> (Rect, Rect) {
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(20), Constraint::Fill(1)]).areas(area);

        (left, right)
    }
}
