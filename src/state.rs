use crate::gmail::models::GmailMessage;
use chrono::{DateTime, Utc};
use ratatui::widgets::ListState;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum AppMode {
    Normal,
    Search,
    Compose,
    EmailView,
    Help,
    Confirm(ConfirmAction),
}

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    DeleteEmail(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    DateDesc,
    DateAsc,
    SenderAsc,
    SubjectAsc,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub mode: AppMode,
    pub active_category: MailboxCategory,
    pub mailbox_states: HashMap<MailboxCategory, MailboxState>,
    pub search: SearchState,
    pub status_message: Option<(String, StatusLevel)>,
    pub is_loading: bool,
    pub current_email: Option<GmailMessage>,
    pub email_view: EmailViewState,
    pub composer: ComposerState,
}

#[derive(Debug, Clone)]
pub struct MailboxState {
    pub emails: Vec<EmailSummary>,
    pub list_state: ListState,
    pub next_page_token: Option<String>,
    pub has_more: bool,
    pub last_fetched: Option<Instant>,
    pub active_sort: SortOrder,
}

#[derive(Debug, Clone)]
pub struct SearchState {
    pub query: String,
    pub cursor_pos: usize,
    pub results: Vec<EmailSummary>,
    pub list_state: ListState,
}

#[derive(Debug, Clone)]
pub struct EmailSummary {
    pub id: String,
    pub thread_id: String,
    pub subject: String,
    pub from: String,
    pub date: DateTime<Utc>,
    pub snippet: String,
    pub is_read: bool,
    pub category: MailboxCategory,
}

pub use crate::gmail::models::MailboxCategory;

#[derive(Debug, Clone)]
pub struct EmailViewState {
    pub scroll: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposerField {
    To,
    Subject,
    Body,
}

#[derive(Debug, Clone)]
pub struct ComposerState {
    pub to: String,
    pub subject: String,
    pub body: String,
    pub cursor_to: usize,
    pub cursor_subject: usize,
    pub cursor_body: usize,
    pub active_field: ComposerField,
}

#[derive(Debug, Clone)]
pub struct CacheState {
    pub ttl: Duration,
}

impl Default for AppState {
    fn default() -> Self {
        let mut mailbox_states = HashMap::new();
        for category in all_categories() {
            mailbox_states.insert(category, MailboxState::default());
        }
        Self {
            mode: AppMode::Normal,
            active_category: MailboxCategory::Primary,
            mailbox_states,
            search: SearchState::default(),
            status_message: None,
            is_loading: false,
            current_email: None,
            email_view: EmailViewState { scroll: 0 },
            composer: ComposerState::new(),
        }
    }
}

impl Default for MailboxState {
    fn default() -> Self {
        let mut list_state = ListState::default();
        list_state.select(None);
        Self {
            emails: Vec::new(),
            list_state,
            next_page_token: None,
            has_more: false,
            last_fetched: None,
            active_sort: SortOrder::DateDesc,
        }
    }
}

impl Default for SearchState {
    fn default() -> Self {
        let mut list_state = ListState::default();
        list_state.select(None);
        Self {
            query: String::new(),
            cursor_pos: 0,
            results: Vec::new(),
            list_state,
        }
    }
}

impl ComposerState {
    pub fn new() -> Self {
        Self {
            to: String::new(),
            subject: String::new(),
            body: String::new(),
            cursor_to: 0,
            cursor_subject: 0,
            cursor_body: 0,
            active_field: ComposerField::To,
        }
    }

