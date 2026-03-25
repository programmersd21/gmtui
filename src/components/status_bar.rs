use crate::config::{Keybindings, Theme};
use crate::state::{AppMode, AppState, SortOrder, StatusLevel};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub struct StatusBarComponent;

impl StatusBarComponent {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        state: &AppState,
        theme: &Theme,
        keys: &Keybindings,
    ) {
        let mode = match &state.mode {
            AppMode::Normal => "NORMAL",
            AppMode::Search => "SEARCH",
            AppMode::Compose => "COMPOSE",
            AppMode::EmailView => "VIEW",
            AppMode::Help => "HELP",
            AppMode::Confirm(_) => "CONFIRM",
        };

        let sort = state
            .mailbox_states
            .get(&state.active_category)
            .map(|m| m.active_sort)
            .unwrap_or(SortOrder::DateDesc);

        let sort_label = sort_label(sort);

        let mut spans = vec![
            Span::styled(
                format!(" {mode} "),
                Style::default()
                    .fg(theme.selected_fg)
                    .bg(theme.selected_bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("Sort: {sort_label}"),
                Style::default().fg(theme.header_fg),
            ),
            Span::raw("  "),
            Span::styled(
                format!("[{}] Up", keycode_label(keys.up)),
                Style::default().fg(theme.header_fg),
            ),
            Span::raw(" "),
            Span::styled(
                format!("[{}] Down", keycode_label(keys.down)),
                Style::default().fg(theme.header_fg),
            ),
            Span::raw(" "),
            Span::styled(
                format!("[{}] Open", keycode_label(keys.open)),
                Style::default().fg(theme.header_fg),
            ),
            Span::raw(" "),
            Span::styled(
                format!("[{}] Compose", keycode_label(keys.compose)),
                Style::default().fg(theme.header_fg),
            ),
            Span::raw(" "),
            Span::styled(
                format!("[{}] Reply", keycode_label(keys.reply)),
                Style::default().fg(theme.header_fg),
            ),
            Span::raw(" "),
            Span::styled(
                format!("[{}] Delete", keycode_label(keys.delete)),
                Style::default().fg(theme.header_fg),
            ),
            Span::raw(" "),
            Span::styled(
                format!("[{}] Search", keycode_label(keys.search)),
                Style::default().fg(theme.header_fg),
            ),
            Span::raw(" "),
            Span::styled(
                format!("[{}] Refresh", keycode_label(keys.refresh)),
                Style::default().fg(theme.header_fg),
            ),
            Span::raw(" "),
            Span::styled(
                format!("[{}] Quit", keycode_label(keys.quit)),
                Style::default().fg(theme.header_fg),
            ),
            Span::raw(" "),
            Span::styled(
                format!("[{}] Next Tab", keycode_label(keys.next_tab)),
                Style::default().fg(theme.header_fg),
            ),
            Span::raw(" "),
            Span::styled(
                format!("[{}] Prev Tab", keycode_label(keys.prev_tab)),
                Style::default().fg(theme.header_fg),
            ),
            Span::raw(" "),
            Span::styled(
                format!("[{}] Load More", keycode_label(keys.load_more)),
                Style::default().fg(theme.header_fg),
            ),
            Span::raw(" "),
            Span::styled(
                format!("[{}] Help", keycode_label(keys.help)),
                Style::default().fg(theme.header_fg),
            ),
        ];

        if let AppMode::Compose = state.mode {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                "[Ctrl+S] Send",
                Style::default().fg(theme.header_fg),
            ));
        }

        if state.is_loading {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                "Loading...",
                Style::default()
                    .fg(theme.updates)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        if let Some((msg, level)) = &state.status_message {
            let color = match level {
                StatusLevel::Info => theme.social,
                StatusLevel::Warning => theme.promotions,
                StatusLevel::Error => theme.updates,
            };
            spans.push(Span::raw("  "));
            spans.push(Span::styled(msg.clone(), Style::default().fg(color)));
        }

        let bar = Paragraph::new(Line::from(spans))
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(theme.header_fg))
                    .style(Style::default().bg(theme.status_bar_bg)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(bar, area);
    }
}

fn sort_label(sort: SortOrder) -> &'static str {
    match sort {
        SortOrder::DateDesc => "Date ↓",
        SortOrder::DateAsc => "Date ↑",
        SortOrder::SenderAsc => "Sender A→Z",
        SortOrder::SubjectAsc => "Subject A→Z",
    }
}

fn keycode_label(code: crossterm::event::KeyCode) -> String {
    use crossterm::event::KeyCode;
    match code {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => "Shift+Tab".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::PageUp => "PgUp".to_string(),
        KeyCode::PageDown => "PgDn".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        _ => "Key".to_string(),
    }
}
