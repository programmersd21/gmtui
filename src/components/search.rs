use crate::config::Theme;
use crate::state::SearchState;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

pub struct SearchComponent;

impl SearchComponent {
    pub fn render(frame: &mut Frame, area: Rect, search: &mut SearchState, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(area);

        let input = format!("/{}", search.query);
        let input_widget = Paragraph::new(input)
            .style(Style::default().fg(theme.primary_unread))
            .block(
                Block::default()
                    .title("Search")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.header_fg)),
            );
        frame.render_widget(input_widget, chunks[0]);
        let cursor_x = chunks[0].x + 2 + search.cursor_pos as u16;
        let cursor_y = chunks[0].y + 1;
        frame.set_cursor_position((cursor_x, cursor_y));

        let items: Vec<ListItem> = search
            .results
            .iter()
            .map(|email| {
                let line = Line::from(vec![
                    Span::styled(email.from.clone(), Style::default().fg(theme.primary_read)),
                    Span::raw("  "),
                    Span::styled(
                        email.subject.clone(),
                        Style::default()
                            .fg(theme.primary_unread)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Results")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.header_fg)),
            )
            .highlight_style(
                Style::default()
                    .bg(theme.selected_bg)
                    .fg(theme.selected_fg)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_stateful_widget(list, chunks[1], &mut search.list_state);
    }
}