    pub fn from_reply(original: &GmailMessage) -> Self {
        let subject = if original.subject.to_lowercase().starts_with("re:") {
            original.subject.clone()
        } else {
            format!("Re: {}", original.subject)
        };
        let quoted = original
            .body
            .lines()
            .map(|line| format!("> {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        Self {
            to: original.from.clone(),
            subject,
            body: quoted,
            cursor_to: original.from.len(),
            cursor_subject: 0,
            cursor_body: 0,
            active_field: ComposerField::Body,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn active_buffer_mut(&mut self) -> (&mut String, &mut usize) {
        match self.active_field {
            ComposerField::To => (&mut self.to, &mut self.cursor_to),
            ComposerField::Subject => (&mut self.subject, &mut self.cursor_subject),
            ComposerField::Body => (&mut self.body, &mut self.cursor_body),
        }
    }

    pub fn cycle_field_forward(&mut self) {
        self.active_field = match self.active_field {
            ComposerField::To => ComposerField::Subject,
            ComposerField::Subject => ComposerField::Body,
            ComposerField::Body => ComposerField::To,
        };
    }

    pub fn cycle_field_backward(&mut self) {
        self.active_field = match self.active_field {
            ComposerField::To => ComposerField::Body,
            ComposerField::Subject => ComposerField::To,
            ComposerField::Body => ComposerField::Subject,
        };
    }
}

impl SearchState {
    pub fn apply_filter(&mut self, emails: &[EmailSummary]) {
        if self.query.is_empty() {
            self.results = emails.to_vec();
            self.list_state.select(if self.results.is_empty() {
                None
            } else {
                Some(0)
            });
            return;
        }
        let needle = self.query.to_lowercase();
        self.results = emails
            .iter()
            .filter(|email| {
                let haystack = format!(
                    "{} {} {}",
                    email.subject.to_lowercase(),
                    email.from.to_lowercase(),
                    email.snippet.to_lowercase()
                );
                haystack.contains(&needle)
            })
            .cloned()
            .collect();
        self.list_state.select(if self.results.is_empty() {
            None
        } else {
            Some(0)
        });
    }

    pub fn reset(&mut self) {
        self.query.clear();
        self.cursor_pos = 0;
        self.results.clear();
        self.list_state.select(None);
    }
}

pub fn all_categories() -> Vec<MailboxCategory> {
    vec![
        MailboxCategory::Primary,
        MailboxCategory::Social,
        MailboxCategory::Promotions,
        MailboxCategory::Updates,
        MailboxCategory::Forums,
    ]
}

pub fn sort_emails(emails: &mut [EmailSummary], order: SortOrder) {
    match order {
        SortOrder::DateDesc => {
            emails.sort_by(|a, b| b.date.cmp(&a.date));
        }
        SortOrder::DateAsc => {
            emails.sort_by(|a, b| a.date.cmp(&b.date));
        }
        SortOrder::SenderAsc => {
            emails.sort_by(|a, b| a.from.to_lowercase().cmp(&b.from.to_lowercase()));
        }
        SortOrder::SubjectAsc => {
            emails.sort_by(|a, b| a.subject.to_lowercase().cmp(&b.subject.to_lowercase()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gmail::client::{BoxFuture, GmailApi};
    use anyhow::Result;
    use chrono::TimeZone;
    use std::sync::{Arc, Mutex};

    struct FakeGmailClient {
        pages: Arc<Mutex<Vec<Vec<EmailSummary>>>>,
        calls: Arc<Mutex<usize>>,
    }

    impl FakeGmailClient {
        fn new(pages: Vec<Vec<EmailSummary>>) -> Self {
            Self {
                pages: Arc::new(Mutex::new(pages)),
                calls: Arc::new(Mutex::new(0)),
            }
        }

        fn call_count(&self) -> usize {
            *self.calls.lock().unwrap()
        }
    }

    impl GmailApi for FakeGmailClient {
        fn list_messages<'a>(
            &'a self,
            _category: MailboxCategory,
            _page_token: Option<&'a str>,
            _max_results: usize,
        ) -> BoxFuture<'a, (Vec<EmailSummary>, Option<String>)> {
            Box::pin(async move {
                let mut calls = self.calls.lock().unwrap();
                *calls += 1;
                drop(calls);
                let mut pages = self.pages.lock().unwrap();
                if pages.is_empty() {
                    return Ok((Vec::new(), None));
                }
                let page = pages.remove(0);
                let next = if pages.is_empty() {
                    None
                } else {
                    Some("next".to_string())
                };
                Ok((page, next))
            })
        }

        fn get_message_full<'a>(&'a self, _id: &'a str) -> BoxFuture<'a, GmailMessage> {
            Box::pin(async move { Err(anyhow::anyhow!("not used")) })
        }

        fn send_message<'a>(&'a self, _raw_mime: String) -> BoxFuture<'a, ()> {
            Box::pin(async move { Ok(()) })
        }

        fn delete_message<'a>(&'a self, _id: &'a str) -> BoxFuture<'a, ()> {
            Box::pin(async move { Ok(()) })
        }

        fn modify_labels<'a>(
            &'a self,
            _id: &'a str,
            _add: &'a [&'a str],
            _remove: &'a [&'a str],
        ) -> BoxFuture<'a, ()> {
            Box::pin(async move { Ok(()) })
        }
    }

