//! Local ACP agent host: spawn a CLI and speak newline-delimited JSON-RPC.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ps_core::agents::{
    AgentChoice, AgentClientEvent, AgentPermission, AgentServer, AgentSessionStats,
    PermissionOutcome, permission_outcome, preferred_plan_mode,
};
use ps_core::{Error, Result};
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter};

/// UI event name for Assistant updates.
pub(crate) const AGENT_EVENT: &str = "agent://event";

enum Incoming {
    Line(String),
    Eof,
}

enum SessionCommand {
    Prompt(String),
    Cancel,
    SetModel(String),
    PermissionReply { id: u64, option_id: String },
    Stop,
}

/// Owns at most one live ACP subprocess.
#[derive(Clone)]
pub(crate) struct AgentHub {
    inner: Arc<Mutex<Option<LiveSession>>>,
    stats: Arc<Mutex<AgentSessionStats>>,
}

struct LiveSession {
    child: Child,
    commands: Sender<SessionCommand>,
    pump: thread::JoinHandle<()>,
    stdout: thread::JoinHandle<()>,
}

impl AgentHub {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(None)),
            stats: Arc::new(Mutex::new(AgentSessionStats::default())),
        }
    }

    pub(crate) fn session_stats(&self) -> AgentSessionStats {
        self.stats
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    fn patch_stats(&self, patch: impl FnOnce(&mut AgentSessionStats)) {
        let mut stats = self.stats.lock().unwrap_or_else(|error| error.into_inner());
        patch(&mut stats);
    }

    pub(crate) fn stop(&self) {
        let mut guard = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(mut live) = guard.take() {
            let _ = live.commands.send(SessionCommand::Stop);
            let _ = live.child.kill();
            let _ = live.child.wait();
            let _ = live.pump.join();
            let _ = live.stdout.join();
        }
        self.patch_stats(|stats| {
            stats.live = false;
            if stats.status != "No session" && !stats.status.is_empty() {
                stats.status = "Stopped".into();
            }
        });
    }

    pub(crate) fn start(
        &self,
        app: AppHandle,
        server: &AgentServer,
        cwd: &Path,
        permission: AgentPermission,
    ) -> Result<AgentClientEvent> {
        self.stop();
        if !server.enabled {
            return Err(Error::Agent {
                message: "This agent is turned off in Settings.".into(),
            });
        }

        let mut command = Command::new(&server.command);
        command
            .args(&server.args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for variable in &server.env {
            command.env(&variable.name, &variable.value);
        }

        let mut child = command.spawn().map_err(|_source| Error::Agent {
            message: format!("Couldn't start {}.", server.name),
        })?;
        let mut stdin = child.stdin.take().ok_or_else(|| Error::Agent {
            message: "Couldn't talk to the agent.".into(),
        })?;
        let stdout = child.stdout.take().ok_or_else(|| Error::Agent {
            message: "Couldn't talk to the agent.".into(),
        })?;

        let (events_tx, events_rx) = mpsc::channel();
        let stdout_thread = thread::Builder::new()
            .name("paperstreet-acp-stdout".into())
            .spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(std::result::Result::ok) {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if events_tx.send(Incoming::Line(line)).is_err() {
                        break;
                    }
                }
                let _ = events_tx.send(Incoming::Eof);
            })
            .map_err(|_source| Error::Agent {
                message: "Couldn't start the agent reader.".into(),
            })?;

        let mut next_id = 1_u64;
        let mut pending: HashMap<u64, Sender<Value>> = HashMap::new();

        write_rpc(
            &mut stdin,
            &rpc_request(
                next_id,
                "initialize",
                json!({
                    "protocolVersion": 1,
                    "clientCapabilities": {
                        "fs": { "readTextFile": false, "writeTextFile": false }
                    },
                    "clientInfo": { "name": "1537paperstreet", "title": "1537paperstreet" }
                }),
            ),
        )?;
        next_id += 1;
        let _init = wait_result(&events_rx, &mut pending, Duration::from_secs(20))?;

        let new_id = next_id;
        next_id += 1;
        write_rpc(
            &mut stdin,
            &rpc_request(
                new_id,
                "session/new",
                json!({
                    "cwd": cwd,
                    "mcpServers": []
                }),
            ),
        )?;
        let created = wait_result(&events_rx, &mut pending, Duration::from_secs(20))?;
        let session_id = created
            .get("sessionId")
            .and_then(Value::as_str)
            .unwrap_or("session")
            .to_owned();
        let models = parse_models(&created);
        let modes = parse_modes(&created);

        if permission == AgentPermission::Plan
            && let Some(mode_id) = preferred_plan_mode(&modes)
        {
            let set_id = next_id;
            next_id += 1;
            write_rpc(
                &mut stdin,
                &rpc_request(
                    set_id,
                    "session/set_mode",
                    json!({ "sessionId": session_id, "modeId": mode_id }),
                ),
            )?;
            let _ = wait_result(&events_rx, &mut pending, Duration::from_secs(8));
        }

        let ready = AgentClientEvent::Ready {
            session_id: session_id.clone(),
            models: models.clone(),
            modes: modes.clone(),
        };
        let _ = app.emit(AGENT_EVENT, &ready);

        {
            let mut stats = self.stats.lock().unwrap_or_else(|error| error.into_inner());
            *stats = AgentSessionStats::started(&server.name);
        }

        let (commands, command_rx) = mpsc::channel();
        let stats = Arc::clone(&self.stats);
        let pump = thread::Builder::new()
            .name("paperstreet-acp-pump".into())
            .spawn(move || {
                pump_loop(
                    app, events_rx, command_rx, stdin, pending, next_id, session_id, permission,
                    stats,
                );
            })
            .map_err(|_source| Error::Agent {
                message: "Couldn't start the agent session.".into(),
            })?;

        let mut guard = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        *guard = Some(LiveSession {
            child,
            commands,
            pump,
            stdout: stdout_thread,
        });
        Ok(ready)
    }

    pub(crate) fn prompt(&self, text: &str) -> Result<()> {
        self.patch_stats(|stats| {
            stats.prompts = stats.prompts.saturating_add(1);
            stats.status = "Streaming".into();
        });
        self.send(SessionCommand::Prompt(text.to_owned()))
    }

    pub(crate) fn cancel(&self) -> Result<()> {
        self.send(SessionCommand::Cancel)
    }

    pub(crate) fn set_model(&self, model_id: String) -> Result<()> {
        self.send(SessionCommand::SetModel(model_id))
    }

    pub(crate) fn permission_reply(&self, id: u64, option_id: String) -> Result<()> {
        self.send(SessionCommand::PermissionReply { id, option_id })
    }

    fn send(&self, command: SessionCommand) -> Result<()> {
        let guard = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        let live = guard.as_ref().ok_or_else(|| Error::Agent {
            message: "Start a chat first.".into(),
        })?;
        live.commands.send(command).map_err(|_source| Error::Agent {
            message: "The agent is no longer running.".into(),
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn pump_loop(
    app: AppHandle,
    events: Receiver<Incoming>,
    commands: Receiver<SessionCommand>,
    mut stdin: ChildStdin,
    mut pending: HashMap<u64, Sender<Value>>,
    mut next_id: u64,
    session_id: String,
    permission: AgentPermission,
    stats: Arc<Mutex<AgentSessionStats>>,
) {
    loop {
        if let Ok(command) = commands.try_recv() {
            match command {
                SessionCommand::Stop => break,
                SessionCommand::Prompt(text) => {
                    let id = next_id;
                    next_id += 1;
                    let payload = rpc_request(
                        id,
                        "session/prompt",
                        json!({
                            "sessionId": session_id,
                            "prompt": [{ "type": "text", "text": text }]
                        }),
                    );
                    if write_rpc(&mut stdin, &payload).is_err() {
                        emit_error(&app, "Couldn't send the prompt.");
                        break;
                    }
                }
                SessionCommand::Cancel => {
                    let payload = rpc_notice("session/cancel", json!({ "sessionId": session_id }));
                    let _ = write_rpc(&mut stdin, &payload);
                }
                SessionCommand::SetModel(model_id) => {
                    let id = next_id;
                    next_id += 1;
                    let payload = rpc_request(
                        id,
                        "session/set_config_option",
                        json!({
                            "sessionId": session_id,
                            "configId": "model",
                            "value": model_id
                        }),
                    );
                    let _ = write_rpc(&mut stdin, &payload);
                }
                SessionCommand::PermissionReply { id, option_id } => {
                    let payload = rpc_result(
                        id,
                        json!({
                            "outcome": { "outcome": "selected", "optionId": option_id }
                        }),
                    );
                    let _ = write_rpc(&mut stdin, &payload);
                }
            }
        }

        match events.recv_timeout(Duration::from_millis(40)) {
            Ok(Incoming::Eof) => {
                emit_error(&app, "The agent stopped.");
                break;
            }
            Ok(Incoming::Line(line)) => {
                if let Some(event) = handle_line(&line, &mut pending, &mut stdin, permission, &app)
                {
                    record_event(&stats, &event);
                    let _ = app.emit(AGENT_EVENT, &event);
                    if matches!(event, AgentClientEvent::Error { .. }) {
                        break;
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn record_event(stats: &Mutex<AgentSessionStats>, event: &AgentClientEvent) {
    let mut stats = stats.lock().unwrap_or_else(|error| error.into_inner());
    match event {
        AgentClientEvent::Tool { .. } => {
            stats.tools = stats.tools.saturating_add(1);
            stats.status = "Using a tool".into();
        }
        AgentClientEvent::Permission { .. } => {
            stats.permission_asks = stats.permission_asks.saturating_add(1);
            stats.status = "Waiting for permission".into();
        }
        AgentClientEvent::Done { .. } => {
            stats.status = "Idle".into();
        }
        AgentClientEvent::Error { .. } => {
            stats.live = false;
            stats.status = "Error".into();
        }
        AgentClientEvent::Message { .. } | AgentClientEvent::Ready { .. } => {}
    }
}

fn handle_line(
    line: &str,
    pending: &mut HashMap<u64, Sender<Value>>,
    stdin: &mut ChildStdin,
    permission: AgentPermission,
    app: &AppHandle,
) -> Option<AgentClientEvent> {
    let value: Value = serde_json::from_str(line).ok()?;
    if let Some(id) = rpc_id(&value)
        && value.get("method").and_then(Value::as_str) == Some("session/request_permission")
    {
        return Some(handle_permission(id, &value, stdin, permission, app));
    }
    if let Some(id) = rpc_id(&value)
        && (value.get("result").is_some() || value.get("error").is_some())
    {
        if let Some(tx) = pending.remove(&id) {
            let _ = tx.send(value.get("result").cloned().unwrap_or(Value::Null));
        }
        if let Some(reason) = value.pointer("/result/stopReason").and_then(Value::as_str) {
            return Some(AgentClientEvent::Done {
                stop_reason: reason.to_owned(),
            });
        }
        return None;
    }
    if value.get("method").and_then(Value::as_str) == Some("session/update") {
        return parse_update(value.get("params").unwrap_or(&Value::Null));
    }
    None
}

fn handle_permission(
    id: u64,
    value: &Value,
    stdin: &mut ChildStdin,
    permission: AgentPermission,
    _app: &AppHandle,
) -> AgentClientEvent {
    let params = value.get("params").unwrap_or(&Value::Null);
    let title = params
        .pointer("/toolCall/title")
        .and_then(Value::as_str)
        .unwrap_or("Tool")
        .to_owned();
    let kind = params.pointer("/toolCall/kind").and_then(Value::as_str);
    let options = parse_permission_options(params);
    match permission_outcome(permission, kind) {
        PermissionOutcome::Prompt => AgentClientEvent::Permission { id, title, options },
        outcome => {
            let option_id = pick_option(&options, outcome);
            let payload = rpc_result(
                id,
                json!({
                    "outcome": { "outcome": "selected", "optionId": option_id }
                }),
            );
            let _ = write_rpc(stdin, &payload);
            AgentClientEvent::Tool {
                title,
                status: match outcome {
                    PermissionOutcome::RejectOnce => "failed".into(),
                    _ => "in_progress".into(),
                },
            }
        }
    }
}

fn pick_option(options: &[AgentChoice], outcome: PermissionOutcome) -> String {
    let preferred: &[&str] = match outcome {
        PermissionOutcome::AllowAlways => &["allow_always", "allow-always", "allow_once"],
        PermissionOutcome::AllowOnce => &["allow_once", "allow-once", "allow_always"],
        PermissionOutcome::RejectOnce => &["reject_once", "reject-once", "reject"],
        PermissionOutcome::Prompt => &["allow_once"],
    };
    for key in preferred {
        if let Some(option) = options.iter().find(|option| {
            option.id.eq_ignore_ascii_case(key) || option.name.to_ascii_lowercase().contains(key)
        }) {
            return option.id.clone();
        }
    }
    options
        .first()
        .map(|option| option.id.clone())
        .unwrap_or_else(|| "allow_once".into())
}

fn parse_update(params: &Value) -> Option<AgentClientEvent> {
    let update = params.get("update")?;
    let kind = update
        .get("sessionUpdate")
        .and_then(Value::as_str)
        .unwrap_or("");
    match kind {
        "agent_message_chunk" | "agent_thought_chunk" => {
            let text = content_text(update.get("content").unwrap_or(&Value::Null));
            if text.is_empty() {
                None
            } else {
                Some(AgentClientEvent::Message { text })
            }
        }
        "tool_call" | "tool_call_update" => Some(AgentClientEvent::Tool {
            title: update
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("Tool")
                .to_owned(),
            status: update
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("pending")
                .to_owned(),
        }),
        _ => None,
    }
}

fn content_text(content: &Value) -> String {
    if let Some(text) = content.get("text").and_then(Value::as_str) {
        return text.to_owned();
    }
    if let Some(items) = content.as_array() {
        return items.iter().map(content_text).collect();
    }
    String::new()
}

fn parse_models(value: &Value) -> Vec<AgentChoice> {
    let mut models = Vec::new();
    if let Some(items) = value
        .pointer("/models/availableModels")
        .and_then(Value::as_array)
    {
        for item in items {
            let id = item
                .get("modelId")
                .or_else(|| item.get("id"))
                .and_then(Value::as_str);
            let name = item.get("name").and_then(Value::as_str).or(id);
            if let (Some(id), Some(name)) = (id, name) {
                models.push(AgentChoice {
                    id: id.to_owned(),
                    name: name.to_owned(),
                });
            }
        }
    }
    if let Some(items) = value.get("configOptions").and_then(Value::as_array) {
        for item in items {
            let category = item
                .get("category")
                .or_else(|| item.get("type"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if category != "model" && category != "model_config" {
                continue;
            }
            if let Some(options) = item.get("options").and_then(Value::as_array) {
                for option in options {
                    let id = option.get("id").and_then(Value::as_str);
                    let name = option.get("name").and_then(Value::as_str).or(id);
                    if let (Some(id), Some(name)) = (id, name) {
                        models.push(AgentChoice {
                            id: id.to_owned(),
                            name: name.to_owned(),
                        });
                    }
                }
            }
        }
    }
    models
}

fn parse_modes(value: &Value) -> Vec<AgentChoice> {
    let mut modes = Vec::new();
    if let Some(items) = value
        .pointer("/modes/availableModes")
        .and_then(Value::as_array)
    {
        for item in items {
            let id = item.get("id").and_then(Value::as_str);
            let name = item.get("name").and_then(Value::as_str).or(id);
            if let (Some(id), Some(name)) = (id, name) {
                modes.push(AgentChoice {
                    id: id.to_owned(),
                    name: name.to_owned(),
                });
            }
        }
    }
    modes
}

fn parse_permission_options(params: &Value) -> Vec<AgentChoice> {
    let mut options = Vec::new();
    if let Some(items) = params.get("options").and_then(Value::as_array) {
        for item in items {
            let id = item
                .get("optionId")
                .or_else(|| item.get("id"))
                .and_then(Value::as_str);
            let name = item.get("name").and_then(Value::as_str).or(id);
            if let (Some(id), Some(name)) = (id, name) {
                options.push(AgentChoice {
                    id: id.to_owned(),
                    name: name.to_owned(),
                });
            }
        }
    }
    options
}

fn rpc_id(value: &Value) -> Option<u64> {
    match value.get("id")? {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.parse().ok(),
        _ => None,
    }
}

fn rpc_request(id: u64, method: &str, params: Value) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    })
    .to_string()
}

fn rpc_notice(method: &str, params: Value) -> String {
    json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params
    })
    .to_string()
}

fn rpc_result(id: u64, result: Value) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
    .to_string()
}

fn write_rpc(stdin: &mut ChildStdin, payload: &str) -> Result<()> {
    writeln!(stdin, "{payload}").map_err(|_source| Error::Agent {
        message: "Couldn't talk to the agent.".into(),
    })?;
    stdin.flush().map_err(|_source| Error::Agent {
        message: "Couldn't talk to the agent.".into(),
    })
}

fn wait_result(
    events: &Receiver<Incoming>,
    pending: &mut HashMap<u64, Sender<Value>>,
    timeout: Duration,
) -> Result<Value> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err(Error::Agent {
                message: "The agent did not respond.".into(),
            });
        }
        match events.recv_timeout(remaining) {
            Ok(Incoming::Eof) => {
                return Err(Error::Agent {
                    message: "The agent stopped before it was ready.".into(),
                });
            }
            Ok(Incoming::Line(line)) => {
                let Ok(value) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if let Some(id) = rpc_id(&value)
                    && (value.get("result").is_some() || value.get("error").is_some())
                {
                    if let Some(tx) = pending.remove(&id) {
                        let _ = tx.send(value.clone());
                    }
                    if let Some(error) = value.get("error") {
                        let message = error
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("The agent returned an error.");
                        return Err(Error::Agent {
                            message: message.to_owned(),
                        });
                    }
                    return Ok(value.get("result").cloned().unwrap_or(Value::Null));
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                return Err(Error::Agent {
                    message: "The agent did not respond.".into(),
                });
            }
            Err(RecvTimeoutError::Disconnected) => {
                return Err(Error::Agent {
                    message: "The agent stopped before it was ready.".into(),
                });
            }
        }
    }
}

fn emit_error(app: &AppHandle, message: &str) {
    let _ = app.emit(
        AGENT_EVENT,
        AgentClientEvent::Error {
            message: message.to_owned(),
        },
    );
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io::BufRead;
    use std::time::Duration;

    use super::{
        Incoming, content_text, parse_models, parse_modes, parse_update, rpc_id, rpc_request,
        wait_result, write_rpc,
    };
    use serde_json::{Value, json};

    #[test]
    fn request_lines_are_newline_delimited_jsonrpc() {
        let encoded = rpc_request(1, "initialize", json!({ "protocolVersion": 1 }));
        let value: serde_json::Value = serde_json::from_str(&encoded).expect("json");
        assert_eq!(value["method"], "initialize");
        assert_eq!(rpc_id(&value), Some(1));
        assert!(!encoded.contains('\n'));
    }

    #[test]
    fn parses_session_models_and_plan_modes() {
        let created = json!({
            "sessionId": "sess_1",
            "models": {
                "availableModels": [{ "modelId": "kimi", "name": "Kimi" }]
            },
            "modes": {
                "availableModes": [
                    { "id": "code", "name": "Code" },
                    { "id": "plan", "name": "Plan" }
                ]
            }
        });
        assert_eq!(parse_models(&created)[0].id, "kimi");
        assert_eq!(parse_modes(&created)[1].id, "plan");
    }

    #[test]
    fn maps_agent_message_chunks_without_file_bodies() {
        let event = parse_update(&json!({
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": { "type": "text", "text": "Hello" }
            }
        }))
        .expect("chunk");
        match event {
            ps_core::agents::AgentClientEvent::Message { text } => {
                assert_eq!(text, "Hello");
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(content_text(&json!({ "text": "x" })), "x");
    }

    const FAKE_ACP: &str = "PAPERSTREET_FAKE_ACP";

    #[test]
    fn fake_stdio_agent_completes_initialize_and_session_new() {
        if std::env::var_os(FAKE_ACP).is_some() {
            run_fake_acp();
            return;
        }

        let mut child = std::process::Command::new(std::env::current_exe().expect("test exe"))
            .args([
                "--exact",
                "agent::tests::fake_stdio_agent_completes_initialize_and_session_new",
                "--nocapture",
            ])
            .env(FAKE_ACP, "1")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("fake agent");
        let mut stdin = child.stdin.take().expect("stdin");
        let stdout = child.stdout.take().expect("stdout");
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines().map_while(std::result::Result::ok) {
                if tx.send(Incoming::Line(line)).is_err() {
                    break;
                }
            }
        });
        write_rpc(
            &mut stdin,
            &rpc_request(1, "initialize", json!({ "protocolVersion": 1 })),
        )
        .expect("init");
        let mut pending = HashMap::new();
        wait_result(&rx, &mut pending, Duration::from_secs(5)).expect("initialized");
        write_rpc(
            &mut stdin,
            &rpc_request(2, "session/new", json!({ "cwd": "/tmp", "mcpServers": [] })),
        )
        .expect("session");
        let created = wait_result(&rx, &mut pending, Duration::from_secs(5)).expect("created");
        assert_eq!(created["sessionId"], "sess_test");
        let _ = child.kill();
        let _ = child.wait();
    }

    fn run_fake_acp() {
        use std::io::{self, Write};
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        for line in stdin.lock().lines().map_while(std::result::Result::ok) {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
                continue;
            };
            let id = value.get("id").cloned().unwrap_or(json!(0));
            let method = value.get("method").and_then(Value::as_str).unwrap_or("");
            let result = match method {
                "initialize" => json!({ "protocolVersion": 1 }),
                "session/new" => json!({
                    "sessionId": "sess_test",
                    "modes": {
                        "availableModes": [{ "id": "plan", "name": "Plan" }]
                    }
                }),
                _ => json!({}),
            };
            let encoded = json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string();
            writeln!(stdout, "{encoded}").expect("reply");
            stdout.flush().expect("flush");
        }
    }
}
