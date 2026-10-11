use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use reqwest::{header, Client, StatusCode};
use serde::Serialize;
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{Mutex, Notify};

const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const EVENT: &str = "codex-usage-updated";

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UsageStatus {
    Connecting,
    Ready,
    MissingSession,
<<<<<<< HEAD
    AppNotRunning,
=======
>>>>>>> b2c37d6f8050781195e3aa58c50efdd5b8c894fc
    InvalidSession,
    UnsupportedAuth,
    ExpiredSession,
    AccessDenied,
    RateLimited,
    NetworkError,
    InvalidResponse,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub account_id: String,
    pub user_id: Option<String>,
    pub email: Option<String>,
    pub plan: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    pub id: String,
    pub remaining_percent: Option<f64>,
    pub window_seconds: Option<u64>,
    pub resets_at: Option<u64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub revision: u64,
    pub status: UsageStatus,
    pub refreshing: bool,
    pub message: String,
    pub source_path: String,
    pub account: Option<Account>,
    pub windows: Vec<UsageWindow>,
    pub updated_at: Option<u64>,
    pub refresh_interval_seconds: u64,
    pub next_refresh_at_ms: Option<u64>,
}

struct Credentials {
    access_token: String,
    identity: String,
    account: Account,
}

struct UsageError {
    status: UsageStatus,
    message: &'static str,
    retry_after: Option<Duration>,
}

impl UsageError {
    fn new(status: UsageStatus, message: &'static str) -> Self {
        Self {
            status,
            message,
            retry_after: None,
        }
    }
}

struct Inner {
    snapshot: UsageSnapshot,
    identity: Option<String>,
    last_attempt: Option<Instant>,
    retry_delay: Duration,
    rate_limited_until: Option<Instant>,
    failures: u32,
}

struct UsageService {
    client: Client,
    auth_path: PathBuf,
    inner: Mutex<Inner>,
    request_gate: Mutex<()>,
    changed: Notify,
}

pub struct UsageState(Arc<UsageService>);

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn auth_path() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(
                std::env::var_os("USERPROFILE")
                    .or_else(|| std::env::var_os("HOME"))
                    .unwrap_or_default(),
            )
            .join(".codex")
        })
        .join("auth.json")
}

fn jwt_claims(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok()?).ok()
}

fn text(value: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_owned())
}

fn parse_credentials(value: &Value) -> Result<Credentials, UsageError> {
    let invalid = || {
        UsageError::new(UsageStatus::InvalidSession, "The Codex session file is incomplete. Sign in again in Codex, then check the connection.")
    };
    if text(&value["auth_mode"]).is_some_and(|mode| mode != "chatgpt")
        || (value["tokens"].is_null() && text(&value["OPENAI_API_KEY"]).is_some())
    {
        return Err(UsageError::new(UsageStatus::UnsupportedAuth, "This session uses an API key. Sign in to Codex with ChatGPT to read subscription limits."));
    }
    let access_token = text(&value["tokens"]["access_token"]).ok_or_else(invalid)?;
    let id_claims = text(&value["tokens"]["id_token"])
        .and_then(|token| jwt_claims(&token))
        .unwrap_or(Value::Null);
    let access_claims = jwt_claims(&access_token).unwrap_or(Value::Null);
    let claims = if id_claims.is_object() {
        &id_claims
    } else {
        &access_claims
    };
    let auth = &claims["https://api.openai.com/auth"];
    let account_id = text(&value["tokens"]["account_id"])
        .or_else(|| text(&auth["chatgpt_account_id"]))
        .ok_or_else(invalid)?;
    let user = text(&auth["chatgpt_user_id"])
        .or_else(|| text(&claims["sub"]));
    Ok(Credentials {
        access_token,
        identity: format!("{account_id}:{}", user.as_deref().unwrap_or_default()),
        account: Account {
            account_id,
            user_id: user,
            email: text(&claims["email"]),
            plan: text(&auth["chatgpt_plan_type"]),
        },
    })
}

