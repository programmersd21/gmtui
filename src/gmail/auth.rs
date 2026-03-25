use crate::config::AuthConfig;
use crate::error::{GmtuiError, Result};
use chrono::{DateTime, Duration, Utc};
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use tokio::runtime::Handle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStore {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
}

impl TokenStore {
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(|e| {
            GmtuiError::Auth(format!(
                "failed to read token store at {}: {e}",
                path.display()
            ))
        })?;
        let store = serde_json::from_str(&content)?;
        Ok(store)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| GmtuiError::Auth(format!("failed to create token directory: {e}")))?;
        }
        let content = serde_json::to_string_pretty(self)?;
        let mut file = fs::File::create(path).map_err(|e| {
            GmtuiError::Auth(format!(
                "failed to write token store at {}: {e}",
                path.display()
            ))
        })?;
        file.write_all(content.as_bytes())
            .map_err(|e| GmtuiError::Auth(format!("failed to write token store: {e}")))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
        }

        Ok(())
    }

    pub fn is_expired(&self) -> bool {
        let buffer = Duration::seconds(60);
        Utc::now() + buffer >= self.expires_at
    }
}

pub struct GmailAuth {
    client: BasicClient,
    token: Option<TokenStore>,
    token_path: PathBuf,
}

impl GmailAuth {
    pub fn new(config: &AuthConfig) -> Result<Self> {
        let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .map_err(|e| GmtuiError::Auth(format!("invalid auth url: {e}")))?;
        let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
            .map_err(|e| GmtuiError::Auth(format!("invalid token url: {e}")))?;

        let client = BasicClient::new(
            ClientId::new(config.client_id.clone()),
            Some(ClientSecret::new(config.client_secret.clone())),
            auth_url,
            Some(token_url),
        )
        .set_redirect_uri(
            RedirectUrl::new("http://127.0.0.1:8080/".to_string())
                .map_err(|e| GmtuiError::Auth(format!("invalid redirect url: {e}")))?,
        );

        let token = TokenStore::load(&config.token_cache_path).ok();

        Ok(Self {
            client,
            token,
            token_path: config.token_cache_path.clone(),
        })
    }

    pub fn authenticate(&mut self) -> Result<String> {
        if let Some(token) = &self.token {
            if !token.is_expired() {
                return Ok(token.access_token.clone());
            }
        }

        if self.token.is_some() {
            self.refresh_token()?;
            if let Some(token) = &self.token {
                return Ok(token.access_token.clone());
            }
        }

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let (auth_url, csrf_token) = self
            .client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/gmail.readonly".to_string(),
            ))
            .add_scope(Scope::new(
                "https://www.googleapis.com/auth/gmail.modify".to_string(),
            ))
            .add_extra_param("access_type", "offline")
            .add_extra_param("prompt", "consent")
            .set_pkce_challenge(pkce_challenge)
            .url();

        println!("\n=== GMTUI AUTHENTICATION ===");
        println!("1. Open this URL in your browser:\n\n{auth_url}\n");
        println!("2. Log in and authorize the application.");
        println!("3. After redirecting, the code will be captured automatically.\n");

        let (code, returned_state) = wait_for_auth_code()?;

        if returned_state != *csrf_token.secret() {
            return Err(GmtuiError::Auth("CSRF validation failed".to_string()));
        }

        self.exchange_code(code, pkce_verifier)?;

        self.token
            .as_ref()
            .map(|t| t.access_token.clone())
            .ok_or_else(|| GmtuiError::Auth("missing access token after auth".to_string()))
    }

    pub fn refresh_token(&mut self) -> Result<()> {
        let refresh = self
            .token
            .as_ref()
            .map(|t| t.refresh_token.clone())
            .ok_or_else(|| GmtuiError::Auth("missing refresh token".to_string()))?;

        let token_result = run_oauth_request(|| async {
            self.client
                .exchange_refresh_token(&RefreshToken::new(refresh))
                .request_async(async_http_client)
                .await
        })?
        .map_err(|e| GmtuiError::Auth(format!("refresh token failed: {e}")))?;

        let expires_at = token_result
            .expires_in()
            .map(|d| Utc::now() + Duration::from_std(d).unwrap_or(Duration::hours(1)))
            .unwrap_or_else(|| Utc::now() + Duration::hours(1));

        let refresh_token = token_result
            .refresh_token()
            .map(|r| r.secret().to_string())
            .unwrap_or_else(|| {
                self.token
                    .as_ref()
                    .map(|t| t.refresh_token.clone())
                    .unwrap_or_default()
            });

        let new_token = TokenStore {
            access_token: token_result.access_token().secret().to_string(),
            refresh_token,
            expires_at,
        };

        new_token.save(&self.token_path)?;
        self.token = Some(new_token);
        Ok(())
    }

    fn exchange_code(&mut self, code: String, verifier: PkceCodeVerifier) -> Result<()> {
        let token_result = run_oauth_request(|| async {
            self.client
                .exchange_code(AuthorizationCode::new(code))
                .set_pkce_verifier(verifier)
                .request_async(async_http_client)
                .await
        })?
        .map_err(|e| GmtuiError::Auth(format!("token exchange failed: {e}")))?;

        let refresh_token = token_result
            .refresh_token()
            .map(|r| r.secret().to_string())
            .unwrap_or_else(|| {
                self.token
                    .as_ref()
                    .map(|t| t.refresh_token.clone())
                    .unwrap_or_default()
            });

        let expires_at = token_result
            .expires_in()
            .map(|d| Utc::now() + Duration::from_std(d).unwrap_or(Duration::hours(1)))
            .unwrap_or_else(|| Utc::now() + Duration::hours(1));

        let token = TokenStore {
            access_token: token_result.access_token().secret().to_string(),
            refresh_token,
            expires_at,
        };

        token.save(&self.token_path)?;
        self.token = Some(token);
        Ok(())
    }
}