    async fn load_more_with_client(
        client: &dyn GmailApi,
        state: &mut MailboxState,
        category: MailboxCategory,
        page_size: usize,
    ) -> Result<()> {
        let (mut emails, next_token) = client
            .list_messages(category, state.next_page_token.as_deref(), page_size)
            .await?;
        state.emails.append(&mut emails);
        state.next_page_token = next_token;
        state.has_more = state.next_page_token.is_some();
        Ok(())
    }

    async fn refresh_with_cache(
        client: &dyn GmailApi,
        state: &mut MailboxState,
        category: MailboxCategory,
        page_size: usize,
        cache_ttl: Duration,
        force: bool,
    ) -> Result<bool> {
        if !force {
            if let Some(last) = state.last_fetched {
                if last.elapsed() < cache_ttl {
                    return Ok(false);
                }
            }
        }
        let (emails, next_token) = client.list_messages(category, None, page_size).await?;
        state.emails = emails;
        state.next_page_token = next_token;
        state.has_more = state.next_page_token.is_some();
        state.last_fetched = Some(Instant::now());
        Ok(true)
    }

    fn sample_email(id: &str, from: &str, subject: &str, ts: i64) -> EmailSummary {
        EmailSummary {
            id: id.to_string(),
            thread_id: format!("t-{id}"),
            subject: subject.to_string(),
            from: from.to_string(),
            date: Utc.timestamp_opt(ts, 0).unwrap(),
            snippet: "".to_string(),
            is_read: true,
            category: MailboxCategory::Primary,
        }
    }

    #[tokio::test]
    async fn test_lazy_load_appends_emails() {
        let page1 = vec![sample_email("1", "a", "one", 1)];
        let page2 = vec![sample_email("2", "b", "two", 2)];
        let client = FakeGmailClient::new(vec![page1, page2]);
        let mut state = MailboxState::default();
        load_more_with_client(&client, &mut state, MailboxCategory::Primary, 1)
            .await
            .unwrap();
        let before = state.emails.len();
        load_more_with_client(&client, &mut state, MailboxCategory::Primary, 1)
            .await
            .unwrap();
        let after = state.emails.len();
        assert_eq!(before + 1, after);
    }

    #[tokio::test]
    async fn test_cache_skips_fetch_within_ttl() {
        let page1 = vec![sample_email("1", "a", "one", 1)];
        let client = FakeGmailClient::new(vec![page1]);
        let mut state = MailboxState::default();
        let ttl = Duration::from_secs(300);
        refresh_with_cache(&client, &mut state, MailboxCategory::Primary, 1, ttl, false)
            .await
            .unwrap();
        let fetched =
            refresh_with_cache(&client, &mut state, MailboxCategory::Primary, 1, ttl, false)
                .await
                .unwrap();
        assert!(!fetched);
        assert_eq!(client.call_count(), 1);
    }

    #[test]
    fn test_search_filters_correctly() {
        let mut search = SearchState::default();
        search.query = "rust".to_string();
        let emails = vec![
            sample_email("1", "a", "Rust newsletter", 1),
            sample_email("2", "b", "Python weekly", 2),
        ];
        search.apply_filter(&emails);
        assert_eq!(search.results.len(), 1);
        assert_eq!(search.results[0].subject, "Rust newsletter");
    }

    #[test]
    fn test_sort_date_desc() {
        let mut emails = vec![
            sample_email("1", "a", "one", 10),
            sample_email("2", "b", "two", 20),
        ];
        sort_emails(&mut emails, SortOrder::DateDesc);
        assert_eq!(emails[0].id, "2");
    }

    #[test]
    fn test_sort_sender_asc() {
        let mut emails = vec![
            sample_email("1", "bob", "one", 10),
            sample_email("2", "alice", "two", 20),
        ];
        sort_emails(&mut emails, SortOrder::SenderAsc);
        assert_eq!(emails[0].from, "alice");
    }

    #[test]
    fn test_compose_reply_prefill() {
        let msg = GmailMessage {
            id: "1".to_string(),
            thread_id: "t1".to_string(),
            subject: "Hello".to_string(),
            from: "sender@example.com".to_string(),
            to: "me@example.com".to_string(),
            date: Utc::now(),
            snippet: "".to_string(),
            body: "Hi there".to_string(),
            is_read: true,
            labels: vec![],
            category: MailboxCategory::Primary,
        };
        let composer = ComposerState::from_reply(&msg);
        assert_eq!(composer.to, "sender@example.com");
        assert!(composer.subject.starts_with("Re: "));
    }
}