fn read_credentials(path: &std::path::Path) -> Result<Credentials, UsageError> {
    let data = std::fs::read(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            UsageError::new(UsageStatus::MissingSession, "No Codex session file found. Sign in in Codex. Sessions stored only in Windows Credential Manager are not supported yet.")
        } else {
            UsageError::new(UsageStatus::InvalidSession, "Could not read the Codex session file. Check file access, then try again.")
        }
    })?;
    let value = serde_json::from_slice(&data).map_err(|_| UsageError::new(UsageStatus::InvalidSession, "The Codex session file is not valid JSON. Wait for Codex to update it, then try again."))?;
    parse_credentials(&value)
}

fn parse_usage(value: &Value, received_at: u64) -> Result<Vec<UsageWindow>, UsageError> {
    let mut windows = Vec::new();
    for (id, key) in [
        ("primary", "primary_window"),
        ("secondary", "secondary_window"),
    ] {
        let window = &value["rate_limit"][key];
        if !window.is_object() || window.as_object().is_some_and(|object| object.is_empty()) {
            continue;
        }
        let remaining_percent = window["used_percent"]
            .as_f64()
            .filter(|used| used.is_finite())
            .map(|used| (100.0 - used).clamp(0.0, 100.0));
        let resets_at = window["reset_at"]
            .as_u64()
            .filter(|time| *time > 0)
            .or_else(|| {
                window["reset_after_seconds"]
                    .as_u64()
                    .and_then(|seconds| received_at.checked_add(seconds))
            })
            .filter(|time| *time <= 253_402_300_799);
        windows.push(UsageWindow {
            id: id.to_owned(),
            remaining_percent,
            window_seconds: window["limit_window_seconds"]
                .as_u64()
                .filter(|seconds| *seconds > 0),
            resets_at,
        });
    }
    if windows.is_empty() {
        return Err(UsageError::new(
            UsageStatus::InvalidResponse,
            "Codex returned no supported quota windows. Try again later.",
        ));
    }
    Ok(windows)
}

fn retry_after(value: Option<&header::HeaderValue>) -> Duration {
    let parsed = value
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value
                .parse::<u64>()
                .ok()
                .map(Duration::from_secs)
                .or_else(|| {
                    httpdate::parse_http_date(value)
                        .ok()
                        .map(|time| time.duration_since(SystemTime::now()).unwrap_or_default())
                })
        });
    parsed
        .unwrap_or(Duration::from_secs(60))
        .max(Duration::from_secs(1))
}

fn http_error(status: StatusCode, retry: Option<&header::HeaderValue>) -> UsageError {
    match status {
        StatusCode::UNAUTHORIZED => UsageError::new(UsageStatus::ExpiredSession, "Your Codex session has expired. Open Codex to update the session or sign in again, then check the connection."),
        StatusCode::FORBIDDEN => UsageError::new(UsageStatus::AccessDenied, "Codex denied access to this account or workspace. Check the account selected in Codex."),
        StatusCode::TOO_MANY_REQUESTS => UsageError { status: UsageStatus::RateLimited, message: "Codex asked us to slow down. UsagePin will retry automatically after the wait period.", retry_after: Some(retry_after(retry)) },
        _ => UsageError::new(UsageStatus::NetworkError, "The Codex usage service is unavailable. UsagePin will retry automatically."),
    }
}

fn next_refresh_delay(inner: &Inner) -> Duration {
    let interval =
        Duration::from_secs(inner.snapshot.refresh_interval_seconds).max(inner.retry_delay);
    let mut delay = inner
        .last_attempt
        .map(|last| interval.saturating_sub(last.elapsed()))
        .unwrap_or_default();
    if let Some(until) = inner.rate_limited_until {
        delay = delay.max(until.saturating_duration_since(Instant::now()));
    }
    // Refresh just after an approaching reset, without treating a past reset as a new window.
    if inner.snapshot.status == UsageStatus::Ready {
        let now = now_seconds();
        if let Some(reset) = inner
            .snapshot
            .windows
            .iter()
            .filter_map(|window| window.resets_at)
            .filter(|reset| *reset > now)
            .min()
        {
            delay = delay.min(Duration::from_secs(reset - now + 1));
        }
    }
    delay
}

