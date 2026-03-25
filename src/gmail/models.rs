use chrono::{DateTime, Utc};
use std::fmt;

#[derive(Debug, Clone)]
pub struct GmailMessage {
    pub id: String,
    pub thread_id: String,
    pub subject: String,
    pub from: String,
    pub to: String,
    pub date: DateTime<Utc>,
    pub snippet: String,
    pub body: String,
    pub is_read: bool,
    pub labels: Vec<String>,
    pub category: MailboxCategory,
}

#[derive(Debug, Clone)]
pub struct GmailThread {
    pub id: String,
    pub messages: Vec<GmailMessage>,
    pub snippet: String,
}

#[derive(Debug, Clone)]
pub struct Label {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MailboxCategory {
    Primary,
    Social,
    Promotions,
    Updates,
    Forums,
}

impl MailboxCategory {
    pub fn from_labels(labels: &[String]) -> Self {
        for label in labels {
            match label.as_str() {
                "CATEGORY_SOCIAL" => return MailboxCategory::Social,
                "CATEGORY_PROMOTIONS" => return MailboxCategory::Promotions,
                "CATEGORY_UPDATES" => return MailboxCategory::Updates,
                "CATEGORY_FORUMS" => return MailboxCategory::Forums,
                _ => {}
            }
        }
        MailboxCategory::Primary
    }

    pub fn query_name(&self) -> &'static str {
        match self {
            MailboxCategory::Primary => "primary",
            MailboxCategory::Social => "social",
            MailboxCategory::Promotions => "promotions",
            MailboxCategory::Updates => "updates",
            MailboxCategory::Forums => "forums",
        }
    }
}

impl fmt::Display for MailboxCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MailboxCategory::Primary => "Primary",
            MailboxCategory::Social => "Social",
            MailboxCategory::Promotions => "Promos",
            MailboxCategory::Updates => "Updates",
            MailboxCategory::Forums => "Forums",
        };
        write!(f, "{s}")
    }
}
