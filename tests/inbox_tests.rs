use chrono::Utc;
use gmtui::gmail::models::MailboxCategory;
use gmtui::state::{AppMode, AppState, EmailSummary, MailboxState, SortOrder};

fn make_summary(category: MailboxCategory) -> EmailSummary {
    EmailSummary {
        id: "id".to_string(),
        thread_id: "thread".to_string(),
        subject: "Subject".to_string(),
        from: "from@example.com".to_string(),
        date: Utc::now(),
        snippet: "snippet".to_string(),
        is_read: false,
        category,
    }
}

#[test]
fn category_filtering() {
    let emails = [
        make_summary(MailboxCategory::Primary),
        make_summary(MailboxCategory::Social),
        make_summary(MailboxCategory::Social),
        make_summary(MailboxCategory::Updates),
    ];

    let filtered: Vec<_> = emails
        .iter()
        .filter(|e| e.category == MailboxCategory::Social)
        .collect();
    assert_eq!(filtered.len(), 2);
}

#[test]
fn mailbox_default_sort() {
    let mailbox = MailboxState::default();
    assert_eq!(mailbox.active_sort, SortOrder::DateDesc);
}

#[test]
fn app_state_defaults() {
    let state = AppState::default();
    assert_eq!(state.active_category, MailboxCategory::Primary);
    assert!(matches!(state.mode, AppMode::Normal));
}
