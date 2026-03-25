use crate::config::Theme;
use crate::state::MailboxState;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;

pub struct InboxComponent;

impl InboxComponent {
    pub fn render(frame: &mut Frame, area: Rect, mailbox: &mut MailboxState, theme: &Theme) {
        let items: Vec<ListItem> = mailbox
            .emails
            .iter()
            .map(|email| {
                let indicator = if email.is_read { " " } else { "•" };
                let date = email.date.format("%b %d").to_string();
                let from = pad_or_trim(&email.from, 22);
                let subject = pad_or_trim(&email.subject, 50);
                let color = if email.is_read {
                    theme.primary_read
                } else {
                    theme.primary_unread
                };
                let line = Line::from(vec![
                    Span::styled(format!("{} ", indicator), Style::default().fg(color)),
                    Span::styled(format!("{} ", date), Style::default().fg(theme.header_fg)),
                    Span::styled(format!("{} ", from), Style::default().fg(color)),
                    Span::styled(subject, Style::default().fg(color)),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().title("Inbox").borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .bg(theme.selected_bg)
                    .fg(theme.selected_fg)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_stateful_widget(list, area, &mut mailbox.list_state);
    }
}

fn pad_or_trim(input: &str, width: usize) -> String {
    let mut out = input.to_string();
    if out.len() > width {
        out.truncate(width);
        return out;
    }
    while out.len() < width {
        out.push(' ');
    }
    out
}
