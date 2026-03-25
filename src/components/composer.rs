use crate::config::Theme;
use crate::state::{ComposerField, ComposerState};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub struct ComposerComponent;

impl ComposerComponent {
    pub fn render(frame: &mut Frame, area: Rect, composer: &ComposerState, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(1),
            ])
            .split(area);

        let to_active = composer.active_field == ComposerField::To;
        let subject_active = composer.active_field == ComposerField::Subject;
        let body_active = composer.active_field == ComposerField::Body;

        let to_style = field_style(to_active, theme);
        let subject_style = field_style(subject_active, theme);
        let body_style = field_style(body_active, theme);

        let to = Paragraph::new(composer.to.as_str()).style(to_style).block(
            Block::default()
                .title(if to_active { "> To" } else { "To" })
                .borders(Borders::ALL)
                .border_style(to_style),
        );
        frame.render_widget(to, chunks[0]);

        let subject = Paragraph::new(composer.subject.as_str())
            .style(subject_style)
            .block(
                Block::default()
                    .title(if subject_active {
                        "> Subject"
                    } else {
                        "Subject"
                    })
                    .borders(Borders::ALL)
                    .border_style(subject_style),
            );
        frame.render_widget(subject, chunks[1]);

        let body = Paragraph::new(composer.body.as_str())
            .style(body_style)
            .block(
                Block::default()
                    .title(if body_active { "> Body" } else { "Body" })
                    .borders(Borders::ALL)
                    .border_style(body_style),
            );
        frame.render_widget(body, chunks[2]);

        if to_active {
            frame.set_cursor_position((
                chunks[0].x + composer.cursor_to as u16 + 1,
                chunks[0].y + 1,
            ));
        } else if subject_active {
            frame.set_cursor_position((
                chunks[1].x + composer.cursor_subject as u16 + 1,
                chunks[1].y + 1,
            ));
        } else if body_active {
            let lines = composer.body.split('\n').count().max(1);
            let cursor_line = composer.body[..composer.cursor_body.min(composer.body.len())]
                .split('\n')
                .count();
            let line_offset = cursor_line.saturating_sub(1);
            let col = composer.body[..composer.cursor_body.min(composer.body.len())]
                .split('\n')
                .last()
                .map(|s| s.len())
                .unwrap_or(0);
            let y = chunks[2].y + 1 + line_offset as u16;
            let x = chunks[2].x + 1 + col as u16;
            if line_offset < lines {
                frame.set_cursor_position((x, y));
            }
        }
    }
}

fn field_style(active: bool, theme: &Theme) -> Style {
    if active {
        Style::default()
            .fg(theme.primary_unread)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.primary_read)
    }
}
