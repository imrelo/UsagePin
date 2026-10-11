use crate::codex_usage::{Account, UsageStatus};
use reqwest::{Client, StatusCode};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, State};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tokio::sync::{Mutex, Notify};

const EVENT: &str = "antigravity-usage-updated";
const SERVICE: &str = "/exa.language_server_pb.LanguageServerService";
const MAX_RESPONSE_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    id: String,
    label: String,
    remaining_percent: Option<f64>,
    window_seconds: Option<u64>,
    resets_at: Option<u64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    revision: u64,
    status: UsageStatus,
    refreshing: bool,
    message: String,
    source_path: String,
    account: Option<Account>,
    windows: Vec<UsageWindow>,
    updated_at: Option<u64>,
    refresh_interval_seconds: u64,
    next_refresh_at_ms: Option<u64>,
}

#[derive(Debug)]
struct UsageError(UsageStatus, &'static str);

struct Inner {
    snapshot: UsageSnapshot,
    last_completed: Option<Instant>,
    failures: u32,
}

struct UsageService {
    client: Client,
    log_path: PathBuf,
    inner: Mutex<Inner>,
    request_gate: Mutex<()>,
    changed: Notify,
}

pub struct UsageState(Arc<UsageService>);

fn now_seconds() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn log_path() -> PathBuf {
    std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(std::env::var_os("USERPROFILE").unwrap_or_default()).join("AppData/Roaming")
    }).join("Antigravity/logs/language_server.log")
}

fn app_not_running() -> UsageError {
    UsageError(UsageStatus::AppNotRunning, "Open Antigravity and sign in to read its model quotas. UsagePin will reconnect automatically.")
}

fn parse_port(log: &str) -> Option<u16> {
    log.lines().rev().find_map(|line| {
        let (_, after) = line.split_once(" port at ")?;
        let (port, protocol) = after.split_once(" for ")?;
        if !line.contains("listening on ") || protocol.split_whitespace().next()? != "HTTP" {
            return None;
        }
        port.parse::<u16>().ok().filter(|port| *port > 0)
    })
}

fn discover_port(path: &std::path::Path) -> Result<u16, UsageError> {
    let mut file = File::open(path).map_err(|_| app_not_running())?;
    let length = file.metadata().map_err(|_| app_not_running())?.len();
    file.seek(SeekFrom::Start(length.saturating_sub(128 * 1024))).map_err(|_| app_not_running())?;
    let mut bytes = Vec::new();
    file.take(128 * 1024).read_to_end(&mut bytes).map_err(|_| app_not_running())?;
    parse_port(&String::from_utf8_lossy(&bytes)).ok_or_else(app_not_running)
}

fn parse_csrf(html: &str) -> Result<String, UsageError> {
    let config = html.split_once("__APP_CONFIG__").and_then(|(_, rest)| rest.split_once('='))
        .and_then(|(_, rest)| rest.split_once("</script>"))
        .map(|(config, _)| config.trim().trim_end_matches(';').trim());
    let value: Value = config.and_then(|config| serde_json::from_str(config).ok()).ok_or_else(|| {
        UsageError(UsageStatus::InvalidResponse, "Antigravity's local connection format has changed. Update UsagePin to reconnect.")
    })?;
    value["csrfToken"].as_str().filter(|token| !token.is_empty() && token.len() <= 1024)
        .map(str::to_owned).ok_or(UsageError(UsageStatus::InvalidResponse, "Antigravity returned no local session token. Reopen Antigravity and try again."))
}

fn parse_usage(payload: &Value) -> Result<Vec<UsageWindow>, UsageError> {
    let groups = payload["response"]["groups"].as_array().ok_or(UsageError(
        UsageStatus::InvalidResponse, "Antigravity returned no model quota groups. Sign in in Antigravity and try again.",
    ))?;
    let mut windows = Vec::new();
    for group in groups {
        for bucket in group["buckets"].as_array().into_iter().flatten() {
            let Some(id) = bucket["bucketId"].as_str().filter(|id| !id.is_empty()) else { continue; };
            if windows.iter().any(|row: &UsageWindow| row.id == id) { continue; }
            let label = match id {
                "gemini-weekly" => "Gemini".to_owned(),
                "3p-weekly" => "Claude/GPT".to_owned(),
                _ => group["displayName"].as_str().unwrap_or("Quota").to_owned(),
            };
            let window_seconds = match bucket["window"].as_str() {
                Some("weekly") => Some(604800),
                Some("five_hour") => Some(18000),
                _ => None,
            };
            let resets_at = bucket["resetTime"].as_str()
                .and_then(|value| OffsetDateTime::parse(value, &Rfc3339).ok())
                .and_then(|date| u64::try_from(date.unix_timestamp()).ok())
                .filter(|time| *time > 0 && *time <= 253_402_300_799);
            windows.push(UsageWindow {
                id: id.to_owned(), label, window_seconds, resets_at,
                remaining_percent: bucket["remainingFraction"].as_f64()
                    .filter(|fraction| fraction.is_finite() && (0.0..=1.0).contains(fraction))
                    .map(|fraction| fraction * 100.0),
            });
        }
    }
    if windows.is_empty() {
        return Err(UsageError(UsageStatus::InvalidResponse, "Antigravity has no supported quota buckets. Try again later."));
    }
    Ok(windows)
}

