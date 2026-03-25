use crate::config::{Action, Config};
use crate::gmail::models::GmailMessage;
use crate::gmail::GmailClient;
use crate::state::{
    all_categories, AppMode, AppState, ComposerField, ConfirmAction, EmailSummary, MailboxCategory,
    SortOrder, StatusLevel,
};
use crate::ui;
use anyhow::Result;
use base64::Engine;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::Stdout;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

pub enum InputEvent {
    Input(Event),
    App(AppEvent),
}

pub enum AppEvent {
    EmailsLoaded {
        category: MailboxCategory,
        emails: Vec<EmailSummary>,
        next_page_token: Option<String>,
        append: bool,
    },
    EmailLoaded {
        message: GmailMessage,
    },
    MessageSent(Result<()>),
    MessageDeleted {
        id: String,
        result: Result<()>,
    },
    Status {
        message: String,
        level: StatusLevel,
    },
    EmailSummaryUpdated {
        category: MailboxCategory,
        summary: EmailSummary,
    },
}

enum ScrollAction {
    Inbox(i32),
    Search(i32),
    Email(i32),
}

pub struct App {
    pub state: AppState,
    pub client: Arc<GmailClient>,
    pub config: Config,
    pub last_action_at: Instant,
    tx: Option<UnboundedSender<InputEvent>>,
}

impl App {
    pub fn new(config: Config, client: GmailClient) -> Self {
        Self {
            state: AppState::default(),
            client: Arc::new(client),
            config,
            last_action_at: Instant::now() - Duration::from_millis(20),
            tx: None,
        }
    }

    pub async fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        let (tx, mut rx) = unbounded_channel();
        self.tx = Some(tx.clone());
        spawn_input_thread(tx.clone());

        self.refresh_category(false).await;

        terminal.draw(|frame| ui::render(frame, &mut self.state, &self.config))?;

        while let Some(event) = rx.recv().await {
            let continue_running = self.handle_event(event).await?;
            terminal.draw(|frame| ui::render(frame, &mut self.state, &self.config))?;
            if !continue_running {
                break;
            }
        }

