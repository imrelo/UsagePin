use crate::codex_usage::{UsageSnapshot, UsageStatus};
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, path::PathBuf, time::Duration};

pub const EVENT: &str = "codex-auto-activation-updated";
pub const MODELS_URL: &str = "https://chatgpt.com/backend-api/codex/models?client_version=0.157.0";
pub const RESPONSES_URL: &str = "https://chatgpt.com/backend-api/codex/responses";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivationSnapshot {
    pub revision: u64,
    pub enabled: bool,
    pub message: String,
    pub last_attempt_at: Option<u64>,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
struct AccountProgress {
    observed_reset: Option<u64>,
    attempted_reset: Option<u64>,
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
struct SavedActivation {
    enabled: bool,
    accounts: HashMap<String, AccountProgress>,
    last_attempt_at: Option<u64>,
}

impl Default for SavedActivation {
    fn default() -> Self {
        Self {
            enabled: true,
            accounts: HashMap::new(),
            last_attempt_at: None,
        }
    }
}

pub struct AutoActivation {
    path: Option<PathBuf>,
    saved: SavedActivation,
    pub snapshot: ActivationSnapshot,
}

impl Default for AutoActivation {
    fn default() -> Self {
        Self {
            path: None,
            saved: SavedActivation::default(),
            snapshot: ActivationSnapshot {
                revision: 0,
                enabled: true,
                message: "Waiting for the next 5-hour quota reset.".to_owned(),
                last_attempt_at: None,
            },
        }
    }
}

impl AutoActivation {
    pub fn load(path: Result<PathBuf, String>) -> Self {
        let mut activation = Self::default();
        match path {
            Ok(path) => {
                match std::fs::read(&path) {
                    Ok(bytes) => match serde_json::from_slice::<SavedActivation>(&bytes) {
                        Ok(saved) => activation.saved = saved,
                        Err(_) => {
                            activation.saved.enabled = false;
                            activation.snapshot.message =
                                "Could not read automatic activation settings. Enable it again to save new settings.".to_owned();
                        }
                    },
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(_) => {
                        activation.saved.enabled = false;
                        activation.snapshot.message = "Could not read automatic activation settings.".to_owned();
                    }
                }
                activation.path = Some(path);
            }
            Err(_) => {
                activation.saved.enabled = false;
                activation.snapshot.message =
                    "Automatic activation settings storage is unavailable.".to_owned()
            }
        }
        activation.snapshot.enabled = activation.saved.enabled;
        activation.snapshot.last_attempt_at = activation.saved.last_attempt_at;
        if activation.saved.enabled {
            activation.snapshot.message = "Waiting for the next 5-hour quota reset.".to_owned();
        }
        activation
    }

    fn persist(&self) -> Result<(), String> {
        let path = self
            .path
            .as_ref()
            .ok_or("Automatic activation settings storage is unavailable.")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|_| "Could not save automatic activation settings.")?;
        }
        let bytes = serde_json::to_vec(&self.saved)
            .map_err(|_| "Could not save automatic activation settings.")?;
        let temporary = path.with_extension("tmp");
        std::fs::write(&temporary, bytes)
            .map_err(|_| "Could not save automatic activation settings.")?;
        std::fs::rename(&temporary, path)
            .map_err(|_| "Could not save automatic activation settings.".to_owned())
    }

    pub fn set_enabled(&mut self, enabled: bool) -> Result<(), String> {
        let previous = self.saved.enabled;
        self.saved.enabled = enabled;
        if let Err(error) = self.persist() {
            self.saved.enabled = previous;
            return Err(error);
        }
        self.snapshot.enabled = enabled;
        self.snapshot.revision += 1;
        self.snapshot.message = if enabled {
            "Waiting for the next 5-hour quota reset."
        } else {
            "Automatic activation is off."
        }
        .to_owned();
        Ok(())
    }

    pub fn claim(
        &mut self,
        identity: &str,
        usage: &UsageSnapshot,
        now: u64,
    ) -> Result<Option<u64>, String> {
        if !self.saved.enabled || usage.status != UsageStatus::Ready || usage.refreshing {
            return Ok(None);
        }
        let Some(window) = usage
            .windows
            .iter()
            .find(|window| window.id == "primary" && window.window_seconds == Some(18_000))
        else {
            return Ok(None);
        };
        let Some(reset) = window.resets_at else {
            return Ok(None);
        };
        let progress = self.saved.accounts.entry(identity.to_owned()).or_default();
        // Preserve the deadline seen before a refresh advances the server's reset timestamp.
        let unhandled = |reset: &u64| {
            *reset <= now
                && progress
                    .attempted_reset
                    .is_none_or(|attempted| *reset > attempted)
        };
        let due_reset = progress
            .observed_reset
            .filter(unhandled)
            .or_else(|| Some(reset).filter(unhandled));
        let previous_reset = progress.observed_reset;
        progress.observed_reset = Some(reset);
        let available = window
            .remaining_percent
            .is_some_and(|remaining| remaining > 0.0)
            && usage
                .windows
                .iter()
                .filter(|window| window.id == "secondary")
                .all(|window| {
                    window
                        .remaining_percent
                        .is_some_and(|remaining| remaining > 0.0)
                });
        if let Some(due_reset) = due_reset.filter(|_| available) {
            // Persist BEFORE the request: an interrupted stream or restart must never send it twice.
            let previous_attempt = progress.attempted_reset;
            progress.attempted_reset = Some(due_reset);
            let previous_time = self.saved.last_attempt_at;
            self.saved.last_attempt_at = Some(now);
            if let Err(error) = self.persist() {
                let progress = self.saved.accounts.get_mut(identity).unwrap();
                progress.attempted_reset = previous_attempt;
                progress.observed_reset = previous_reset;
                self.saved.last_attempt_at = previous_time;
                return Err(error);
            }
            self.snapshot.last_attempt_at = Some(now);
            self.snapshot.revision += 1;
            self.snapshot.message = "Activating the new 5-hour quota window…".to_owned();
            return Ok(Some(due_reset));
        }
        // Keep a missed reset pending while quota is unavailable.
        if due_reset.is_some() && !available {
            self.saved
                .accounts
                .get_mut(identity)
                .unwrap()
                .observed_reset = due_reset;
        }
        if self.saved.accounts[identity].observed_reset != previous_reset {
            self.persist()?;
        }
        Ok(None)
    }

    pub fn finish(&mut self, result: Result<(), String>) {
        self.snapshot.revision += 1;
        self.snapshot.message = if !self.snapshot.enabled {
            "Automatic activation is off.".to_owned()
        } else {
            match result {
                Ok(()) => {
                    "Activation request completed. Waiting for the next 5-hour reset.".to_owned()
                }
                Err(error) => format!("{error} No repeat request will be sent for this reset."),
            }
        };
    }
}

