use crate::gmail::auth::GmailAuth;
use crate::gmail::models::{GmailMessage, MailboxCategory};
use crate::state::EmailSummary;
use anyhow::{anyhow, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::time::Duration;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

pub trait GmailApi: Send + Sync {
    fn list_messages<'a>(
        &'a self,
        category: MailboxCategory,
        page_token: Option<&'a str>,
        max_results: usize,
    ) -> BoxFuture<'a, (Vec<EmailSummary>, Option<String>)>;

    fn get_message_full<'a>(&'a self, id: &'a str) -> BoxFuture<'a, GmailMessage>;

    fn send_message<'a>(&'a self, raw_mime: String) -> BoxFuture<'a, ()>;

    fn delete_message<'a>(&'a self, id: &'a str) -> BoxFuture<'a, ()>;

    fn modify_labels<'a>(
        &'a self,
        id: &'a str,
        add: &'a [&'a str],
        remove: &'a [&'a str],
    ) -> BoxFuture<'a, ()>;
}

pub struct GmailClient {
    pub http: reqwest::Client,
    pub auth: Mutex<GmailAuth>,
    pub base_url: &'static str,
}

impl GmailClient {
    pub fn new(auth: GmailAuth) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(15))
            .pool_max_idle_per_host(10)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            http,
            auth: Mutex::new(auth),
            base_url: "https://gmail.googleapis.com/gmail/v1",
        }
    }

    pub async fn list_messages(
        &self,
        category: MailboxCategory,
        page_token: Option<&str>,
        max_results: usize,
    ) -> Result<(Vec<EmailSummary>, Option<String>)> {
        let mut url = format!(
            "{}/users/me/messages?labelIds=INBOX&q=category:{}&maxResults={}&fields=messages(id,threadId),nextPageToken",
            self.base_url,
            category.query_name(),
            max_results
        );
        if let Some(token) = page_token {
            url.push_str("&pageToken=");
            url.push_str(token);
        }

        let token = self.authenticate()?;
        let resp = self.http.get(url).bearer_auth(&token).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("list messages failed: {status} {body}"));
        }

        let body = resp.json::<MessagesList>().await?;
        let mut emails = Vec::new();
        if let Some(messages) = body.messages {
            for msg in messages {
                emails.push(EmailSummary {
                    id: msg.id,
                    thread_id: msg.thread_id.unwrap_or_default(),
                    subject: String::new(),
                    from: String::new(),
                    date: Utc::now(),
                    snippet: String::new(),
                    is_read: false,
                    category,
                });
            }
        }
        Ok((emails, body.next_page_token))
    }

    pub async fn get_message_full(&self, id: &str) -> Result<GmailMessage> {
        let url = format!("{}/users/me/messages/{}?format=full", self.base_url, id);
        let token = self.authenticate()?;
        let resp = self.http.get(url).bearer_auth(token).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("get message failed: {status} {body}"));
        }

        let msg = resp.json::<MessageDetail>().await?;
        let headers = &msg.payload.headers;
        let subject = header_value(headers, "Subject");
        let from = header_value(headers, "From");
        let to = header_value(headers, "To");
        let date_str = header_value(headers, "Date");
        let date = parse_date(&date_str);
        let labels = msg.label_ids.unwrap_or_default();
        let category = MailboxCategory::from_labels(&labels);
        let is_read = !labels.iter().any(|l| l == "UNREAD");

        let body = extract_body(&msg.payload).unwrap_or_default();

        Ok(GmailMessage {
            id: msg.id,
            thread_id: msg.thread_id,
            subject,
            from,
            to,
            date,
            snippet: msg.snippet.unwrap_or_default(),
            body,
            is_read,
            labels,
            category,
        })
    }

    pub async fn send_message(&self, raw_mime: String) -> Result<()> {
        let url = format!("{}/users/me/messages/send", self.base_url);
        let token = self.authenticate()?;
        let resp = self
            .http
            .post(url)
            .bearer_auth(token)
            .json(&serde_json::json!({"raw": raw_mime}))
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("send message failed: {status} {body}"));
        }
        Ok(())
    }

    pub async fn delete_message(&self, id: &str) -> Result<()> {
        let url = format!("{}/users/me/messages/{}", self.base_url, id);
        let token = self.authenticate()?;
        let resp = self.http.delete(url).bearer_auth(token).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("delete message failed: {status} {body}"));
        }
        Ok(())
    }

    pub async fn modify_labels(&self, id: &str, add: &[&str], remove: &[&str]) -> Result<()> {
        let url = format!("{}/users/me/messages/{}/modify", self.base_url, id);
        let token = self.authenticate()?;
        let resp = self
            .http
            .post(url)
            .bearer_auth(token)
            .json(&serde_json::json!({
                "addLabelIds": add,
                "removeLabelIds": remove
            }))
            .send()
            .await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("modify labels failed: {status} {body}"));
        }
        Ok(())
    }

    pub async fn get_message_metadata(&self, id: &str) -> Result<EmailSummary> {
        let url = format!(
            "{}/users/me/messages/{}?format=metadata&metadataHeaders=Subject&metadataHeaders=From&metadataHeaders=Date",
            self.base_url, id
        );
        let token = self.authenticate()?;
        let resp = self.http.get(url).bearer_auth(token).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("get metadata failed: {status} {body}"));
        }

        let msg = resp.json::<MessageDetail>().await?;
        let headers = &msg.payload.headers;
        let subject = header_value(headers, "Subject");
        let from = header_value(headers, "From");
        let date_str = header_value(headers, "Date");
        let date = parse_date(&date_str);
        let labels = msg.label_ids.unwrap_or_default();
        let category = MailboxCategory::from_labels(&labels);
        let is_read = !labels.iter().any(|l| l == "UNREAD");

        Ok(EmailSummary {
            id: msg.id,
            thread_id: msg.thread_id,
            subject,
            from,
            date,
            snippet: msg.snippet.unwrap_or_default(),
            is_read,
            category,
        })
    }

    fn authenticate(&self) -> Result<String> {
        let mut auth = self
            .auth
            .lock()
            .map_err(|_| anyhow!("auth lock poisoned"))?;
        Ok(auth.authenticate()?)
    }
}