        Ok(())
    }

    pub async fn handle_event(&mut self, event: InputEvent) -> Result<bool> {
        match event {
            InputEvent::Input(Event::Key(key)) => {
                if self.should_throttle_key(key) {
                    return Ok(true);
                }
                if self.config.keybindings.matches(Action::Quit, key) {
                    return Ok(false);
                }
                self.handle_key(key).await?;
            }
            InputEvent::Input(_) => {}
            InputEvent::App(app_event) => {
                self.handle_app_event(app_event)?;
            }
        }
        Ok(true)
    }

    async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.state.mode {
            AppMode::Normal => self.handle_normal_key(key).await?,
            AppMode::Search => self.handle_search_key(key).await?,
            AppMode::Compose => self.handle_compose_key(key).await?,
            AppMode::EmailView => self.handle_email_view_key(key).await?,
            AppMode::Help => self.handle_help_key(key).await?,
            AppMode::Confirm(_) => self.handle_confirm_key(key).await?,
        }
        Ok(())
    }

    async fn handle_normal_key(&mut self, key: KeyEvent) -> Result<()> {
        let keys = &self.config.keybindings;
        if keys.matches(Action::Down, key) {
            self.throttled_scroll(ScrollAction::Inbox(1));
            return Ok(());
        }
        if keys.matches(Action::Up, key) {
            self.throttled_scroll(ScrollAction::Inbox(-1));
            return Ok(());
        }
        if keys.matches(Action::Open, key) {
            self.open_selected_email();
            return Ok(());
        }
        if keys.matches(Action::Search, key) {
            self.enter_search_mode();
            return Ok(());
        }
        if keys.matches(Action::Compose, key) {
            self.enter_compose_mode(None);
            return Ok(());
        }
        if keys.matches(Action::Help, key) {
            self.state.mode = AppMode::Help;
            return Ok(());
        }
        if keys.matches(Action::Refresh, key) {
            self.refresh_category(true).await;
            return Ok(());
        }
        if keys.matches(Action::LoadMore, key) {
            self.load_more().await;
            return Ok(());
        }
        if keys.matches(Action::NextTab, key) {
            self.cycle_category(true).await;
            return Ok(());
        }
        if keys.matches(Action::PrevTab, key) {
            self.cycle_category(false).await;
            return Ok(());
        }
        if key.code == KeyCode::Char('s') {
            self.toggle_sort();
            return Ok(());
        }
        Ok(())
    }

    async fn handle_search_key(&mut self, key: KeyEvent) -> Result<()> {
        let keys = &self.config.keybindings;
        if key.code == KeyCode::Esc {
            self.exit_search_mode();
            return Ok(());
        }
        if keys.matches(Action::Down, key) {
            self.throttled_scroll(ScrollAction::Search(1));
            return Ok(());
        }
        if keys.matches(Action::Up, key) {
            self.throttled_scroll(ScrollAction::Search(-1));
            return Ok(());
        }
        match key.code {
            KeyCode::Backspace => {
                if self.state.search.cursor_pos > 0 {
                    self.state.search.cursor_pos -= 1;
                    self.state.search.query.remove(self.state.search.cursor_pos);
                    self.update_search_results();
                }
            }
            KeyCode::Enter => {
                self.open_search_selected();
            }
            KeyCode::Char(c) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                {
                    self.state
                        .search
                        .query
                        .insert(self.state.search.cursor_pos, c);
                    self.state.search.cursor_pos += 1;
                    self.update_search_results();
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_email_view_key(&mut self, key: KeyEvent) -> Result<()> {
        let keys = &self.config.keybindings;
        if key.code == KeyCode::Esc {
            self.state.mode = AppMode::Normal;
            return Ok(());
        }
        if keys.matches(Action::Help, key) {
            self.state.mode = AppMode::Help;
            return Ok(());
        }
        if keys.matches(Action::Reply, key) {
            if let Some(email) = self.state.current_email.clone() {
                self.enter_compose_mode(Some(email));
            }
            return Ok(());
        }
        if keys.matches(Action::Delete, key) {
            if let Some(email) = &self.state.current_email {
                self.state.mode = AppMode::Confirm(ConfirmAction::DeleteEmail(email.id.clone()));
            }
            return Ok(());
        }
        if keys.matches(Action::Down, key) {
            self.throttled_scroll(ScrollAction::Email(1));
            return Ok(());
        }
        if keys.matches(Action::Up, key) {
            self.throttled_scroll(ScrollAction::Email(-1));
            return Ok(());
        }
        Ok(())
    }

    async fn handle_compose_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.code == KeyCode::Esc {
            self.state.composer.reset();
            self.state.mode = AppMode::Normal;
            return Ok(());
        }
        if self.config.keybindings.matches(Action::Help, key) {
            self.state.mode = AppMode::Help;
            return Ok(());
        }
        if key.code == KeyCode::Tab {
            self.state.composer.cycle_field_forward();
            return Ok(());
        }
        if key.code == KeyCode::BackTab {
            self.state.composer.cycle_field_backward();
            return Ok(());
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
            if key.kind == KeyEventKind::Press {
                self.send_composed_email().await;
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Backspace => {
                let (buf, cursor) = self.state.composer.active_buffer_mut();
                if *cursor > 0 {
                    *cursor -= 1;
                    buf.remove(*cursor);
                }
            }
            KeyCode::Left => {
                let (_, cursor) = self.state.composer.active_buffer_mut();
                if *cursor > 0 {
                    *cursor -= 1;
                }
            }
            KeyCode::Right => {
                let (buf, cursor) = self.state.composer.active_buffer_mut();
                if *cursor < buf.len() {
                    *cursor += 1;
                }
            }
            KeyCode::Enter => {
                if self.state.composer.active_field == ComposerField::Body {
                    let (buf, cursor) = self.state.composer.active_buffer_mut();
                    buf.insert(*cursor, '\n');
                    *cursor += 1;
                }
            }
            KeyCode::Char(c) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT)
                {
                    let (buf, cursor) = self.state.composer.active_buffer_mut();
                    buf.insert(*cursor, c);
                    *cursor += 1;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_confirm_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('y') => {
                if let AppMode::Confirm(ConfirmAction::DeleteEmail(id)) = &self.state.mode {
                    self.delete_email(id.clone()).await;
                }
                self.state.mode = AppMode::Normal;
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.state.mode = AppMode::Normal;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_help_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.code == KeyCode::Esc || self.config.keybindings.matches(Action::Help, key) {
            self.state.mode = AppMode::Normal;
        }
        Ok(())
    }

    fn handle_app_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::EmailsLoaded {
                category,
                mut emails,
                next_page_token,
                append,
            } => {
                if let Some(mailbox) = self.state.mailbox_states.get_mut(&category) {
                    if append {
                        mailbox.emails.append(&mut emails);
                    } else {
                        mailbox.emails = emails;
                    }
                    crate::state::sort_emails(&mut mailbox.emails, mailbox.active_sort);
                    mailbox.next_page_token = next_page_token;
                    mailbox.has_more = mailbox.next_page_token.is_some();
                    mailbox.last_fetched = Some(Instant::now());
                    if mailbox.list_state.selected().is_none() && !mailbox.emails.is_empty() {
                        mailbox.list_state.select(Some(0));
                    }
                }
                self.state.is_loading = false;
                self.state.status_message =
                    Some(("Mailbox updated".to_string(), StatusLevel::Info));
                self.spawn_metadata_fetches(category);
            }
            AppEvent::EmailLoaded { message } => {
                self.state.current_email = Some(message.clone());
                self.state.email_view.scroll = 0;
                self.state.mode = AppMode::EmailView;
                self.state.is_loading = false;
                if let Some(mailbox) = self.state.mailbox_states.get_mut(&message.category) {
                    for email in &mut mailbox.emails {
                        if email.id == message.id {
                            email.is_read = true;
                        }
                    }
                }
            }
            AppEvent::MessageSent(result) => {
                self.state.is_loading = false;
                match result {
                    Ok(()) => {
                        self.state.status_message =
                            Some(("Message sent".to_string(), StatusLevel::Info));
                        self.state.mode = AppMode::Normal;
                        self.state.composer.reset();
                    }
                    Err(err) => {
                        self.state.status_message = Some((err.to_string(), StatusLevel::Error));
                    }
                }
            }
            AppEvent::MessageDeleted { id, result } => {
                self.state.is_loading = false;
                match result {
                    Ok(()) => {
                        if let Some(mailbox) = self
                            .state
                            .mailbox_states
                            .get_mut(&self.state.active_category)
                        {
                            mailbox.emails.retain(|email| email.id != id);
                        }
                        self.state.status_message =
                            Some(("Message deleted".to_string(), StatusLevel::Info));
                        self.state.mode = AppMode::Normal;
                        self.state.current_email = None;
                    }
                    Err(err) => {
                        self.state.status_message = Some((err.to_string(), StatusLevel::Error));
                    }
                }
            }
            AppEvent::Status { message, level } => {
                self.state.is_loading = false;
                self.state.status_message = Some((message, level));
            }
            AppEvent::EmailSummaryUpdated { category, summary } => {
                if let Some(mailbox) = self.state.mailbox_states.get_mut(&category) {
                    if let Some(existing) = mailbox.emails.iter_mut().find(|e| e.id == summary.id) {
                        existing.subject = if summary.subject.is_empty() {
                            summary.snippet.clone()
                        } else {
                            summary.subject
                        };
                        existing.from = summary.from;
                        existing.date = summary.date;
                        existing.snippet = summary.snippet;
                        existing.is_read = summary.is_read;
                        existing.category = category;
                    }
                    crate::state::sort_emails(&mut mailbox.emails, mailbox.active_sort);
                }
            }
        }
        Ok(())
    }

    async fn refresh_category(&mut self, force: bool) {
        let category = self.state.active_category;
        let cache_ttl = Duration::from_secs(self.config.cache_ttl_secs);
        let should_fetch = {
            if let Some(mailbox) = self.state.mailbox_states.get(&category) {
                if force {
                    true
                } else if let Some(last) = mailbox.last_fetched {
                    last.elapsed() >= cache_ttl
                } else {
                    true
                }
            } else {
                true
            }
        };
        if !should_fetch {
            self.state.status_message =
                Some(("Using cached mailbox".to_string(), StatusLevel::Info));
            return;
        }

        self.spawn_list_messages(category, None, false).await;
    }

    async fn load_more(&mut self) {
        self.state.status_message = Some((
            "Load more disabled (top 20 only)".to_string(),
            StatusLevel::Info,
        ));
    }

    async fn spawn_list_messages(
        &mut self,
        category: MailboxCategory,
        page_token: Option<String>,
        append: bool,
    ) {
        self.state.is_loading = true;
        let client = self.client.clone();
        let tx = self.tx.clone();
        let page_size = self.config.page_size.min(20);
        tokio::spawn(async move {
            let result = client
                .list_messages(category, page_token.as_deref(), page_size)
                .await;
            if let Some(tx) = tx {
                match result {
                    Ok((emails, next_token)) => {
                        let _ = tx.send(InputEvent::App(AppEvent::EmailsLoaded {
                            category,
                            emails,
                            next_page_token: next_token,
                            append,
                        }));
                    }
                    Err(err) => {
                        let _ = tx.send(InputEvent::App(AppEvent::Status {
                            message: err.to_string(),
                            level: StatusLevel::Error,
                        }));
                    }
                }
            }
        });
    }

    fn open_selected_email(&mut self) {
        let email = self
            .state
            .mailbox_states
            .get(&self.state.active_category)
            .and_then(|mailbox| {
                mailbox
                    .list_state
                    .selected()
                    .and_then(|idx| mailbox.emails.get(idx).cloned())
            });
        if let Some(summary) = email {
            self.request_open_email(summary.id.clone());
        }
    }

    fn open_search_selected(&mut self) {
        let idx = self.state.search.list_state.selected().unwrap_or(0);
        if let Some(summary) = self.state.search.results.get(idx) {
            self.request_open_email(summary.id.clone());
        }
    }

    fn request_open_email(&mut self, id: String) {
        self.state.is_loading = true;
        let client = self.client.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = client.get_message_full(&id).await;
            if let Ok(message) = &result {
                let _ = client.modify_labels(&message.id, &[], &["UNREAD"]).await;
            }
            if let Some(tx) = tx {
                match result {
                    Ok(message) => {
                        let _ = tx.send(InputEvent::App(AppEvent::EmailLoaded { message }));
                    }
                    Err(err) => {
                        let _ = tx.send(InputEvent::App(AppEvent::Status {
                            message: err.to_string(),
                            level: StatusLevel::Error,
                        }));
                    }
                }
            }
        });
    }

    fn spawn_metadata_fetches(&mut self, category: MailboxCategory) {
        let emails = self
            .state
            .mailbox_states
            .get(&category)
            .map(|m| m.emails.clone())
            .unwrap_or_default();
        if emails.is_empty() {
            return;
        }
        let visible_count = 10.min(emails.len());
        let immediate = emails
            .iter()
            .take(visible_count)
            .cloned()
            .collect::<Vec<_>>();
        let background = emails
            .iter()
            .skip(visible_count)
            .cloned()
            .collect::<Vec<_>>();

        for summary in immediate {
            self.spawn_metadata_fetch(category, summary.id, Duration::from_millis(0));
        }
        for (idx, summary) in background.into_iter().enumerate() {
            let delay = Duration::from_millis(150 * (idx as u64 + 1));
            self.spawn_metadata_fetch(category, summary.id, delay);
        }
    }

    fn spawn_metadata_fetch(&self, category: MailboxCategory, id: String, delay: Duration) {
        let client = self.client.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            if delay.as_millis() > 0 {
                tokio::time::sleep(delay).await;
            }
            let result = client.get_message_metadata(&id).await;
            if let Some(tx) = tx {
                match result {
                    Ok(mut summary) => {
                        summary.category = category;
                        let _ = tx.send(InputEvent::App(AppEvent::EmailSummaryUpdated {
                            category,
                            summary,
                        }));
                    }
                    Err(err) => {
                        let _ = tx.send(InputEvent::App(AppEvent::Status {
                            message: err.to_string(),
                            level: StatusLevel::Error,
                        }));
                    }
                }
            }
        });
    }

    fn enter_search_mode(&mut self) {
        self.state.search.reset();
        self.update_search_results();
        self.state.mode = AppMode::Search;
    }

    fn exit_search_mode(&mut self) {
        self.state.search.reset();
        self.state.mode = AppMode::Normal;
    }

    fn update_search_results(&mut self) {
        if let Some(mailbox) = self.state.mailbox_states.get(&self.state.active_category) {
            self.state.search.apply_filter(&mailbox.emails);
        }
    }

    fn move_selection(&mut self, delta: i32) {
        if let Some(mailbox) = self
            .state
            .mailbox_states
            .get_mut(&self.state.active_category)
        {
            let len = mailbox.emails.len();
            if len == 0 {
                mailbox.list_state.select(None);
                return;
            }
            let selected = mailbox.list_state.selected().unwrap_or(0) as i32;
            let next = (selected + delta).clamp(0, (len - 1) as i32) as usize;
            mailbox.list_state.select(Some(next));
        }
    }

    fn move_search_selection(&mut self, delta: i32) {
        let len = self.state.search.results.len();
        if len == 0 {
            self.state.search.list_state.select(None);
            return;
        }
        let selected = self.state.search.list_state.selected().unwrap_or(0) as i32;
        let next = (selected + delta).clamp(0, (len - 1) as i32) as usize;
        self.state.search.list_state.select(Some(next));
    }

    async fn cycle_category(&mut self, forward: bool) {
        let categories = all_categories();
        let idx = categories
            .iter()
            .position(|c| *c == self.state.active_category)
            .unwrap_or(0);
        let next = if forward {
            (idx + 1) % categories.len()
        } else if idx == 0 {
            categories.len() - 1
        } else {
            idx - 1
        };
        self.state.active_category = categories[next];
        self.refresh_category(false).await;
    }

    fn enter_compose_mode(&mut self, reply_to: Option<GmailMessage>) {
        if let Some(message) = reply_to.as_ref() {
            self.state.composer = crate::state::ComposerState::from_reply(message);
        } else {
            self.state.composer.reset();
        }
        self.state.mode = AppMode::Compose;
    }

    async fn send_composed_email(&mut self) {
        if self.state.composer.to.trim().is_empty() {
            self.state.status_message =
                Some(("Recipient is empty".to_string(), StatusLevel::Warning));
            return;
        }
        let mime = build_mime_message(&self.state.composer);
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mime);
        self.state.is_loading = true;
        let client = self.client.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = client.send_message(raw).await;
            if let Some(tx) = tx {
                let _ = tx.send(InputEvent::App(AppEvent::MessageSent(result)));
            }
        });
    }

    async fn delete_email(&mut self, id: String) {
        self.state.is_loading = true;
        let client = self.client.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = client.delete_message(&id).await;
            if let Some(tx) = tx {
                let _ = tx.send(InputEvent::App(AppEvent::MessageDeleted { id, result }));
            }
        });
    }

    fn toggle_sort(&mut self) {
        if let Some(mailbox) = self
            .state
            .mailbox_states
            .get_mut(&self.state.active_category)
        {
            mailbox.active_sort = match mailbox.active_sort {
                SortOrder::DateDesc => SortOrder::DateAsc,
                SortOrder::DateAsc => SortOrder::SenderAsc,
                SortOrder::SenderAsc => SortOrder::SubjectAsc,
                SortOrder::SubjectAsc => SortOrder::DateDesc,
            };
            crate::state::sort_emails(&mut mailbox.emails, mailbox.active_sort);
        }
    }

    fn throttled_scroll(&mut self, action: ScrollAction) {
        match action {
            ScrollAction::Inbox(delta) => self.move_selection(delta),
            ScrollAction::Search(delta) => self.move_search_selection(delta),
            ScrollAction::Email(delta) => {
                if delta > 0 {
                    self.state.email_view.scroll =
                        self.state.email_view.scroll.saturating_add(delta as u16);
                } else {
                    self.state.email_view.scroll =
                        self.state.email_view.scroll.saturating_sub((-delta) as u16);
                }
            }
        }
    }

    fn should_throttle_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::ALT)
        {
            return false;
        }
        let now = Instant::now();
        if now.duration_since(self.last_action_at) < Duration::from_millis(10) {
            return true;
        }
        self.last_action_at = now;
        false
    }
}

fn spawn_input_thread(tx: UnboundedSender<InputEvent>) {
    std::thread::spawn(move || loop {
        if let Ok(true) = crossterm::event::poll(Duration::from_millis(50)) {
            if let Ok(event) = crossterm::event::read() {
                let _ = tx.send(InputEvent::Input(event));
            }
        }
    });
}

fn build_mime_message(composer: &crate::state::ComposerState) -> String {
    format!(
        "To: {}\r\nSubject: {}\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\n\r\n{}\r\n",
        composer.to.trim(),
        composer.subject.trim(),
        composer.body
    )
}