fn activation_http_error(status: reqwest::StatusCode) -> String {
    match status.as_u16() {
        401 => "Activation failed: your Codex session expired. Sign in again in Codex.",
        403 => "Activation failed: this account does not have access to the model.",
        429 => "Activation failed: Codex usage is currently limited.",
        _ => "Activation failed: the Codex service could not complete the request.",
    }
    .to_owned()
}

fn activation_payload(catalog: &Value) -> Result<Value, String> {
    let models = catalog["models"]
        .as_array()
        .ok_or("Codex returned an unreadable model list.")?;
    let model = models
        .iter()
        .filter(|model| model["visibility"] == "list" && model["slug"].is_string())
        .min_by_key(|model| {
            let slug = model["slug"].as_str().unwrap_or_default();
            let family = if slug.contains("luna") {
                0
            } else if slug.contains("sol") {
                1
            } else {
                2
            };
            (family, model["priority"].as_u64().unwrap_or(u64::MAX))
        })
        .and_then(|model| model["slug"].as_str().map(|slug| (slug, model)))
        .ok_or("No Codex model is available for automatic activation.")?;
    let mut payload = json!({
        "model": model.0,
        "instructions": "Reply with exactly OK.",
        "input": [{"role": "user", "content": [{"type": "input_text", "text": "Reply with exactly OK."}]}],
        "tools": [], "tool_choice": "none", "parallel_tool_calls": false,
        "store": false, "stream": true, "include": []
    });
    if model.1["supported_reasoning_levels"]
        .as_array()
        .is_some_and(|levels| levels.iter().any(|level| level["effort"] == "low"))
    {
        payload["reasoning"] = json!({"effort": "low"});
    }
    Ok(payload)
}