fn wait_for_auth_code() -> Result<(String, String)> {
    let listener = TcpListener::bind("127.0.0.1:8080")
        .map_err(|e| GmtuiError::Auth(format!("failed to bind listener: {e}")))?;

    let (mut stream, _) = listener
        .accept()
        .map_err(|e| GmtuiError::Auth(format!("failed to accept auth redirect: {e}")))?;

    let mut buffer = [0u8; 4096];
    let n = stream
        .read(&mut buffer)
        .map_err(|e| GmtuiError::Auth(format!("failed to read auth redirect: {e}")))?;

    let request = String::from_utf8_lossy(&buffer[..n]);
    let first_line = request.lines().next().unwrap_or("");
    let (code, state) = parse_code_and_state(first_line)?;

    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\nYou can close this window and return to gmtui.";
    let _ = stream.write_all(response.as_bytes());

    Ok((code, state))
}

fn run_oauth_request<F, Fut, T>(f: F) -> Result<T>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    if let Ok(handle) = Handle::try_current() {
        tokio::task::block_in_place(|| Ok(handle.block_on(f())))
    } else {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| GmtuiError::Auth(e.to_string()))?;
        Ok(runtime.block_on(f()))
    }
}

fn parse_code_and_state(line: &str) -> Result<(String, String)> {
    let start = line
        .find('/')
        .ok_or_else(|| GmtuiError::Auth("invalid redirect".to_string()))?;
    let end = line
        .find(" HTTP")
        .ok_or_else(|| GmtuiError::Auth("invalid redirect".to_string()))?;
    let path = &line[start..end];

    let query_start = path
        .find('?')
        .ok_or_else(|| GmtuiError::Auth("missing query string".to_string()))?;

    let query = &path[query_start + 1..];

    let params: std::collections::HashMap<_, _> = url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect();

    if let Some(err) = params.get("error") {
        return Err(GmtuiError::Auth(format!("oauth error: {err}")));
    }

    let code = params
        .get("code")
        .ok_or_else(|| GmtuiError::Auth("missing auth code".to_string()))?
        .to_string();

    let state = params
        .get("state")
        .ok_or_else(|| GmtuiError::Auth("missing state".to_string()))?
        .to_string();

    Ok((code, state))
}
