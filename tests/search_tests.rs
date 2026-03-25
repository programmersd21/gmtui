use chrono::Utc;
use gmtui::gmail::models::MailboxCategory;
use gmtui::state::{EmailSummary, SearchState};

fn make_summary(subject: &str, from: &str) -> EmailSummary {
    EmailSummary {
        id: "id".to_string(),
        thread_id: "thread".to_string(),
        subject: subject.to_string(),
        from: from.to_string(),
        date: Utc::now(),
        snippet: "snippet".to_string(),
        is_read: false,
        category: MailboxCategory::Primary,
    }
}

#[test]
fn search_subject_match() {
    let emails = vec![
        make_summary("Buy now 50% off", "promo@example.com"),
        make_summary("Meeting notes", "team@example.com"),
        make_summary("GitHub PR review", "no-reply@github.com"),
    ];
    let mut state = SearchState::default();
    state.query = "meeting".to_string();

    state.apply_filter(&emails);
    assert_eq!(state.results.len(), 1);
    assert_eq!(state.results[0].subject, "Meeting notes");
}

#[test]
fn search_sender_match() {
    let emails = vec![
        make_summary("Hello", "alice@example.com"),
        make_summary("PR update", "no-reply@github.com"),
        make_summary("Status", "boss@work.com"),
    ];
    let mut state = SearchState::default();
    state.query = "github".to_string();

    state.apply_filter(&emails);
    assert_eq!(state.results.len(), 1);
    assert_eq!(state.results[0].from, "no-reply@github.com");
}

#[test]
fn search_empty_query_returns_all() {
    let emails = vec![
        make_summary("One", "a@example.com"),
        make_summary("Two", "b@example.com"),
        make_summary("Three", "c@example.com"),
    ];
    let mut state = SearchState::default();
    state.query = "".to_string();

    state.apply_filter(&emails);
    assert_eq!(state.results.len(), 3);
}

#[test]
fn search_case_insensitive() {
    let emails = vec![
        make_summary("Meeting notes", "team@example.com"),
        make_summary("Other", "other@example.com"),
    ];
    let mut state = SearchState::default();
    state.query = "MEETING".to_string();

    state.apply_filter(&emails);
    assert_eq!(state.results.len(), 1);
    assert_eq!(state.results[0].subject, "Meeting notes");
}

#[test]
fn search_no_match() {
    let emails = vec![
        make_summary("Hello", "alice@example.com"),
        make_summary("World", "bob@example.com"),
    ];
    let mut state = SearchState::default();
    state.query = "xyzzy_nomatch_123".to_string();

    state.apply_filter(&emails);
    assert_eq!(state.results.len(), 0);
}
