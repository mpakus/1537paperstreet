//! Local ACP agent host: spawn a CLI and speak newline-delimited JSON-RPC.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ps_core::agents::{
    AgentChoice, AgentClientEvent, AgentPermission, AgentPreset, AgentServer, AgentSessionStats,
    PermissionOutcome, login_path, permission_outcome, preferred_plan_mode,
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

        let (program, args, wire) = resolve_agent_command(server);
        let mut command = Command::new(&program);
        command
            .args(&args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env(
                "PATH",
                login_path(
                    std::env::var_os("PATH").as_deref(),
                    std::env::var_os("HOME").as_deref().map(Path::new),
                ),
            );
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
        let stderr = child.stderr.take().ok_or_else(|| Error::Agent {
            message: "Couldn't talk to the agent.".into(),
        })?;

        let stderr_buf = Arc::new(Mutex::new(String::new()));
        let stderr_done = Arc::new(AtomicBool::new(false));
        let stderr_writer = Arc::clone(&stderr_buf);
        let stderr_flag = Arc::clone(&stderr_done);
        thread::Builder::new()
            .name("paperstreet-acp-stderr".into())
            .spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(std::result::Result::ok) {
                    let mut buf = stderr_writer
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());
                    if buf.len() < 4_096 {
                        if !buf.is_empty() {
                            buf.push('\n');
                        }
                        buf.push_str(&line);
                    }
                }
                stderr_flag.store(true, Ordering::SeqCst);
            })
            .map_err(|_source| Error::Agent {
                message: "Couldn't start the agent reader.".into(),
            })?;
        let take_stderr = {
            let stderr_buf = Arc::clone(&stderr_buf);
            let stderr_done = Arc::clone(&stderr_done);
            move || collect_stderr(&stderr_buf, &stderr_done)
        };

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

        let init_params = match wire {
            AgentWire::Acp => json!({
                "protocolVersion": 1,
                "clientCapabilities": {
                    "fs": { "readTextFile": false, "writeTextFile": false }
                },
                "clientInfo": {
                    "name": "1537paperstreet",
                    "title": "1537paperstreet",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
            AgentWire::CodexApp => json!({
                "clientInfo": {
                    "name": "1537paperstreet",
                    "title": "1537paperstreet",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        };
        write_rpc(
            &mut stdin,
            &rpc_request_for(next_id, "initialize", init_params, wire),
        )?;
        next_id += 1;
        let _init = wait_result(
            &events_rx,
            &mut pending,
            Duration::from_secs(20),
            &take_stderr,
        )?;

        if wire == AgentWire::CodexApp {
            write_rpc(&mut stdin, &rpc_notice_for("initialized", json!({}), wire))?;
        }

        let new_id = next_id;
        next_id += 1;
        let session_params = match wire {
            AgentWire::Acp => json!({
                "cwd": cwd,
                "mcpServers": []
            }),
            AgentWire::CodexApp => json!({
                "cwd": cwd,
                "approvalPolicy": codex_approval_policy(permission),
                "sandbox": codex_sandbox(permission)
            }),
        };
        write_rpc(
            &mut stdin,
            &rpc_request_for(
                new_id,
                match wire {
                    AgentWire::Acp => "session/new",
                    AgentWire::CodexApp => "thread/start",
                },
                session_params,
                wire,
            ),
        )?;
        let created = wait_result(
            &events_rx,
            &mut pending,
            Duration::from_secs(20),
            &take_stderr,
        )?;
        let session_id = match wire {
            AgentWire::Acp => created
                .get("sessionId")
                .and_then(Value::as_str)
                .unwrap_or("session")
                .to_owned(),
            AgentWire::CodexApp => created
                .pointer("/thread/id")
                .or_else(|| created.pointer("/thread/sessionId"))
                .and_then(Value::as_str)
                .unwrap_or("session")
                .to_owned(),
        };
        let models = parse_models(&created);
        let modes = parse_modes(&created);

        if wire == AgentWire::Acp
            && permission == AgentPermission::Plan
            && let Some(mode_id) = preferred_plan_mode(&modes)
        {
            let set_id = next_id;
            next_id += 1;
            write_rpc(
                &mut stdin,
                &rpc_request_for(
                    set_id,
                    "session/set_mode",
                    json!({ "sessionId": session_id, "modeId": mode_id }),
                    wire,
                ),
            )?;
            let _ = wait_result(
                &events_rx,
                &mut pending,
                Duration::from_secs(8),
                &take_stderr,
            );
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
                    wire, stats,
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
    wire: AgentWire,
    stats: Arc<Mutex<AgentSessionStats>>,
) {
    let mut turn_id: Option<String> = None;
    loop {
        if let Ok(command) = commands.try_recv() {
            match command {
                SessionCommand::Stop => break,
                SessionCommand::Prompt(text) => {
                    let id = next_id;
                    next_id += 1;
                    let payload = match wire {
                        AgentWire::Acp => rpc_request_for(
                            id,
                            "session/prompt",
                            json!({
                                "sessionId": session_id,
                                "prompt": [{ "type": "text", "text": text }]
                            }),
                            wire,
                        ),
                        AgentWire::CodexApp => rpc_request_for(
                            id,
                            "turn/start",
                            json!({
                                "threadId": session_id,
                                "input": [{ "type": "text", "text": text }]
                            }),
                            wire,
                        ),
                    };
                    if write_rpc(&mut stdin, &payload).is_err() {
                        emit_error(&app, "Couldn't send the prompt.");
                        break;
                    }
                }
                SessionCommand::Cancel => match wire {
                    AgentWire::Acp => {
                        let payload = rpc_notice_for(
                            "session/cancel",
                            json!({ "sessionId": session_id }),
                            wire,
                        );
                        let _ = write_rpc(&mut stdin, &payload);
                    }
                    AgentWire::CodexApp => {
                        if let Some(active) = turn_id.clone() {
                            let id = next_id;
                            next_id += 1;
                            let payload = rpc_request_for(
                                id,
                                "turn/interrupt",
                                json!({ "threadId": session_id, "turnId": active }),
                                wire,
                            );
                            let _ = write_rpc(&mut stdin, &payload);
                        }
                    }
                },
                SessionCommand::SetModel(model_id) => {
                    if wire == AgentWire::Acp {
                        let id = next_id;
                        next_id += 1;
                        let payload = rpc_request_for(
                            id,
                            "session/set_config_option",
                            json!({
                                "sessionId": session_id,
                                "configId": "model",
                                "value": model_id
                            }),
                            wire,
                        );
                        let _ = write_rpc(&mut stdin, &payload);
                    }
                }
                SessionCommand::PermissionReply { id, option_id } => {
                    let payload = match wire {
                        AgentWire::Acp => rpc_result_for(
                            id,
                            json!({
                                "outcome": { "outcome": "selected", "optionId": option_id }
                            }),
                            wire,
                        ),
                        AgentWire::CodexApp => rpc_result_for(id, json!(option_id), wire),
                    };
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
                if let Some(event) = handle_line(
                    &line,
                    &mut pending,
                    &mut stdin,
                    permission,
                    wire,
                    &mut turn_id,
                    &app,
                ) {
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
    wire: AgentWire,
    turn_id: &mut Option<String>,
    app: &AppHandle,
) -> Option<AgentClientEvent> {
    let value: Value = serde_json::from_str(line).ok()?;
    let method = value.get("method").and_then(Value::as_str).unwrap_or("");
    if let Some(id) = rpc_id(&value) {
        if method == "session/request_permission" {
            return Some(handle_permission(id, &value, stdin, permission, wire, app));
        }
        if is_codex_approval(method) {
            return Some(handle_codex_approval(
                id, method, &value, stdin, permission, wire, app,
            ));
        }
        if value.get("result").is_some() || value.get("error").is_some() {
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
    }
    if method == "session/update" {
        return parse_update(value.get("params").unwrap_or(&Value::Null));
    }
    parse_codex_event(&value, turn_id)
}

fn is_codex_approval(method: &str) -> bool {
    matches!(
        method,
        "item/commandExecution/requestApproval"
            | "item/fileChange/requestApproval"
            | "item/permissions/requestApproval"
    )
}

fn handle_permission(
    id: u64,
    value: &Value,
    stdin: &mut ChildStdin,
    permission: AgentPermission,
    wire: AgentWire,
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
            let payload = rpc_result_for(
                id,
                json!({
                    "outcome": { "outcome": "selected", "optionId": option_id }
                }),
                wire,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AgentWire {
    Acp,
    CodexApp,
}

fn resolve_agent_command(server: &AgentServer) -> (String, Vec<String>, AgentWire) {
    let name = Path::new(&server.command)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(server.command.as_str());
    if name == "codex-acp" {
        return (server.command.clone(), server.args.clone(), AgentWire::Acp);
    }
    if name == "codex" && uses_codex_app_server(&server.args, server.preset) {
        let args = if server.args.first().map(String::as_str) == Some("app-server") {
            let mut args = server.args.clone();
            if !args.iter().any(|arg| arg == "--stdio") {
                args.push("--stdio".into());
            }
            args
        } else {
            vec!["app-server".into(), "--stdio".into()]
        };
        return (server.command.clone(), args, AgentWire::CodexApp);
    }
    (server.command.clone(), server.args.clone(), AgentWire::Acp)
}

fn uses_codex_app_server(args: &[String], preset: AgentPreset) -> bool {
    args.first().map(String::as_str) == Some("app-server")
        || args.first().map(String::as_str) == Some("acp") && args.len() == 1
        || (preset == AgentPreset::Codex && args.is_empty())
}

fn codex_approval_policy(permission: AgentPermission) -> &'static str {
    match permission {
        AgentPermission::Full => "never",
        AgentPermission::Allowance | AgentPermission::Plan => "on-request",
    }
}

fn codex_sandbox(permission: AgentPermission) -> &'static str {
    match permission {
        AgentPermission::Plan => "read-only",
        AgentPermission::Full | AgentPermission::Allowance => "workspace-write",
    }
}

fn handle_codex_approval(
    id: u64,
    method: &str,
    value: &Value,
    stdin: &mut ChildStdin,
    permission: AgentPermission,
    wire: AgentWire,
    _app: &AppHandle,
) -> AgentClientEvent {
    let params = value.get("params").unwrap_or(&Value::Null);
    let title = params
        .get("command")
        .and_then(Value::as_str)
        .or_else(|| params.get("reason").and_then(Value::as_str))
        .unwrap_or("Tool")
        .to_owned();
    let kind = if method.contains("fileChange") {
        Some("edit")
    } else if method.contains("commandExecution") {
        Some("execute")
    } else {
        None
    };
    let options = vec![
        AgentChoice {
            id: "accept".into(),
            name: "Allow".into(),
        },
        AgentChoice {
            id: "acceptForSession".into(),
            name: "Allow for session".into(),
        },
        AgentChoice {
            id: "decline".into(),
            name: "Reject".into(),
        },
    ];
    match permission_outcome(permission, kind) {
        PermissionOutcome::Prompt => AgentClientEvent::Permission { id, title, options },
        outcome => {
            let decision = match outcome {
                PermissionOutcome::RejectOnce => "decline",
                PermissionOutcome::AllowAlways => "acceptForSession",
                PermissionOutcome::AllowOnce | PermissionOutcome::Prompt => "accept",
            };
            let payload = rpc_result_for(id, json!(decision), wire);
            let _ = write_rpc(stdin, &payload);
            AgentClientEvent::Tool {
                title,
                status: if decision == "decline" {
                    "failed".into()
                } else {
                    "in_progress".into()
                },
            }
        }
    }
}

fn parse_codex_event(value: &Value, turn_id: &mut Option<String>) -> Option<AgentClientEvent> {
    let method = value.get("method").and_then(Value::as_str)?;
    let params = value.get("params").unwrap_or(&Value::Null);
    match method {
        "turn/started" => {
            if let Some(id) = params.pointer("/turn/id").and_then(Value::as_str) {
                *turn_id = Some(id.to_owned());
            }
            None
        }
        "item/agentMessage/delta" => {
            let text = agent_delta_text(params);
            if text.is_empty() {
                None
            } else {
                Some(AgentClientEvent::Message { text })
            }
        }
        "item/started" => {
            let item = params.get("item").unwrap_or(params);
            let kind = item.get("type").and_then(Value::as_str).unwrap_or("");
            match kind {
                "commandExecution" => Some(AgentClientEvent::Tool {
                    title: item
                        .get("command")
                        .and_then(Value::as_str)
                        .unwrap_or("Command")
                        .to_owned(),
                    status: "in_progress".into(),
                }),
                "fileChange" => Some(AgentClientEvent::Tool {
                    title: "Edit".into(),
                    status: "in_progress".into(),
                }),
                _ => None,
            }
        }
        "turn/completed" => Some(AgentClientEvent::Done {
            stop_reason: params
                .pointer("/turn/status")
                .and_then(Value::as_str)
                .unwrap_or("end_turn")
                .to_owned(),
        }),
        "error" => Some(AgentClientEvent::Error {
            message: params
                .pointer("/error/message")
                .and_then(Value::as_str)
                .or_else(|| params.get("message").and_then(Value::as_str))
                .unwrap_or("The agent returned an error.")
                .to_owned(),
        }),
        _ => None,
    }
}

fn agent_delta_text(params: &Value) -> String {
    if let Some(text) = params.get("delta").and_then(Value::as_str) {
        return text.to_owned();
    }
    if let Some(text) = params.pointer("/delta/text").and_then(Value::as_str) {
        return text.to_owned();
    }
    params
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
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

fn rpc_request_for(id: u64, method: &str, params: Value, wire: AgentWire) -> String {
    match wire {
        AgentWire::Acp => rpc_request(id, method, params),
        AgentWire::CodexApp => json!({ "id": id, "method": method, "params": params }).to_string(),
    }
}

fn rpc_notice_for(method: &str, params: Value, wire: AgentWire) -> String {
    match wire {
        AgentWire::Acp => rpc_notice(method, params),
        AgentWire::CodexApp => json!({ "method": method, "params": params }).to_string(),
    }
}

fn rpc_result_for(id: u64, result: Value, wire: AgentWire) -> String {
    match wire {
        AgentWire::Acp => rpc_result(id, result),
        AgentWire::CodexApp => json!({ "id": id, "result": result }).to_string(),
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
    on_exit: &dyn Fn() -> String,
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
                    message: agent_exit_message(&on_exit()),
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
                    message: agent_exit_message(&on_exit()),
                });
            }
        }
    }
}

fn collect_stderr(buf: &Mutex<String>, done: &AtomicBool) -> String {
    for _ in 0..20 {
        if done.load(Ordering::SeqCst) {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    buf.lock()
        .unwrap_or_else(|error| error.into_inner())
        .trim()
        .to_owned()
}

fn agent_exit_message(stderr: &str) -> String {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("not a terminal") || lower.contains("term is set") {
        return "This command started a terminal UI, not an ACP server. Codex CLI no longer has an `acp` subcommand — install `codex-acp`, or use OpenCode or Claude.".into();
    }
    let detail: String = stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let detail: String = detail.chars().take(240).collect();
    if detail.is_empty() {
        "The agent stopped before it was ready.".into()
    } else {
        format!("The agent stopped before it was ready. {detail}")
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
        AgentWire, Incoming, agent_delta_text, agent_exit_message, codex_approval_policy,
        codex_sandbox, content_text, is_codex_approval, parse_codex_event, parse_models,
        parse_modes, parse_permission_options, parse_update, pick_option, resolve_agent_command,
        rpc_id, rpc_notice, rpc_notice_for, rpc_request, rpc_request_for, rpc_result,
        rpc_result_for, wait_result, write_rpc,
    };
    use ps_core::agents::{
        AgentChoice, AgentPermission, AgentPreset, AgentServer, PermissionOutcome,
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

    #[test]
    fn rewrites_codex_acp_to_the_app_server() {
        let server = AgentServer {
            id: "01TEST".into(),
            name: "Codex".into(),
            preset: AgentPreset::Codex,
            command: "codex".into(),
            args: vec!["acp".into()],
            env: vec![],
            enabled: true,
        };
        let (command, args, wire) = resolve_agent_command(&server);
        assert_eq!(command, "codex");
        assert_eq!(args, vec!["app-server", "--stdio"]);
        assert_eq!(wire, AgentWire::CodexApp);
    }

    #[test]
    fn codex_thread_start_uses_kebab_case_wire_values() {
        assert_eq!(codex_sandbox(AgentPermission::Full), "workspace-write");
        assert_eq!(codex_sandbox(AgentPermission::Plan), "read-only");
        assert_eq!(codex_approval_policy(AgentPermission::Full), "never");
        assert_eq!(
            codex_approval_policy(AgentPermission::Allowance),
            "on-request"
        );
        assert_eq!(
            json!({
                "approvalPolicy": codex_approval_policy(AgentPermission::Full),
                "sandbox": codex_sandbox(AgentPermission::Full)
            }),
            json!({ "approvalPolicy": "never", "sandbox": "workspace-write" })
        );
    }

    #[test]
    fn streams_codex_app_server_message_deltas() {
        let mut turn_id = None;
        let event = parse_codex_event(
            &json!({
                "method": "item/agentMessage/delta",
                "params": { "delta": "Hello" }
            }),
            &mut turn_id,
        )
        .expect("delta");
        match event {
            ps_core::agents::AgentClientEvent::Message { text } => {
                assert_eq!(text, "Hello");
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(
            agent_delta_text(&json!({ "delta": { "text": "Hi" } })),
            "Hi"
        );
        let done = parse_codex_event(
            &json!({
                "method": "turn/completed",
                "params": { "turn": { "id": "t1", "status": "completed" } }
            }),
            &mut turn_id,
        )
        .expect("done");
        match done {
            ps_core::agents::AgentClientEvent::Done { stop_reason } => {
                assert_eq!(stop_reason, "completed");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn wait_result_explains_a_tui_exit() {
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(Incoming::Eof).expect("eof");
        let mut pending = HashMap::new();
        let error = wait_result(&rx, &mut pending, Duration::from_secs(1), &|| {
            "Error: stdin is not a terminal".into()
        })
        .expect_err("exited");
        assert!(error.to_string().contains("ACP server"));
    }

    #[test]
    fn agent_exit_message_keeps_a_short_stderr_tail() {
        assert_eq!(
            agent_exit_message(""),
            "The agent stopped before it was ready."
        );
        assert_eq!(
            agent_exit_message("boom"),
            "The agent stopped before it was ready. boom"
        );
    }

    #[test]
    fn resolve_agent_command_covers_acp_and_codex_shapes() {
        let custom = AgentServer {
            id: "01A".into(),
            name: "OpenCode".into(),
            preset: AgentPreset::Opencode,
            command: "opencode".into(),
            args: vec!["acp".into()],
            env: vec![],
            enabled: true,
        };
        let (command, args, wire) = resolve_agent_command(&custom);
        assert_eq!(command, "opencode");
        assert_eq!(args, vec!["acp"]);
        assert_eq!(wire, AgentWire::Acp);

        let adapter = AgentServer {
            id: "01B".into(),
            name: "Codex ACP".into(),
            preset: AgentPreset::Custom,
            command: "/opt/codex-acp".into(),
            args: vec![],
            env: vec![],
            enabled: true,
        };
        let (command, args, wire) = resolve_agent_command(&adapter);
        assert_eq!(command, "/opt/codex-acp");
        assert!(args.is_empty());
        assert_eq!(wire, AgentWire::Acp);

        let already = AgentServer {
            id: "01C".into(),
            name: "Codex".into(),
            preset: AgentPreset::Custom,
            command: "codex".into(),
            args: vec!["app-server".into()],
            env: vec![],
            enabled: true,
        };
        let (_, args, wire) = resolve_agent_command(&already);
        assert_eq!(args, vec!["app-server", "--stdio"]);
        assert_eq!(wire, AgentWire::CodexApp);

        let empty_preset = AgentServer {
            id: "01D".into(),
            name: "Codex".into(),
            preset: AgentPreset::Codex,
            command: "codex".into(),
            args: vec![],
            env: vec![],
            enabled: true,
        };
        let (_, args, wire) = resolve_agent_command(&empty_preset);
        assert_eq!(args, vec!["app-server", "--stdio"]);
        assert_eq!(wire, AgentWire::CodexApp);
    }

    #[test]
    fn parses_codex_events_tools_errors_and_empty_deltas() {
        let mut turn_id = None;
        assert!(
            parse_codex_event(
                &json!({
                    "method": "turn/started",
                    "params": { "turn": { "id": "turn-1" } }
                }),
                &mut turn_id
            )
            .is_none()
        );
        assert_eq!(turn_id.as_deref(), Some("turn-1"));
        assert!(
            parse_codex_event(
                &json!({
                    "method": "item/agentMessage/delta",
                    "params": { "delta": "" }
                }),
                &mut turn_id
            )
            .is_none()
        );
        match parse_codex_event(
            &json!({
                "method": "item/started",
                "params": { "item": { "type": "commandExecution", "command": "ls" } }
            }),
            &mut turn_id,
        )
        .expect("command")
        {
            ps_core::agents::AgentClientEvent::Tool { title, status } => {
                assert_eq!(title, "ls");
                assert_eq!(status, "in_progress");
            }
            other => panic!("unexpected {other:?}"),
        }
        match parse_codex_event(
            &json!({
                "method": "item/started",
                "params": { "item": { "type": "fileChange" } }
            }),
            &mut turn_id,
        )
        .expect("edit")
        {
            ps_core::agents::AgentClientEvent::Tool { title, .. } => {
                assert_eq!(title, "Edit");
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(
            parse_codex_event(
                &json!({
                    "method": "item/started",
                    "params": { "item": { "type": "other" } }
                }),
                &mut turn_id
            )
            .is_none()
        );
        match parse_codex_event(
            &json!({
                "method": "error",
                "params": { "error": { "message": "quota" } }
            }),
            &mut turn_id,
        )
        .expect("error")
        {
            ps_core::agents::AgentClientEvent::Error { message } => {
                assert_eq!(message, "quota");
            }
            other => panic!("unexpected {other:?}"),
        }
        match parse_codex_event(
            &json!({ "method": "error", "params": { "message": "plain" } }),
            &mut turn_id,
        )
        .expect("plain error")
        {
            ps_core::agents::AgentClientEvent::Error { message } => {
                assert_eq!(message, "plain");
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(parse_codex_event(&json!({ "method": "noop" }), &mut turn_id).is_none());
        assert_eq!(agent_delta_text(&json!({ "text": "tail" })), "tail");
        assert!(is_codex_approval("item/fileChange/requestApproval"));
        assert!(!is_codex_approval("session/update"));
    }

    #[test]
    fn maps_session_updates_models_permissions_and_wire_payloads() {
        match parse_update(&json!({
            "update": {
                "sessionUpdate": "tool_call",
                "title": "Read",
                "status": "completed"
            }
        }))
        .expect("tool")
        {
            ps_core::agents::AgentClientEvent::Tool { title, status } => {
                assert_eq!(title, "Read");
                assert_eq!(status, "completed");
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(
            parse_update(&json!({
                "update": {
                    "sessionUpdate": "agent_thought_chunk",
                    "content": { "type": "text", "text": "" }
                }
            }))
            .is_none()
        );
        assert_eq!(
            content_text(&json!([{ "text": "a" }, { "text": "b" }])),
            "ab"
        );
        assert_eq!(codex_sandbox(AgentPermission::Allowance), "workspace-write");
        assert_eq!(codex_approval_policy(AgentPermission::Plan), "on-request");

        let models = parse_models(&json!({
            "configOptions": [{
                "category": "model",
                "options": [{ "id": "gpt", "name": "GPT" }]
            }]
        }));
        assert_eq!(models[0].id, "gpt");

        let options = parse_permission_options(&json!({
            "options": [
                { "optionId": "allow_once", "name": "Allow once" },
                { "id": "reject_once", "name": "Reject" }
            ]
        }));
        assert_eq!(
            pick_option(&options, PermissionOutcome::AllowOnce),
            "allow_once"
        );
        assert_eq!(
            pick_option(&options, PermissionOutcome::RejectOnce),
            "reject_once"
        );
        assert_eq!(
            pick_option(&options, PermissionOutcome::AllowAlways),
            "allow_once"
        );
        assert_eq!(pick_option(&[], PermissionOutcome::Prompt), "allow_once");

        let named = [AgentChoice {
            id: "allow-always".into(),
            name: "Always".into(),
        }];
        assert_eq!(
            pick_option(&named, PermissionOutcome::AllowAlways),
            "allow-always"
        );

        assert_eq!(rpc_id(&json!({ "id": "7" })), Some(7));
        let request = rpc_request_for(3, "turn/start", json!({ "x": 1 }), AgentWire::CodexApp);
        assert!(!request.contains("jsonrpc"));
        assert!(rpc_notice("initialized", json!({})).contains("jsonrpc"));
        assert!(
            rpc_notice_for("initialized", json!({}), AgentWire::CodexApp).contains("initialized")
        );
        assert!(rpc_result(4, json!({ "ok": true })).contains("jsonrpc"));
        assert!(!rpc_result_for(5, json!("accept"), AgentWire::CodexApp).contains("jsonrpc"));
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
        wait_result(&rx, &mut pending, Duration::from_secs(5), &|| String::new())
            .expect("initialized");
        write_rpc(
            &mut stdin,
            &rpc_request(2, "session/new", json!({ "cwd": "/tmp", "mcpServers": [] })),
        )
        .expect("session");
        let created = wait_result(&rx, &mut pending, Duration::from_secs(5), &|| String::new())
            .expect("created");
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