fn snapshot_with_refresh_schedule(inner: &Inner) -> UsageSnapshot {
    let mut snapshot = inner.snapshot.clone();
    snapshot.next_refresh_at_ms = if snapshot.refreshing || inner.last_attempt.is_none() {
        None
    } else {
        SystemTime::now()
            .checked_add(next_refresh_delay(inner))
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .and_then(|duration| u64::try_from(duration.as_millis()).ok())
    };
    snapshot
}

impl UsageService {
    fn new(client: Client, auth_path: PathBuf) -> Self {
        Self {
            client,
            inner: Mutex::new(Inner {
                snapshot: UsageSnapshot {
                    revision: 0,
                    status: UsageStatus::Connecting,
                    refreshing: false,
                    message: "Checking your existing Codex Windows session…".to_owned(),
                    source_path: auth_path.display().to_string(),
                    account: None,
                    windows: Vec::new(),
                    updated_at: None,
                    refresh_interval_seconds: 60,
                    next_refresh_at_ms: None,
                },
                identity: None,
                last_attempt: None,
                retry_delay: Duration::ZERO,
                rate_limited_until: None,
                failures: 0,
            }),
            auth_path,
            request_gate: Mutex::new(()),
            changed: Notify::new(),
        }
    }

    async fn snapshot(&self) -> UsageSnapshot {
        let inner = self.inner.lock().await;
        snapshot_with_refresh_schedule(&inner)
    }

    async fn bind_account(&self, credentials: &Credentials) {
        let mut inner = self.inner.lock().await;
        if inner.identity.as_ref() != Some(&credentials.identity) {
            inner.snapshot.windows.clear();
            inner.snapshot.updated_at = None;
            inner.identity = Some(credentials.identity.clone());
        }
        let mut account = credentials.account.clone();
        if account.plan.is_none() {
            account.plan = inner
                .snapshot
                .account
                .as_ref()
                .filter(|account| account.account_id == credentials.account.account_id)
                .and_then(|account| account.plan.clone());
        }
        inner.snapshot.account = Some(account);
    }

    async fn fetch(
        &self,
        url: &str,
        mut credentials: Credentials,
    ) -> Result<(Credentials, Value), UsageError> {
        for attempt in 0..2 {
            let response = self.client.get(url)
                .bearer_auth(&credentials.access_token)
                .header(header::ACCEPT, "application/json")
                .header("chatgpt-account-id", &credentials.account.account_id)
                .header("originator", "codex_cli_rs")
                .send().await
                .map_err(|_| UsageError::new(UsageStatus::NetworkError, "Could not reach Codex. Check your internet connection; UsagePin will retry automatically."))?;
            if response.status() == StatusCode::UNAUTHORIZED && attempt == 0 {
                if let Ok(new_credentials) = read_credentials(&self.auth_path) {
                    if new_credentials.access_token != credentials.access_token {
                        credentials = new_credentials;
                        self.bind_account(&credentials).await;
                        continue;
                    }
                }
            }
            if !response.status().is_success() {
                return Err(http_error(
                    response.status(),
                    response.headers().get(header::RETRY_AFTER),
                ));
            }
            let payload = response.json::<Value>().await.map_err(|_| {
                UsageError::new(
                    UsageStatus::InvalidResponse,
                    "Codex returned an unreadable usage response. Try again later.",
                )
            })?;
            return Ok((credentials, payload));
        }
        unreachable!()
    }

    async fn refresh(&self, app: &AppHandle) -> UsageSnapshot {
        self.refresh_with(USAGE_URL, |snapshot| {
            let _ = app.emit(EVENT, snapshot);
        })
        .await
    }

