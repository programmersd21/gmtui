use crate::config::Theme;
use crate::state::{all_categories, AppState};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

pub struct SidebarComponent;

impl SidebarComponent {
    pub fn render(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
        let mut items = Vec::new();
        let categories = all_categories();
        for category in &categories {
            let unread = state
                .mailbox_states
                .get(category)
                .map(|mb| mb.emails.iter().filter(|e| !e.is_read).count())
                .unwrap_or(0);
            let label = format!("{} ({})", category, unread);
            let line = Line::from(Span::styled(label, Style::default().fg(theme.header_fg)));
            items.push(ListItem::new(line));
        }

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Mailboxes")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.header_fg)),
            )
            .highlight_style(
                Style::default()
                    .bg(theme.selected_bg)
                    .fg(theme.selected_fg)
                    .add_modifier(Modifier::BOLD),
            );

        let active_index = categories
            .iter()
            .position(|c| *c == state.active_category)
            .unwrap_or(0);
        let mut list_state = ListState::default();
        list_state.select(Some(active_index));

        frame.render_stateful_widget(list, area, &mut list_state);
    }
}
