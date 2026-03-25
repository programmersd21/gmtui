use crate::config::Theme;
use crate::gmail::models::GmailMessage;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub struct EmailViewComponent;

impl EmailViewComponent {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        message: Option<&GmailMessage>,
        scroll: u16,
        theme: &Theme,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(6), Constraint::Min(1)])
            .split(area);

        let header_lines = if let Some(email) = message {
            vec![
                Line::from(Span::styled(
                    email.subject.clone(),
                    Style::default()
                        .fg(theme.primary_unread)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(format!("From: {}", email.from)),
                Line::from(format!("To: {}", email.to)),
                Line::from(format!("Date: {}", email.date.to_rfc2822())),
            ]
        } else {
            vec![Line::from("No email selected")]
        };

        let header = Paragraph::new(Text::from(header_lines))
            .style(Style::default().fg(theme.primary_read))
            .block(
                Block::default()
                    .title("Message")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.header_fg)),
            )
            .wrap(Wrap { trim: true });

        let body_text = message
            .map(|m| m.body.clone())
            .unwrap_or_else(|| "(no body loaded)".to_string());

        let body = Paragraph::new(body_text)
            .style(Style::default().fg(theme.primary_read))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.header_fg)),
            )
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));

        frame.render_widget(header, chunks[0]);
        frame.render_widget(body, chunks[1]);
    }
}
