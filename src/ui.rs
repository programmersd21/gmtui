use crate::components::{
    composer::ComposerComponent, email_view::EmailViewComponent, inbox::InboxComponent,
    search::SearchComponent, sidebar::SidebarComponent, status_bar::StatusBarComponent,
};
use crate::config::Config;
use crate::state::{all_categories, AppMode, AppState};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn render(frame: &mut Frame, state: &mut AppState, config: &Config) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(frame.area());

    render_header(frame, layout[0], state, config);
    render_body(frame, layout[1], state, config);
    StatusBarComponent::render(frame, layout[2], state, &config.theme, &config.keybindings);

    if let AppMode::Search = state.mode {
        render_search_overlay(frame, frame.area(), state, config);
    }

    if let AppMode::Confirm(_) = state.mode {
        render_confirm_overlay(frame, frame.area(), state, config);
    }

    if let AppMode::Help = state.mode {
        render_help_overlay(frame, frame.area(), config);
    }
}

fn render_header(frame: &mut Frame, area: Rect, state: &AppState, config: &Config) {
    let theme = &config.theme;
    let mut spans = vec![Span::styled(
        "gmtui",
        Style::default()
            .fg(theme.primary_unread)
            .add_modifier(Modifier::BOLD),
    )];
    spans.push(Span::raw("  "));

    for category in all_categories() {
        let active = category == state.active_category;
        let style = if active {
            Style::default()
                .fg(theme.selected_fg)
                .bg(theme.selected_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.header_fg)
        };
        spans.push(Span::styled(format!(" {} ", category), style));
        spans.push(Span::raw(" "));
    }

    let header = Paragraph::new(Line::from(spans)).style(Style::default().fg(theme.header_fg));
    frame.render_widget(header, area);
}

fn render_body(frame: &mut Frame, area: Rect, state: &mut AppState, config: &Config) {
    let theme = &config.theme;
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(22), Constraint::Percentage(78)])
        .split(area);

    SidebarComponent::render(frame, columns[0], state, theme);

    match state.mode {
        AppMode::Normal | AppMode::Search | AppMode::Help => {
            if let Some(mailbox) = state.mailbox_states.get_mut(&state.active_category) {
                InboxComponent::render(frame, columns[1], mailbox, theme);
            }
        }
        AppMode::EmailView => {
            EmailViewComponent::render(
                frame,
                columns[1],
                state.current_email.as_ref(),
                state.email_view.scroll,
                theme,
            );
        }
        AppMode::Compose => {
            ComposerComponent::render(frame, columns[1], &state.composer, theme);
        }
        AppMode::Confirm(_) => {
            if let Some(mailbox) = state.mailbox_states.get_mut(&state.active_category) {
                InboxComponent::render(frame, columns[1], mailbox, theme);
            }
        }
    }
}

fn render_search_overlay(frame: &mut Frame, area: Rect, state: &mut AppState, config: &Config) {
    let overlay_height = (area.height as f32 * 0.3).ceil() as u16;
    let overlay_area = Rect {
        x: area.x + 2,
        y: area.y + area.height.saturating_sub(overlay_height + 1),
        width: area.width.saturating_sub(4),
        height: overlay_height,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(config.theme.header_fg))
        .style(Style::default().bg(config.theme.status_bar_bg));
    frame.render_widget(block, overlay_area);

    let inner = Rect {
        x: overlay_area.x + 1,
        y: overlay_area.y + 1,
        width: overlay_area.width.saturating_sub(2),
        height: overlay_area.height.saturating_sub(2),
    };

    SearchComponent::render(frame, inner, &mut state.search, &config.theme);
}

fn render_confirm_overlay(frame: &mut Frame, area: Rect, _state: &AppState, config: &Config) {
    let width = 50.min(area.width.saturating_sub(4));
    let height = 5.min(area.height.saturating_sub(2));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let rect = Rect {
        x,
        y,
        width,
        height,
    };

    let text = "Delete this email? (y/n)";
    let block = Paragraph::new(text)
        .style(Style::default().fg(config.theme.primary_unread))
        .block(
            Block::default()
                .title("Confirm")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(config.theme.header_fg)),
        );
    frame.render_widget(block, rect);
}

fn render_help_overlay(frame: &mut Frame, area: Rect, config: &Config) {
    let width = 60.min(area.width.saturating_sub(4));
    let height = 12.min(area.height.saturating_sub(2));
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let rect = Rect {
        x,
        y,
        width,
        height,
    };

    let keys = &config.keybindings;
    let lines = vec![
        format!("Up: {}", keycode_label(keys.up)),
        format!("Down: {}", keycode_label(keys.down)),
        format!("Open: {}", keycode_label(keys.open)),
        format!("Compose: {}", keycode_label(keys.compose)),
        "Send: Ctrl+S".to_string(),
        format!("Reply: {}", keycode_label(keys.reply)),
        format!("Delete: {}", keycode_label(keys.delete)),
        format!("Search: {}", keycode_label(keys.search)),
        format!("Refresh: {}", keycode_label(keys.refresh)),
        format!("Quit: {}", keycode_label(keys.quit)),
        format!("Next Tab: {}", keycode_label(keys.next_tab)),
        format!("Prev Tab: {}", keycode_label(keys.prev_tab)),
        format!("Load More: {}", keycode_label(keys.load_more)),
        format!("Help: {}", keycode_label(keys.help)),
        "Esc: close".to_string(),
    ];

    let text = lines.join("\n");
    let block = Paragraph::new(text)
        .style(Style::default().fg(config.theme.primary_unread))
        .block(
            Block::default()
                .title("Help")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(config.theme.header_fg)),
        );
    frame.render_widget(block, rect);
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