    async fn refresh_with(&self, url: &str, publish: impl Fn(UsageSnapshot)) -> UsageSnapshot {
        let Ok(_gate) = self.request_gate.try_lock() else {
            return self.snapshot().await;
        };
        {
            let mut inner = self.inner.lock().await;
            if inner
                .rate_limited_until
                .is_some_and(|until| until > Instant::now())
            {
                return snapshot_with_refresh_schedule(&inner);
            }
            inner.snapshot.refreshing = true;
            inner.snapshot.revision += 1;
            inner.last_attempt = Some(Instant::now());
        }
        publish(self.snapshot().await);
        let result = match read_credentials(&self.auth_path) {
            Ok(credentials) => {
                self.bind_account(&credentials).await;
                self.fetch(url, credentials)
                    .await
                    .and_then(|(credentials, payload)| {
                        parse_usage(&payload, now_seconds())
                            .map(|windows| (credentials, payload, windows))
                    })
            }
            Err(error) => {
                let mut inner = self.inner.lock().await;
                inner.identity = None;
                inner.snapshot.account = None;
                inner.snapshot.windows.clear();
                inner.snapshot.updated_at = None;
                Err(error)
            }
        };
        let snapshot = {
            let mut inner = self.inner.lock().await;
            inner.snapshot.refreshing = false;
            inner.snapshot.revision += 1;
            // Schedule from completion, so a slow request cannot trigger a tight polling loop.
            inner.last_attempt = Some(Instant::now());
            match result {
                Ok((credentials, payload, windows)) => {
                    let mut account = credentials.account;
                    account.plan = text(&payload["plan_type"]).or(account.plan);
                    inner.snapshot.account = Some(account);
                    inner.snapshot.windows = windows;
                    inner.snapshot.updated_at = Some(now_seconds());
                    inner.snapshot.status = UsageStatus::Ready;
                    inner.snapshot.message =
                        "Connected using your existing Codex Windows session.".to_owned();
                    inner.failures = 0;
                    inner.retry_delay = Duration::ZERO;
                    inner.rate_limited_until = None;
                }
                Err(error) => {
                    inner.failures = inner.failures.saturating_add(1);
                    inner.retry_delay = error.retry_after.unwrap_or_else(|| {
                        Duration::from_secs((15u64 << inner.failures.min(5)).min(300))
                    });
                    inner.rate_limited_until = error
                        .retry_after
                        .and_then(|delay| Instant::now().checked_add(delay));
                    inner.snapshot.status = error.status;
                    inner.snapshot.message = error.message.to_owned();
                }
            }
            snapshot_with_refresh_schedule(&inner)
        };
        publish(snapshot.clone());
        self.changed.notify_one();
        snapshot
    }

    async fn delay(&self) -> Duration {
        // Wait for any in-flight refresh before calculating its next due time.
        let _gate = self.request_gate.lock().await;
        let inner = self.inner.lock().await;
        next_refresh_delay(&inner)
    }
}