fn parse_account(payload: &Value) -> Option<Account> {
    let status = &payload["userStatus"];
    let email = status["email"].as_str().filter(|email| !email.trim().is_empty())?;
    Some(Account {
        account_id: email.to_owned(), user_id: Some(email.to_owned()), email: Some(email.to_owned()),
        plan: status["userTier"]["name"].as_str().map(str::to_owned),
    })
}

fn delay(inner: &Inner) -> Duration {
    let backoff = if inner.failures == 0 { 0 } else { (15u64 << inner.failures.min(5)).min(300) };
    let interval = Duration::from_secs(inner.snapshot.refresh_interval_seconds.max(backoff));
    inner.last_completed.map(|completed| interval.saturating_sub(completed.elapsed())).unwrap_or_default()
}

fn snapshot(inner: &Inner) -> UsageSnapshot {
    let mut snapshot = inner.snapshot.clone();
    snapshot.next_refresh_at_ms = if snapshot.refreshing || inner.last_completed.is_none() { None } else {
        SystemTime::now().checked_add(delay(inner)).and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .and_then(|duration| u64::try_from(duration.as_millis()).ok())
    };
    snapshot
}

impl UsageService {
    fn new(client: Client, log_path: PathBuf) -> Self {
        Self {
            client,
            inner: Mutex::new(Inner {
                snapshot: UsageSnapshot {
                    revision: 0, status: UsageStatus::Connecting, refreshing: false,
                    message: "Checking the running Antigravity app…".to_owned(),
                    source_path: log_path.display().to_string(), account: None, windows: Vec::new(),
                    updated_at: None, refresh_interval_seconds: 60, next_refresh_at_ms: None,
                },
                last_completed: None, failures: 0,
            }),
            log_path, request_gate: Mutex::new(()), changed: Notify::new(),
        }
    }

    async fn read_response(&self, response: reqwest::Response) -> Result<String, UsageError> {
        match response.status() {
            status if status.is_success() => {},
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => return Err(UsageError(UsageStatus::AccessDenied, "Antigravity denied access to its local session. Reopen Antigravity and check the connection.")),
            StatusCode::TOO_MANY_REQUESTS => return Err(UsageError(UsageStatus::RateLimited, "Antigravity asked us to slow down. UsagePin will retry automatically.")),
            _ => return Err(UsageError(UsageStatus::InvalidResponse, "Antigravity's local quota service is unavailable. Try again later.")),
        }
        let mut response = response;
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| app_not_running())? {
            if bytes.len() + chunk.len() > MAX_RESPONSE_BYTES {
                return Err(UsageError(UsageStatus::InvalidResponse, "Antigravity returned an unexpected response. Update UsagePin and try again."));
            }
            bytes.extend_from_slice(&chunk);
        }
        String::from_utf8(bytes).map_err(|_| UsageError(UsageStatus::InvalidResponse, "Antigravity returned unreadable data. Try again later."))
    }

    async fn rpc(&self, base: &str, csrf: &str, method: &str, body: Value) -> Result<Value, UsageError> {
        let response = self.client.post(format!("{base}{SERVICE}/{method}"))
            .header("X-Codeium-Csrf-Token", csrf).header("Connect-Protocol-Version", "1")
            .json(&body).send().await.map_err(|_| app_not_running())?;
        serde_json::from_str(&self.read_response(response).await?).map_err(|_| {
            UsageError(UsageStatus::InvalidResponse, "Antigravity returned invalid quota data. Try again later.")
        })
    }

    async fn fetch(&self, port: u16) -> Result<(Vec<UsageWindow>, Option<Account>), UsageError> {
        // The CSRF token is only sent to this explicit loopback address. No proxies or redirects.
        let base = format!("http://127.0.0.1:{port}");
        let root = self.client.get(format!("{base}/")).send().await.map_err(|_| app_not_running())?;
        let csrf = parse_csrf(&self.read_response(root).await?)?;
        let status = self.rpc(&base, &csrf, "GetUserStatus", json!({})).await?;
        let quota = self.rpc(&base, &csrf, "RetrieveUserQuotaSummary", json!({"forceRefresh": false})).await?;
        Ok((parse_usage(&quota)?, parse_account(&status)))
    }

    async fn refresh(&self, publish: impl Fn(UsageSnapshot)) -> UsageSnapshot {
        let Ok(_gate) = self.request_gate.try_lock() else { return snapshot(&*self.inner.lock().await); };
        {
            let mut inner = self.inner.lock().await;
            inner.snapshot.refreshing = true;
            inner.snapshot.revision += 1;
            publish(snapshot(&inner));
        }
        let result = discover_port(&self.log_path);
        let result = match result { Ok(port) => self.fetch(port).await, Err(error) => Err(error) };
        let result = {
            let mut inner = self.inner.lock().await;
            inner.snapshot.refreshing = false;
            inner.snapshot.revision += 1;
            inner.last_completed = Some(Instant::now());
            match result {
                Ok((windows, account)) => {
                    inner.snapshot.status = UsageStatus::Ready;
                    inner.snapshot.message = "Connected to the running Antigravity app. Quotas follow its Models & Usage summary.".to_owned();
                    inner.snapshot.windows = windows;
                    inner.snapshot.account = account;
                    inner.snapshot.updated_at = Some(now_seconds());
                    inner.failures = 0;
                },
                Err(error) => {
                    inner.snapshot.status = error.0;
                    inner.snapshot.message = error.1.to_owned();
                    inner.snapshot.windows.clear();
                    inner.snapshot.account = None;
                    inner.snapshot.updated_at = None;
                    inner.failures = inner.failures.saturating_add(1);
                },
            }
            snapshot(&inner)
        };
        publish(result.clone());
        self.changed.notify_one();
        result
    }
}