impl GmailApi for GmailClient {
    fn list_messages<'a>(
        &'a self,
        category: MailboxCategory,
        page_token: Option<&'a str>,
        max_results: usize,
    ) -> BoxFuture<'a, (Vec<EmailSummary>, Option<String>)> {
        Box::pin(async move {
            GmailClient::list_messages(self, category, page_token, max_results).await
        })
    }

    fn get_message_full<'a>(&'a self, id: &'a str) -> BoxFuture<'a, GmailMessage> {
        Box::pin(async move { GmailClient::get_message_full(self, id).await })
    }

    fn send_message<'a>(&'a self, raw_mime: String) -> BoxFuture<'a, ()> {
        Box::pin(async move { GmailClient::send_message(self, raw_mime).await })
    }

    fn delete_message<'a>(&'a self, id: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async move { GmailClient::delete_message(self, id).await })
    }

    fn modify_labels<'a>(
        &'a self,
        id: &'a str,
        add: &'a [&'a str],
        remove: &'a [&'a str],
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move { GmailClient::modify_labels(self, id, add, remove).await })
    }
}

#[derive(Debug, Deserialize)]
struct MessagesList {
    messages: Option<Vec<MessageId>>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageId {
    id: String,
    #[serde(rename = "threadId")]
    thread_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageDetail {
    id: String,
    #[serde(rename = "threadId")]
    thread_id: String,
    snippet: Option<String>,
    #[serde(rename = "labelIds")]
    label_ids: Option<Vec<String>>,
    payload: Payload,
}

#[derive(Debug, Deserialize)]
struct Payload {
    #[serde(rename = "mimeType")]
    mime_type: String,
    headers: Vec<Header>,
    body: Body,
    parts: Option<Vec<Payload>>,
}

#[derive(Debug, Deserialize)]
struct Header {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct Body {
    data: Option<String>,
}

fn header_value(headers: &[Header], name: &str) -> String {
    headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case(name))
        .map(|h| h.value.clone())
        .unwrap_or_default()
}

fn parse_date(input: &str) -> DateTime<Utc> {
    if let Ok(parsed) = DateTime::parse_from_rfc2822(input) {
        return parsed.with_timezone(&Utc);
    }
    if let Ok(parsed) = DateTime::parse_from_rfc3339(input) {
        return parsed.with_timezone(&Utc);
    }
    Utc::now()
}

fn extract_body(payload: &Payload) -> Option<String> {
    let mut plain = None;
    let mut html = None;
    collect_bodies(payload, &mut plain, &mut html);

    if let Some(data) = plain {
        return decode_body(&data);
    }
    if let Some(data) = html {
        return decode_body(&data).map(strip_html);
    }
    None
}

fn collect_bodies(payload: &Payload, plain: &mut Option<String>, html: &mut Option<String>) {
    if let Some(data) = &payload.body.data {
        match payload.mime_type.as_str() {
            "text/plain" => {
                if plain.is_none() {
                    *plain = Some(data.clone());
                }
            }
            "text/html" => {
                if html.is_none() {
                    *html = Some(data.clone());
                }
            }
            _ => {}
        }
    }
    if let Some(parts) = &payload.parts {
        for part in parts {
            collect_bodies(part, plain, html);
        }
    }
}

fn decode_body(data: &str) -> Option<String> {
    URL_SAFE_NO_PAD
        .decode(data.as_bytes())
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

fn strip_html(input: String) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ => {
                if !in_tag {
                    out.push(ch);
                }
            }
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}