pub fn start(app: &AppHandle) -> Result<(), reqwest::Error> {
    let client = Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .user_agent(concat!("UsagePin/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let service = Arc::new(UsageService::new(client, auth_path()));
    app.manage(UsageState(service.clone()));
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let delay = service.delay().await;
            if !delay.is_zero()
                && tokio::time::timeout(delay, service.changed.notified())
                    .await
                    .is_ok()
            {
                continue;
            }
            service.refresh(&app).await;
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn get_codex_usage(state: State<'_, UsageState>) -> Result<UsageSnapshot, String> {
    Ok(state.0.snapshot().await)
}

#[tauri::command]
pub async fn refresh_codex_usage(
    app: AppHandle,
    state: State<'_, UsageState>,
) -> Result<UsageSnapshot, String> {
    Ok(state.0.refresh(&app).await)
}

#[tauri::command]
pub async fn set_usage_refresh_interval(
    app: AppHandle,
    state: State<'_, UsageState>,
<<<<<<< HEAD
    antigravity_state: State<'_, crate::antigravity_usage::UsageState>,
=======
>>>>>>> b2c37d6f8050781195e3aa58c50efdd5b8c894fc
    seconds: u64,
) -> Result<(), String> {
    if seconds == 0 || seconds > 9_007_199_254_740_991 {
        return Err("Use a positive whole number of seconds.".to_owned());
    }
<<<<<<< HEAD
    crate::antigravity_usage::set_refresh_interval(&app, &antigravity_state, seconds).await;
=======
>>>>>>> b2c37d6f8050781195e3aa58c50efdd5b8c894fc
    let snapshot = {
        let mut inner = state.0.inner.lock().await;
        if inner.snapshot.refresh_interval_seconds == seconds {
            return Ok(());
        }
        inner.snapshot.refresh_interval_seconds = seconds;
        inner.snapshot.revision += 1;
        snapshot_with_refresh_schedule(&inner)
    };
    let _ = app.emit(EVENT, snapshot);
    state.0.changed.notify_one();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn token(claims: Value) -> String {
        format!(
            "header.{}.signature",
            URL_SAFE_NO_PAD.encode(claims.to_string())
        )
    }

    struct TestAuth(PathBuf);

    impl TestAuth {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "usagepin-auth-{}-{}.json",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::write(&path, json!({"tokens":{"access_token":"first-test-token", "account_id":"test-workspace"}}).to_string()).unwrap();
            Self(path)
        }
    }

    impl Drop for TestAuth {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn mock_server(
        auth: Option<PathBuf>,
        responses: Vec<(u16, String)>,
    ) -> (String, std::thread::JoinHandle<Vec<String>>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/usage", listener.local_addr().unwrap());
        let thread = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for (index, (status, body)) in responses.into_iter().enumerate() {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                while !request.windows(4).any(|value| value == b"\r\n\r\n") {
                    let mut bytes = [0u8; 1024];
                    let count = stream.read(&mut bytes).unwrap();
                    if count == 0 {
                        break;
                    }
                    request.extend_from_slice(&bytes[..count]);
                }
                requests.push(String::from_utf8(request).unwrap().to_lowercase());
                if index == 0 {
                    if let Some(path) = &auth {
                        std::fs::write(path, json!({"tokens":{"access_token":"second-test-token","account_id":"test-workspace"}}).to_string()).unwrap();
                    }
                }
                std::thread::sleep(Duration::from_millis(30));
                write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
            requests
        });
        (url, thread)
    }

    #[test]
    fn unauthorized_retries_only_if_codex_has_written_a_new_token() {
        tauri::async_runtime::block_on(async {
            for rotate in [false, true] {
                let auth = TestAuth::new();
                let mut responses = vec![(401, "{}".to_owned())];
                if rotate {
                    responses.push((
                        200,
                        json!({"rate_limit":{"primary_window":{"used_percent":25}}}).to_string(),
                    ));
                }
                let (url, server) = mock_server(rotate.then(|| auth.0.clone()), responses);
                let service = UsageService::new(
                    Client::builder().no_proxy().build().unwrap(),
                    auth.0.clone(),
                );
                let result = service.refresh_with(&url, |_| {}).await;
                let requests = server.join().unwrap();
                if rotate {
                    assert_eq!(result.status, UsageStatus::Ready);
                    assert_eq!(result.windows[0].remaining_percent, Some(75.0));
                    assert_eq!(requests.len(), 2);
                    assert!(requests[1].contains("authorization: bearer second-test-token"));
                } else {
                    assert_eq!(result.status, UsageStatus::ExpiredSession);
                    assert_eq!(requests.len(), 1);
                }
                assert!(requests[0].contains("chatgpt-account-id: test-workspace"));
            }
        });
    }

    #[test]
    fn overlapping_manual_and_scheduled_refresh_share_one_request() {
        tauri::async_runtime::block_on(async {
            let auth = TestAuth::new();
            let (url, server) = mock_server(
                None,
                vec![(
                    200,
                    json!({"rate_limit":{"primary_window":{"used_percent":35}}}).to_string(),
                )],
            );
            let service = Arc::new(UsageService::new(
                Client::builder().no_proxy().build().unwrap(),
                auth.0.clone(),
            ));
            let running_service = service.clone();
            let running_url = url.clone();
            let first = tauri::async_runtime::spawn(async move {
                running_service.refresh_with(&running_url, |_| {}).await
            });
            tokio::time::timeout(Duration::from_secs(2), async {
                while !service.snapshot().await.refreshing {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            })
            .await
            .unwrap();
            assert!(service.refresh_with(&url, |_| {}).await.refreshing);
            assert_eq!(
                first.await.unwrap().windows[0].remaining_percent,
                Some(65.0)
            );
            assert_eq!(server.join().unwrap().len(), 1);
        });
    }

    #[test]
    fn lost_session_clears_account_and_last_known_values() {
        tauri::async_runtime::block_on(async {
            let auth = TestAuth::new();
            let service = UsageService::new(Client::new(), auth.0.clone());
            service
                .bind_account(
                    &read_credentials(&auth.0).unwrap_or_else(|_| panic!("auth rejected")),
                )
                .await;
            std::fs::remove_file(&auth.0).unwrap();
            let snapshot = service.refresh_with("http://unused.invalid", |_| {}).await;
            assert_eq!(snapshot.status, UsageStatus::MissingSession);
            assert!(snapshot.account.is_none());
            assert!(snapshot.windows.is_empty());
        });
    }

    #[test]
    fn existing_windows_auth_accepts_missing_auth_mode_and_no_refresh_token() {
        let credentials = parse_credentials(&json!({"tokens": {"access_token": "access", "account_id": "selected", "id_token": token(json!({"email":"person@example.test", "https://api.openai.com/auth":{"chatgpt_account_id":"other", "chatgpt_user_id":"user"}}))}})).unwrap_or_else(|_| panic!("valid auth rejected"));
        assert_eq!(credentials.account.account_id, "selected");
        assert_eq!(credentials.identity, "selected:user");
        assert_eq!(credentials.account.user_id.as_deref(), Some("user"));
        assert_eq!(serde_json::to_value(&credentials.account).unwrap()["userId"], "user");
        assert_eq!(
            credentials.account.email.as_deref(),
            Some("person@example.test")
        );
    }

    #[test]
    fn api_key_and_incomplete_sessions_are_distinct() {
        assert_eq!(
            parse_credentials(&json!({"OPENAI_API_KEY":"key"}))
                .err()
                .unwrap()
                .status,
            UsageStatus::UnsupportedAuth
        );
        assert_eq!(
            parse_credentials(&json!({"tokens":{"access_token":"access"}}))
                .err()
                .unwrap()
                .status,
            UsageStatus::InvalidSession
        );
    }

    #[test]
    fn quota_converts_percent_preserves_real_duration_and_missing_window() {
        let windows = parse_usage(&json!({"rate_limit":{"primary_window":{"used_percent":25,"limit_window_seconds":604800,"reset_at":2000},"secondary_window":null}}), 1000).unwrap_or_else(|_| panic!("quota rejected"));
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].remaining_percent, Some(75.0));
        assert_eq!(windows[0].window_seconds, Some(604800));
        assert_eq!(windows[0].resets_at, Some(2000));
    }

    #[test]
    fn missing_percent_is_unknown_and_reset_fallback_includes_zero() {
        let windows = parse_usage(&json!({"rate_limit":{"primary_window":{"used_percent":null,"reset_after_seconds":0},"secondary_window":{"used_percent":120,"reset_after_seconds":60}}}), 1000).unwrap_or_else(|_| panic!("quota rejected"));
        assert_eq!(windows[0].remaining_percent, None);
        assert_eq!(windows[0].resets_at, Some(1000));
        assert_eq!(windows[1].remaining_percent, Some(0.0));
        assert_eq!(windows[1].resets_at, Some(1060));
        assert_eq!(
            parse_usage(&json!({"error":"changed schema"}), 1000)
                .err()
                .unwrap()
                .status,
            UsageStatus::InvalidResponse
        );
    }

    #[test]
    fn errors_distinguish_expiry_permissions_and_retry_after() {
        assert_eq!(
            http_error(StatusCode::UNAUTHORIZED, None).status,
            UsageStatus::ExpiredSession
        );
        assert_eq!(
            http_error(StatusCode::FORBIDDEN, None).status,
            UsageStatus::AccessDenied
        );
        assert_eq!(
            http_error(
                StatusCode::TOO_MANY_REQUESTS,
                Some(&header::HeaderValue::from_static("120"))
            )
            .retry_after,
            Some(Duration::from_secs(120))
        );
    }

    #[test]
    fn changing_user_in_same_workspace_clears_previous_quota() {
        tauri::async_runtime::block_on(async {
            let service = UsageService::new(Client::new(), PathBuf::from("unused"));
            let first = parse_credentials(
                &json!({"tokens":{"access_token":token(json!({"sub":"one"})),"account_id":"team"}}),
            )
            .unwrap_or_else(|_| panic!("auth rejected"));
            service.bind_account(&first).await;
            assert_eq!(service.snapshot().await.account.unwrap().user_id.as_deref(), Some("one"));
            {
                let mut inner = service.inner.lock().await;
                inner.snapshot.updated_at = Some(1000);
                inner.snapshot.windows = parse_usage(
                    &json!({"rate_limit":{"primary_window":{"used_percent":25}}}),
                    1000,
                )
                .unwrap_or_default();
            }
            let second = parse_credentials(
                &json!({"tokens":{"access_token":token(json!({"sub":"two"})),"account_id":"team"}}),
            )
            .unwrap_or_else(|_| panic!("auth rejected"));
            service.bind_account(&second).await;
            assert_eq!(service.snapshot().await.account.unwrap().user_id.as_deref(), Some("two"));
            assert!(service.snapshot().await.windows.is_empty());
            assert_eq!(service.snapshot().await.updated_at, None);
        });
    }

    #[test]
    fn user_identity_does_not_depend_on_email_or_token_rotation() {
        tauri::async_runtime::block_on(async {
            let service = UsageService::new(Client::new(), PathBuf::from("unused"));
            let credentials = |marker: &str| {
                parse_credentials(&json!({"tokens": {
                    "access_token": token(json!({"sub":"user", "marker":marker})),
                    "account_id":"team"
                }})).unwrap_or_else(|_| panic!("auth rejected"))
            };
            let first = credentials("first");
            assert_eq!(first.account.user_id.as_deref(), Some("user"));
            assert!(first.account.email.is_none());
            service.bind_account(&first).await;
            {
                let mut inner = service.inner.lock().await;
                inner.snapshot.updated_at = Some(1000);
                inner.snapshot.windows = parse_usage(
                    &json!({"rate_limit":{"primary_window":{"used_percent":25}}}), 1000,
                ).unwrap_or_default();
            }
            let rotated = credentials("rotated");
            assert_ne!(first.access_token, rotated.access_token);
            service.bind_account(&rotated).await;
            let snapshot = service.snapshot().await;
            assert_eq!(snapshot.updated_at, Some(1000));
            assert_eq!(snapshot.windows[0].remaining_percent, Some(75.0));

            let unknown = parse_credentials(&json!({"tokens": {
                "access_token":"opaque-token", "account_id":"team"
            }})).unwrap_or_else(|_| panic!("auth rejected"));
            assert!(unknown.account.user_id.is_none());
        });
    }

    #[test]
    fn scheduler_waits_for_inflight_refresh_before_computing_delay() {
        tauri::async_runtime::block_on(async {
            let service = UsageService::new(Client::new(), PathBuf::from("unused"));
            let gate = service.request_gate.lock().await;
            {
                let mut inner = service.inner.lock().await;
                inner.snapshot.refreshing = true;
                inner.snapshot.refresh_interval_seconds = 1;
                inner.last_attempt = Some(Instant::now() - Duration::from_secs(2));
            }

            let mut delay = Box::pin(service.delay());
            assert!(tokio::time::timeout(Duration::from_millis(30), &mut delay)
                .await
                .is_err());
            // An overlapping refresh still returns the current snapshot without a request.
            assert!(service.refresh_with("http://unused.invalid", |_| {}).await.refreshing);

            {
                let mut inner = service.inner.lock().await;
                inner.snapshot.refreshing = false;
                inner.last_attempt = Some(Instant::now());
            }
            drop(gate);
            let next_delay = tokio::time::timeout(Duration::from_secs(2), delay)
                .await
                .expect("scheduler did not resume after refresh completion");
            assert!(next_delay > Duration::ZERO);
            assert!(next_delay <= Duration::from_secs(1));
        });
    }

    #[test]
    fn scheduler_uses_completion_interval_and_error_backoff() {
        tauri::async_runtime::block_on(async {
            let service = UsageService::new(Client::new(), PathBuf::from("unused"));
            assert!(service.delay().await.is_zero());
            {
                let mut inner = service.inner.lock().await;
                inner.last_attempt = Some(Instant::now());
                inner.snapshot.refresh_interval_seconds = 1;
                inner.retry_delay = Duration::from_secs(120);
            }
            assert!(service.delay().await > Duration::from_secs(119));
        });
    }

    #[test]
    fn refresh_countdown_tracks_interval_changes_backoff_and_quota_reset() {
        tauri::async_runtime::block_on(async {
            let service = UsageService::new(Client::new(), PathBuf::from("unused"));
            let mut inner = service.inner.lock().await;
            inner.last_attempt = Some(Instant::now() - Duration::from_secs(10));

            let remaining_ms = |inner: &Inner| {
                let snapshot = snapshot_with_refresh_schedule(inner);
                let now_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
                snapshot.next_refresh_at_ms.unwrap().saturating_sub(now_ms)
            };
            assert!((49_000..=50_000).contains(&remaining_ms(&inner)));
            inner.snapshot.refresh_interval_seconds = 30;
            assert!((19_000..=20_000).contains(&remaining_ms(&inner)));
            inner.retry_delay = Duration::from_secs(120);
            assert!((109_000..=110_000).contains(&remaining_ms(&inner)));
            inner.rate_limited_until = Some(Instant::now() + Duration::from_secs(200));
            assert!((199_000..=200_000).contains(&remaining_ms(&inner)));

            inner.rate_limited_until = None;
            inner.retry_delay = Duration::ZERO;
            inner.snapshot.status = UsageStatus::Ready;
            inner.snapshot.windows = vec![UsageWindow {
                id: "primary".to_owned(), remaining_percent: Some(80.0),
                window_seconds: Some(18000), resets_at: Some(now_seconds() + 5),
            }];
            assert!((4_000..=6_000).contains(&remaining_ms(&inner)));
            let snapshot = snapshot_with_refresh_schedule(&inner);
            drop(inner);
            let delay = service.delay().await;
            let now_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
            let displayed_delay = Duration::from_millis(snapshot.next_refresh_at_ms.unwrap().saturating_sub(now_ms));
            assert!(delay.abs_diff(displayed_delay) < Duration::from_millis(1100));
            assert!(serde_json::to_value(snapshot).unwrap()["nextRefreshAtMs"].is_u64());
        });
    }

    #[test]
    fn refresh_countdown_has_no_deadline_before_start_or_during_request() {
        tauri::async_runtime::block_on(async {
            let service = UsageService::new(Client::new(), PathBuf::from("unused"));
            assert!(service.snapshot().await.next_refresh_at_ms.is_none());
            {
                let mut inner = service.inner.lock().await;
                inner.last_attempt = Some(Instant::now());
                inner.snapshot.refreshing = true;
            }
            assert!(service.snapshot().await.next_refresh_at_ms.is_none());
        });
    }

    #[test]
    #[ignore = "reads the local Codex session and makes one authenticated usage request"]
    fn live_windows_session() {
        tauri::async_runtime::block_on(async {
            let client = Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(10))
                .user_agent("UsagePin/0.1.0")
                .build()
                .unwrap();
            let service = UsageService::new(client, auth_path());
            let credentials = read_credentials(&service.auth_path)
                .unwrap_or_else(|error| panic!("{}", error.message));
            let (_, payload) = service
                .fetch(USAGE_URL, credentials)
                .await
                .unwrap_or_else(|error| panic!("{}", error.message));
            let windows = parse_usage(&payload, now_seconds())
                .unwrap_or_else(|error| panic!("{}", error.message));
            for window in windows {
                println!(
                    "{}: remaining={:?}, window_seconds={:?}, resets_at={:?}",
                    window.id, window.remaining_percent, window.window_seconds, window.resets_at
                );
            }
        });
    }
}