fn client() -> Result<Client, reqwest::Error> {
    Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10)).build()
}

pub fn start(app: &AppHandle) -> Result<(), reqwest::Error> {
    let service = Arc::new(UsageService::new(client()?, log_path()));
    app.manage(UsageState(service.clone()));
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let wait = {
                let _gate = service.request_gate.lock().await;
                delay(&*service.inner.lock().await)
            };
            if !wait.is_zero() && tokio::time::timeout(wait, service.changed.notified()).await.is_ok() { continue; }
            service.refresh(|snapshot| { let _ = app.emit(EVENT, snapshot); }).await;
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn get_antigravity_usage(state: State<'_, UsageState>) -> Result<UsageSnapshot, String> {
    Ok(snapshot(&*state.0.inner.lock().await))
}

#[tauri::command]
pub async fn refresh_antigravity_usage(app: AppHandle, state: State<'_, UsageState>) -> Result<UsageSnapshot, String> {
    Ok(state.0.refresh(|snapshot| { let _ = app.emit(EVENT, snapshot); }).await)
}

pub async fn set_refresh_interval(app: &AppHandle, state: &UsageState, seconds: u64) {
    let mut inner = state.0.inner.lock().await;
    if inner.snapshot.refresh_interval_seconds == seconds { return; }
    inner.snapshot.refresh_interval_seconds = seconds;
    inner.snapshot.revision += 1;
    let _ = app.emit(EVENT, snapshot(&inner));
    state.0.changed.notify_one();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn quota() -> Value {
        json!({"response":{"groups":[
            {"displayName":"Gemini Models","buckets":[{"bucketId":"gemini-weekly","window":"weekly","remainingFraction":1,"resetTime":"2026-10-18T04:30:42Z"}]},
            {"displayName":"Claude and GPT models","buckets":[{"bucketId":"3p-weekly","window":"weekly","remainingFraction":0.25,"resetTime":"2026-10-18T11:30:42+07:00"}]}
        ]}})
    }

    #[test]
    fn native_groups_keep_separate_percentages_and_parse_rfc3339_reset() {
        let windows = parse_usage(&quota()).unwrap();
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].label, "Gemini");
        assert_eq!(windows[1].label, "Claude/GPT");
        assert_eq!(windows[0].remaining_percent, Some(100.0));
        assert_eq!(windows[1].remaining_percent, Some(25.0));
        assert_eq!(windows[0].window_seconds, Some(604800));
        assert_eq!(windows[0].resets_at, windows[1].resets_at);
        assert_eq!(windows[0].resets_at, Some(1_792_297_842));
    }

    #[test]
    fn missing_or_invalid_fraction_never_becomes_full_quota() {
        for fraction in [Value::Null, json!(-1), json!(1.1), json!("1")] {
            let mut payload = quota();
            payload["response"]["groups"][0]["buckets"][0]["remainingFraction"] = fraction;
            payload["response"]["groups"][0]["buckets"][0]["resetTime"] = json!("invalid");
            let windows = parse_usage(&payload).unwrap();
            assert_eq!(windows[0].remaining_percent, None);
            assert_eq!(windows[0].resets_at, None);
        }
        for payload in [json!({}), json!({"response":{"groups":[]}})] {
            assert_eq!(parse_usage(&payload).unwrap_err().0, UsageStatus::InvalidResponse);
        }
    }

    #[test]
    fn discovery_uses_latest_plain_http_listener_and_only_valid_ports() {
        assert_eq!(parse_port("listening on http port at 60200 for HTTP\nlistening on https port at 60201 for HTTPS"), Some(60200));
        assert_eq!(parse_port("listening on http port at 60200 for HTTP\nlistening on http port at 60300 for HTTP"), Some(60300));
        for log in ["listening on https port at 123 for HTTPS", "port at 123 for HTTP", "listening on http port at 0 for HTTP", "listening on http port at 65536 for HTTP"] {
            assert_eq!(parse_port(log), None);
        }
        let token = parse_csrf("<script>window.__APP_CONFIG__ = {\"csrfToken\":\"test-local-token\"};</script>").unwrap();
        assert_eq!(token, "test-local-token");
        assert!(parse_csrf("<script>window.__APP_CONFIG__ = {};</script>").is_err());
    }

    fn mock_server() -> (u16, std::thread::JoinHandle<Vec<String>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let server = std::thread::spawn(move || {
            let mut requests = Vec::new();
            let responses = [
                "<script>window.__APP_CONFIG__ = {\"csrfToken\":\"test-local-token\"};</script>".to_owned(),
                json!({"userStatus":{"email":"test@example.test","userTier":{"name":"Starter"}}}).to_string(),
                quota().to_string(),
            ];
            let deadline = Instant::now() + Duration::from_secs(10);
            for body in responses {
                let (mut stream, _) = loop {
                    match listener.accept() {
                        Ok(connection) => break connection,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            assert!(Instant::now() < deadline, "quota test server timed out");
                            std::thread::sleep(Duration::from_millis(10));
                        },
                        Err(error) => panic!("{error}"),
                    }
                };
                stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let mut request = Vec::new();
                loop {
                    let mut bytes = [0u8; 1024];
                    let count = stream.read(&mut bytes).unwrap();
                    request.extend_from_slice(&bytes[..count]);
                    if let Some(header_end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                        let header = String::from_utf8_lossy(&request[..header_end]).to_lowercase();
                        let content_length = header.lines().find_map(|line| line.strip_prefix("content-length: ").and_then(|value| value.parse::<usize>().ok())).unwrap_or(0);
                        if request.len() >= header_end + 4 + content_length { break; }
                    }
                    assert!(count > 0, "incomplete request");
                }
                requests.push(String::from_utf8(request).unwrap().to_lowercase());
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
            requests
        });
        (port, server)
    }

    #[test]
    fn native_rpc_uses_local_csrf_and_clears_quota_when_app_stops() {
        tauri::async_runtime::block_on(async {
            let (port, server) = mock_server();
            let path = std::env::temp_dir().join(format!("usagepin-antigravity-test-{}-{}.log", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
            std::fs::write(&path, format!("listening on http port at {port} for HTTP\n")).unwrap();
            let service = UsageService::new(client().unwrap(), path.clone());
            let ready = service.refresh(|_| {}).await;
            assert_eq!(ready.status, UsageStatus::Ready);
            assert_eq!(ready.windows[1].remaining_percent, Some(25.0));
            assert_eq!(ready.account.as_ref().unwrap().plan.as_deref(), Some("Starter"));
            let serialized = serde_json::to_string(&ready).unwrap();
            assert!(!serialized.contains("test-local-token"));
            let requests = server.join().unwrap();
            assert!(requests[0].starts_with("get / "));
            assert!(requests[2].starts_with("post /exa.language_server_pb.languageserverservice/retrieveuserquotasummary "));
            assert!(requests[2].contains("x-codeium-csrf-token: test-local-token"));
            assert!(requests[2].contains("\"forcerefresh\":false"));
            assert!(!requests.iter().any(|request| request.contains("authorization:")));
            std::fs::remove_file(&path).unwrap();
            let stopped = service.refresh(|_| {}).await;
            assert_eq!(stopped.status, UsageStatus::AppNotRunning);
            assert!(stopped.windows.is_empty());
            assert!(stopped.account.is_none());
            assert!(stopped.updated_at.is_none());
        });
    }

    #[test]
    #[ignore = "reads the running Antigravity app's local quota summary"]
    fn live_antigravity_session() {
        tauri::async_runtime::block_on(async {
            let path = log_path();
            let service = UsageService::new(client().unwrap(), path.clone());
            let (windows, _) = service.fetch(discover_port(&path).unwrap()).await.unwrap();
            for window in windows {
                println!("{}: remaining={:?}, reset={:?}", window.label, window.remaining_percent, window.resets_at);
            }
        });
    }
}