pub async fn send_activation(
    client: &Client,
    models_url: &str,
    responses_url: &str,
    token: &str,
    account_id: &str,
) -> Result<(), String> {
    let catalog = client
        .get(models_url)
        .bearer_auth(token)
        .header("chatgpt-account-id", account_id)
        .header("originator", "codex_cli_rs")
        .send()
        .await
        .map_err(|_| "Could not reach Codex for automatic activation.")?;
    if !catalog.status().is_success() {
        return Err(activation_http_error(catalog.status()));
    }
    let catalog = catalog
        .json::<Value>()
        .await
        .map_err(|_| "Codex returned an unreadable model list.")?;
    let payload = activation_payload(&catalog)?;
    // Never retry this POST: the server might already have counted a timed-out request.
    let mut response = client
        .post(responses_url)
        .bearer_auth(token)
        .header("chatgpt-account-id", account_id)
        .header("originator", "codex_cli_rs")
        .header(header::ACCEPT, "text/event-stream")
        .timeout(Duration::from_secs(60))
        .json(&payload)
        .send()
        .await
        .map_err(|_| "The activation request was interrupted; its result is unknown.")?;
    if !response.status().is_success() {
        return Err(activation_http_error(response.status()));
    }
    let mut pending = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "The activation stream was interrupted; its result is unknown.")?
    {
        pending.extend_from_slice(&chunk);
        if pending.len() > 262_144 {
            return Err("Codex returned an oversized activation event.".to_owned());
        }
        while let Some(end) = pending.iter().position(|byte| *byte == b'\n') {
            let line: Vec<_> = pending.drain(..=end).collect();
            let line = std::str::from_utf8(&line)
                .map_err(|_| "Codex returned an unreadable activation event.")?
                .trim();
            if let Some(data) = line.strip_prefix("data:") {
                let data = data.trim();
                if data == "[DONE]" {
                    continue;
                }
                let event: Value = serde_json::from_str(data)
                    .map_err(|_| "Codex returned an unreadable activation event.")?;
                match event["type"].as_str() {
                    Some("response.completed") => return Ok(()),
                    Some("response.failed" | "response.incomplete" | "error") => {
                        return Err("Codex did not complete the activation request.".to_owned())
                    }
                    _ => {}
                }
            }
        }
    }
    Err("The activation stream ended without confirmation; its result is unknown.".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex_usage::UsageWindow;

    struct TestStorage(PathBuf);

    impl TestStorage {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!(
                    "usagepin-activation-{}-{}.json",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                )))
        }

        fn enabled(&self) -> AutoActivation {
            let mut activation = AutoActivation::load(Ok(self.0.clone()));
            activation.set_enabled(true).unwrap();
            activation
        }
    }

    impl Drop for TestStorage {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
            let _ = std::fs::remove_file(self.0.with_extension("tmp"));
        }
    }

    fn usage(reset: u64) -> UsageSnapshot {
        UsageSnapshot {
            revision: 1,
            status: UsageStatus::Ready,
            refreshing: false,
            message: String::new(),
            source_path: String::new(),
            account: None,
            windows: vec![UsageWindow {
                id: "primary".to_owned(),
                remaining_percent: Some(100.0),
                window_seconds: Some(18_000),
                resets_at: Some(reset),
            }],
            updated_at: Some(100),
            refresh_interval_seconds: 60,
            next_refresh_at_ms: None,
        }
    }

    #[test]
    fn new_preferences_default_on_and_saved_off_survives_reload() {
        let storage = TestStorage::new();
        let mut activation = AutoActivation::load(Ok(storage.0.clone()));
        assert!(activation.snapshot.enabled);
        activation.set_enabled(false).unwrap();
        assert!(!AutoActivation::load(Ok(storage.0.clone())).snapshot.enabled);
        std::fs::write(&storage.0, "{}").unwrap();
        assert!(AutoActivation::load(Ok(storage.0.clone())).snapshot.enabled);
    }

    #[test]
    fn unreadable_preferences_do_not_replace_an_unknown_saved_choice_with_on() {
        let storage = TestStorage::new();
        std::fs::write(&storage.0, "invalid json").unwrap();
        let activation = AutoActivation::load(Ok(storage.0.clone()));
        assert!(!activation.snapshot.enabled);
        assert!(activation.snapshot.message.contains("Could not read"));
        assert!(!AutoActivation::load(Err("Unavailable".to_owned())).snapshot.enabled);
    }

    #[test]
    fn only_claims_after_reset_once_even_after_restart_or_toggle() {
        let storage = TestStorage::new();
        let mut activation = storage.enabled();
        assert_eq!(
            activation.claim("account:user", &usage(100), 99).unwrap(),
            None
        );
        assert_eq!(
            activation.claim("account:user", &usage(100), 100).unwrap(),
            Some(100)
        );
        assert_eq!(
            activation.claim("account:user", &usage(100), 101).unwrap(),
            None
        );
        let mut reopened = AutoActivation::load(Ok(storage.0.clone()));
        assert!(reopened.snapshot.enabled);
        assert_eq!(
            reopened.claim("account:user", &usage(100), 102).unwrap(),
            None
        );
        reopened.set_enabled(false).unwrap();
        assert_eq!(
            reopened.claim("account:user", &usage(200), 200).unwrap(),
            None
        );
        reopened.set_enabled(true).unwrap();
        assert_eq!(
            reopened.claim("account:user", &usage(100), 202).unwrap(),
            None
        );
        assert_eq!(
            reopened.claim("account:user", &usage(200), 202).unwrap(),
            Some(200)
        );
    }

    #[test]
    fn preserves_due_reset_when_server_advances_timestamp_and_tracks_accounts_separately() {
        let storage = TestStorage::new();
        let mut activation = storage.enabled();
        assert_eq!(activation.claim("team:one", &usage(100), 90).unwrap(), None);
        let mut activation = AutoActivation::load(Ok(storage.0.clone()));
        assert_eq!(
            activation.claim("team:one", &usage(18_100), 101).unwrap(),
            Some(100)
        );
        assert_eq!(
            activation.claim("team:one", &usage(18_100), 102).unwrap(),
            None
        );
        assert_eq!(
            activation.claim("team:two", &usage(100), 102).unwrap(),
            Some(100)
        );
        assert_eq!(
            activation
                .claim("team:one", &usage(18_100), 18_101)
                .unwrap(),
            Some(18_100)
        );
    }

    #[test]
    fn skips_disabled_missing_stale_wrong_window_and_depleted_quota() {
        let storage = TestStorage::new();
        let mut activation = AutoActivation::load(Ok(storage.0.clone()));
        activation.set_enabled(false).unwrap();
        assert!(!activation.snapshot.enabled);
        assert_eq!(activation.claim("one", &usage(100), 101).unwrap(), None);
        activation.set_enabled(true).unwrap();
        for status in [
            UsageStatus::Connecting,
            UsageStatus::ExpiredSession,
            UsageStatus::NetworkError,
            UsageStatus::RateLimited,
        ] {
            let mut unavailable = usage(100);
            unavailable.status = status;
            assert_eq!(activation.claim("one", &unavailable, 101).unwrap(), None);
        }
        let mut refreshing = usage(100);
        refreshing.refreshing = true;
        assert_eq!(activation.claim("one", &refreshing, 101).unwrap(), None);
        let mut missing = usage(100);
        missing.windows[0].resets_at = None;
        assert_eq!(activation.claim("one", &missing, 101).unwrap(), None);
        let mut wrong_window = usage(100);
        wrong_window.windows[0].window_seconds = Some(604_800);
        assert_eq!(activation.claim("one", &wrong_window, 101).unwrap(), None);
        for remaining in [None, Some(0.0)] {
            let mut depleted = usage(100);
            depleted.windows[0].remaining_percent = remaining;
            assert_eq!(activation.claim("one", &depleted, 101).unwrap(), None);
        }
        let mut weekly_depleted = usage(100);
        weekly_depleted.windows.push(UsageWindow {
            id: "secondary".to_owned(),
            remaining_percent: Some(0.0),
            window_seconds: Some(604_800),
            resets_at: Some(200),
        });
        assert_eq!(
            activation.claim("one", &weekly_depleted, 101).unwrap(),
            None
        );
        // Even if a subsequent refresh advances the deadline, the missed reset is still due.
        assert_eq!(
            activation.claim("one", &usage(18_100), 102).unwrap(),
            Some(100)
        );
    }

    #[test]
    fn save_failure_prevents_activation_and_does_not_claim_a_reset() {
        let storage = TestStorage::new();
        let mut activation = storage.enabled();
        activation.path = None;
        assert!(activation.claim("one", &usage(100), 101).is_err());
        assert_eq!(activation.saved.accounts["one"].attempted_reset, None);
        activation.path = Some(storage.0.clone());
        assert_eq!(
            activation.claim("one", &usage(100), 102).unwrap(),
            Some(100)
        );
    }

    #[test]
    fn failed_or_interrupted_activation_is_not_retried_for_the_same_reset() {
        let storage = TestStorage::new();
        let mut activation = storage.enabled();
        assert_eq!(
            activation.claim("one", &usage(100), 101).unwrap(),
            Some(100)
        );
        activation.finish(Err("Interrupted".to_owned()));
        assert!(activation.snapshot.message.contains("No repeat"));
        assert_eq!(activation.claim("one", &usage(100), 102).unwrap(), None);
        assert_eq!(
            AutoActivation::load(Ok(storage.0.clone()))
                .claim("one", &usage(100), 103)
                .unwrap(),
            None
        );
    }

    #[test]
    fn payload_uses_available_model_with_no_tools_or_conversation_storage() {
        let payload = activation_payload(&json!({"models": [
            {"slug":"hidden", "priority":0, "visibility":"hide"},
            {"slug":"available-second", "priority":2, "visibility":"list"},
            {"slug":"available-first", "priority":1, "visibility":"list", "supported_reasoning_levels":[{"effort":"low"}]}
        ]})).unwrap();
        assert_eq!(payload["model"], "available-first");
        assert_eq!(payload["tools"], json!([]));
        assert_eq!(payload["tool_choice"], "none");
        assert_eq!(payload["store"], false);
        assert_eq!(payload["stream"], true);
        assert_eq!(payload["reasoning"]["effort"], "low");
        assert!(activation_payload(&json!({"models":[]})).is_err());
        let lightweight = activation_payload(&json!({"models": [
            {"slug":"gpt-6-astra", "priority":0, "visibility":"list"},
            {"slug":"gpt-6-luna", "priority":5, "visibility":"list"}
        ]}))
        .unwrap();
        assert_eq!(lightweight["model"], "gpt-6-luna");
    }

    fn server(status: u16, chunks: Vec<Vec<u8>>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let thread = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for index in 0..2 {
                let started = std::time::Instant::now();
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error)
                            if error.kind() == std::io::ErrorKind::WouldBlock
                                && started.elapsed() < Duration::from_secs(5) =>
                        {
                            std::thread::sleep(Duration::from_millis(5))
                        }
                        Err(error) => panic!("mock server did not receive request: {error}"),
                    }
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                loop {
                    let mut bytes = [0; 1024];
                    let count = stream.read(&mut bytes).unwrap();
                    assert!(count > 0);
                    request.extend_from_slice(&bytes[..count]);
                    if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&request[..end]);
                        let length = headers
                            .lines()
                            .find_map(|line| {
                                line.to_lowercase()
                                    .strip_prefix("content-length:")
                                    .and_then(|length| length.trim().parse::<usize>().ok())
                            })
                            .unwrap_or(0);
                        if request.len() >= end + 4 + length {
                            break;
                        }
                    }
                }
                requests.push(String::from_utf8(request).unwrap());
                if index == 0 {
                    let body = r#"{"models":[{"slug":"test-model","visibility":"list"}]}"#;
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .unwrap();
                } else {
                    let length: usize = chunks.iter().map(Vec::len).sum();
                    write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: text/event-stream\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n").unwrap();
                    for chunk in &chunks {
                        if stream.write_all(chunk).is_err() {
                            break;
                        }
                        stream.flush().unwrap();
                        std::thread::sleep(Duration::from_millis(5));
                    }
                }
            }
            requests
        });
        (url, thread)
    }

    #[test]
    fn sends_one_authenticated_post_and_requires_completed_stream() {
        tauri::async_runtime::block_on(async {
            for (status, chunks, completed) in [
                (200, vec![b"data: {\"type\":\"response.created\"}\r\n\r\ndata: {\"type\":\"response.com".to_vec(), b"pleted\"}\n\n".to_vec()], true),
                (200, vec![b"data: {\"type\":\"response.failed\"}\n\n".to_vec()], false),
                (200, vec![b"data: {\"type\":\"response.incomplete\"}\n\n".to_vec()], false),
                (200, vec![b"data: {\"type\":\"response.created\"}\n\n".to_vec()], false),
                (429, vec![], false),
            ] {
                let (url, thread) = server(status, chunks);
                let client = Client::builder().no_proxy().timeout(Duration::from_secs(5)).build().unwrap();
                let result = send_activation(&client, &format!("{url}/models"), &format!("{url}/responses"), "test-token", "test-account").await;
                assert_eq!(result.is_ok(), completed, "{result:?}");
                let requests = thread.join().unwrap();
                assert_eq!(requests.len(), 2);
                assert!(requests[0].starts_with("GET /models "));
                assert!(requests[1].starts_with("POST /responses "));
                assert!(requests[1].to_lowercase().contains("authorization: bearer test-token"));
                assert!(requests[1].to_lowercase().contains("chatgpt-account-id: test-account"));
                let body: Value = serde_json::from_str(requests[1].split("\r\n\r\n").nth(1).unwrap()).unwrap();
                assert_eq!(body["model"], "test-model");
                assert_eq!(body["store"], false);
            }
        });
    }
}
