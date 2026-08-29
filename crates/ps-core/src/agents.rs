//! External ACP agent configuration, PATH presets, and local prompt history.

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use ts_rs::TS;
use ulid::Ulid;

use crate::store::{JsonStore, VersionedDocument};
use crate::{Error, Result};

/// Maximum stored user prompts.
pub const MAX_PROMPT_HISTORY: usize = 50;
/// Longest user prompt kept in history.
pub const MAX_PROMPT_CHARS: usize = 8_000;

/// How an agent may use tools in the open project.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum AgentPermission {
    /// Auto-allow tool calls. The agent process can still change files in `cwd`.
    Full,
    /// Prefer a plan/ask mode; reject write-like tools when the agent asks.
    Plan,
    /// Ask before each tool call.
    #[default]
    Allowance,
}

/// Known ACP command presets. Custom agents use [`AgentPreset::Custom`].
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum AgentPreset {
    /// OpenCode (`opencode acp`).
    Opencode,
    /// Claude Code (`claude --acp`).
    Claude,
    /// Codex (`codex app-server`, or the `codex-acp` adapter).
    Codex,
    /// User-supplied command and arguments.
    #[default]
    Custom,
}

/// One environment variable passed to an agent process.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, TS)]
pub struct AgentEnvVar {
    /// Environment variable name.
    pub name: String,
    /// Environment variable value. Never written to application logs.
    pub value: String,
}

/// A configured ACP agent command.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
pub struct AgentServer {
    /// Stable ULID.
    pub id: String,
    /// User-visible name.
    pub name: String,
    /// Built-in preset, or custom.
    pub preset: AgentPreset,
    /// Program name on `PATH`, or an absolute path.
    pub command: String,
    /// Arguments passed without a shell.
    pub args: Vec<String>,
    /// Extra environment variables for the process.
    #[serde(default)]
    pub env: Vec<AgentEnvVar>,
    /// Whether the Assistant may start this agent.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl AgentServer {
    /// Builds a custom agent with a fresh identifier.
    pub fn custom(name: impl Into<String>, command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            id: Ulid::generate().to_string(),
            name: name.into(),
            preset: AgentPreset::Custom,
            command: command.into(),
            args,
            env: Vec::new(),
            enabled: true,
        }
    }

    /// Builds a preset agent with the default command and arguments.
    pub fn from_preset(preset: AgentPreset) -> Self {
        let (command, args) = preset.binaries().first().copied().unwrap_or(("", &[]));
        Self {
            id: Ulid::generate().to_string(),
            name: preset.display_name().to_owned(),
            preset,
            command: command.to_owned(),
            args: args.iter().map(|arg| (*arg).to_owned()).collect(),
            env: Vec::new(),
            enabled: true,
        }
    }
}

/// External-agent settings stored in `config.json`.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(default)]
pub struct Agents {
    /// Configured ACP servers.
    pub servers: Vec<AgentServer>,
    /// Last selected server, when it still exists.
    pub default_server_id: Option<String>,
    /// Default tool-permission policy for new chats.
    pub permission: AgentPermission,
}

impl Agents {
    /// Validates server records, commands, and the default selection.
    pub fn validate(&self) -> Result<()> {
        let mut seen = std::collections::HashSet::new();
        for server in &self.servers {
            if !seen.insert(server.id.as_str()) {
                return Err(Error::InvalidConfigFormat {
                    field: "agents.servers.id",
                    expected: "unique ULID",
                });
            }
            if Ulid::from_string(&server.id).is_err() {
                return Err(Error::InvalidConfigFormat {
                    field: "agents.servers.id",
                    expected: "ULID",
                });
            }
            if server.name.trim().is_empty() {
                return Err(Error::InvalidConfigFormat {
                    field: "agents.servers.name",
                    expected: "non-empty name",
                });
            }
            validate_command(&server.command)?;
            for arg in &server.args {
                validate_arg(arg)?;
            }
            for variable in &server.env {
                validate_env_name(&variable.name)?;
            }
        }
        if let Some(id) = &self.default_server_id
            && !self.servers.iter().any(|server| server.id == *id)
        {
            return Err(Error::InvalidConfigFormat {
                field: "agents.default_server_id",
                expected: "identifier of a configured agent",
            });
        }
        Ok(())
    }
}

/// PATH probe result for a built-in preset.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
pub struct AgentPresetInfo {
    /// Preset identifier.
    pub preset: AgentPreset,
    /// User-visible name.
    pub name: String,
    /// Command to store if the user adds this preset.
    pub command: String,
    /// Arguments to store if the user adds this preset.
    pub args: Vec<String>,
    /// Whether the command exists on `PATH`.
    pub available: bool,
}

/// A model or permission-mode choice advertised by an agent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
pub struct AgentChoice {
    /// Agent-defined identifier.
    pub id: String,
    /// User-visible label.
    pub name: String,
}

/// Event streamed from an ACP session to the UI.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentClientEvent {
    /// Session is ready for prompts.
    Ready {
        /// ACP session identifier.
        session_id: String,
        /// Models advertised by the agent, if any.
        models: Vec<AgentChoice>,
        /// Session modes advertised by the agent, if any.
        modes: Vec<AgentChoice>,
    },
    /// Streamed assistant text.
    Message {
        /// Markdown or plain text chunk.
        text: String,
    },
    /// Tool call progress without file bodies.
    Tool {
        /// Short tool title from the agent.
        title: String,
        /// pending, in_progress, completed, or failed.
        status: String,
    },
    /// The agent is waiting for a permission decision.
    Permission {
        /// JSON-RPC request id to answer.
        id: u64,
        /// Tool title shown in the dialog.
        title: String,
        /// Options supplied by the agent.
        options: Vec<AgentChoice>,
    },
    /// The prompt turn finished.
    Done {
        /// ACP stop reason.
        stop_reason: String,
    },
    /// The session failed. Must not include document text.
    Error {
        /// User-visible explanation.
        message: String,
    },
}

/// How the client should answer `session/request_permission`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PermissionOutcome {
    /// Allow this and later tools in the turn.
    AllowAlways,
    /// Allow this tool once.
    AllowOnce,
    /// Reject this tool once.
    RejectOnce,
    /// Ask the user in the Assistant tab.
    Prompt,
}

/// Live or last ACP session counters for the Dashboard. No prompt text.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionStats {
    /// Whether a subprocess is running.
    pub live: bool,
    /// Configured agent name, empty when no session has started.
    pub agent_name: String,
    /// Short status such as Ready, Streaming, or Stopped.
    pub status: String,
    /// User prompts sent in this session.
    pub prompts: u32,
    /// Tool-call updates received in this session.
    pub tools: u32,
    /// Permission prompts shown to the user.
    pub permission_asks: u32,
}

impl AgentSessionStats {
    /// Idle session for a named agent.
    pub fn started(agent_name: impl Into<String>) -> Self {
        Self {
            live: true,
            agent_name: agent_name.into(),
            status: "Ready".into(),
            prompts: 0,
            tools: 0,
            permission_asks: 0,
        }
    }
}

/// One stored user prompt. Agent replies are not saved here.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
pub struct PromptHistoryEntry {
    /// Stable ULID.
    pub id: String,
    /// RFC 3339 timestamp.
    pub ts: String,
    /// Prompt text shown in history.
    pub text: String,
    /// Agent that received the prompt.
    pub server_id: String,
}

/// On-disk prompt history under `~/.1537paperstreet/agents/`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PromptHistory {
    /// Storage schema version.
    pub schema_version: u32,
    /// Newest prompts first.
    #[serde(default)]
    pub entries: Vec<PromptHistoryEntry>,
}

impl Default for PromptHistory {
    fn default() -> Self {
        Self {
            schema_version: <Self as VersionedDocument>::SCHEMA_VERSION,
            entries: Vec::new(),
        }
    }
}

impl VersionedDocument for PromptHistory {
    const SCHEMA_VERSION: u32 = 1;

    fn migrate(value: Value, from: u32) -> Result<Value> {
        let _ = value;
        Err(Error::UnsupportedSchema {
            found: from,
            supported: Self::SCHEMA_VERSION,
        })
    }

    fn validate(&self) -> Result<()> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(Error::UnsupportedSchema {
                found: self.schema_version,
                supported: Self::SCHEMA_VERSION,
            });
        }
        Ok(())
    }
}

impl PromptHistory {
    /// Inserts a prompt at the front and drops entries beyond [`MAX_PROMPT_HISTORY`].
    pub fn push(&mut self, server_id: String, text: &str) -> Result<PromptHistoryEntry> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(Error::InvalidConfigFormat {
                field: "prompt",
                expected: "non-empty prompt",
            });
        }
        let text = if trimmed.chars().count() > MAX_PROMPT_CHARS {
            trimmed.chars().take(MAX_PROMPT_CHARS).collect()
        } else {
            trimmed.to_owned()
        };
        let entry = PromptHistoryEntry {
            id: Ulid::generate().to_string(),
            ts: OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .map_err(|source| Error::TimeFormat { source })?,
            text,
            server_id,
        };
        self.entries.insert(0, entry.clone());
        self.entries.truncate(MAX_PROMPT_HISTORY);
        Ok(entry)
    }

    /// Removes one stored prompt. Missing ids are an error, not a no-op.
    pub fn remove(&mut self, id: &str) -> Result<PromptHistoryEntry> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.id == id)
            .ok_or_else(|| Error::Agent {
                message: "That prompt is no longer in history.".into(),
            })?;
        Ok(self.entries.remove(index))
    }

    /// Drops every stored prompt.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Opens or creates the prompt-history store.
pub fn open_prompt_history(path: impl AsRef<Path>) -> Result<JsonStore<PromptHistory>> {
    JsonStore::open(path.as_ref())
}

impl AgentPreset {
    /// User-visible name for Settings.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Opencode => "OpenCode",
            Self::Claude => "Claude",
            Self::Codex => "Codex",
            Self::Custom => "Custom",
        }
    }

    /// Candidate binaries and their ACP arguments, first match wins.
    pub fn binaries(self) -> &'static [(&'static str, &'static [&'static str])] {
        match self {
            Self::Opencode => &[("opencode", &["acp"])],
            Self::Claude => &[("claude", &["--acp"])],
            Self::Codex => &[("codex", &["app-server", "--stdio"]), ("codex-acp", &[])],
            Self::Custom => &[],
        }
    }
}

/// Named presets in Settings order.
pub fn named_presets() -> &'static [AgentPreset] {
    &[
        AgentPreset::Opencode,
        AgentPreset::Claude,
        AgentPreset::Codex,
    ]
}

/// Probes `PATH` (or `path`) for built-in ACP commands. Does not download anything.
///
/// When `path` is omitted, common login-shell directories are prepended so a
/// GUI launch still finds Homebrew and `~/.local/bin` agents.
pub fn detect_presets(path: Option<&OsStr>) -> Vec<AgentPresetInfo> {
    let search = match path {
        Some(path) => path.to_os_string(),
        None => login_path(
            env::var_os("PATH").as_deref(),
            env::var_os("HOME").as_deref().map(Path::new),
        ),
    };
    let directories = split_path(&search);

    named_presets()
        .iter()
        .copied()
        .map(|preset| {
            let found = preset.binaries().iter().find_map(|(binary, args)| {
                directories
                    .iter()
                    .find(|dir| is_runnable(&dir.join(binary)))
                    .map(|_| (*binary, *args))
            });
            let (command, args, available) = match found {
                Some((command, args)) => (
                    command.to_owned(),
                    args.iter().map(|arg| (*arg).to_owned()).collect(),
                    true,
                ),
                None => {
                    let (command, args) = preset.binaries().first().copied().unwrap_or(("", &[]));
                    (
                        command.to_owned(),
                        args.iter().map(|arg| (*arg).to_owned()).collect(),
                        false,
                    )
                }
            };
            AgentPresetInfo {
                preset,
                name: preset.display_name().to_owned(),
                command,
                args,
                available,
            }
        })
        .collect()
}

/// Chooses a plan-like ACP session mode when the agent advertises one.
pub fn preferred_plan_mode(modes: &[AgentChoice]) -> Option<&str> {
    const KEYWORDS: &[&str] = &["plan", "architect", "ask"];
    modes.iter().find_map(|mode| {
        let haystack = format!("{} {}", mode.id, mode.name).to_ascii_lowercase();
        KEYWORDS
            .iter()
            .any(|keyword| haystack.contains(keyword))
            .then_some(mode.id.as_str())
    })
}

/// Decides how to answer a tool permission request without prompting.
pub fn permission_outcome(
    permission: AgentPermission,
    tool_kind: Option<&str>,
) -> PermissionOutcome {
    match permission {
        AgentPermission::Full => PermissionOutcome::AllowAlways,
        AgentPermission::Allowance => PermissionOutcome::Prompt,
        AgentPermission::Plan => match tool_kind.map(str::to_ascii_lowercase).as_deref() {
            Some("edit" | "delete" | "move" | "execute" | "terminal") => {
                PermissionOutcome::RejectOnce
            }
            Some("read" | "search" | "think" | "switch_mode") => PermissionOutcome::AllowOnce,
            _ => PermissionOutcome::Prompt,
        },
    }
}

fn split_path(path: &OsStr) -> Vec<PathBuf> {
    env::split_paths(path)
        .filter(|dir| !dir.as_os_str().is_empty())
        .collect()
}

/// Prepends common user bin directories so GUI launches still find local agents.
pub fn login_path(existing: Option<&OsStr>, home: Option<&Path>) -> OsString {
    let mut dirs = Vec::new();
    if let Some(home) = home {
        dirs.push(home.join(".local/bin"));
        dirs.push(home.join(".cargo/bin"));
    }
    dirs.push(PathBuf::from("/opt/homebrew/bin"));
    dirs.push(PathBuf::from("/usr/local/bin"));
    if let Some(existing) = existing {
        for dir in env::split_paths(existing) {
            if !dir.as_os_str().is_empty() && !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
    }
    env::join_paths(&dirs).unwrap_or_else(|_| existing.map(OsStr::to_os_string).unwrap_or_default())
}

fn is_runnable(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
}

fn validate_command(command: &str) -> Result<()> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidConfigFormat {
            field: "agents.command",
            expected: "program name or absolute path",
        });
    }
    if trimmed.chars().any(is_shell_meta) {
        return Err(Error::InvalidConfigFormat {
            field: "agents.command",
            expected: "program name or absolute path without shell characters",
        });
    }
    let path = Path::new(trimmed);
    if path.components().count() > 1 && !path.is_absolute() {
        return Err(Error::InvalidConfigFormat {
            field: "agents.command",
            expected: "program name or absolute path",
        });
    }
    Ok(())
}

fn validate_arg(arg: &str) -> Result<()> {
    if arg.chars().any(|ch| matches!(ch, '\n' | '\r' | '\0')) {
        return Err(Error::InvalidConfigFormat {
            field: "agents.args",
            expected: "argument without line breaks",
        });
    }
    Ok(())
}

fn validate_env_name(name: &str) -> Result<()> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(Error::InvalidConfigFormat {
            field: "agents.env.name",
            expected: "environment variable name",
        });
    };
    if !(first.is_ascii_alphabetic() || first == '_')
        || !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return Err(Error::InvalidConfigFormat {
            field: "agents.env.name",
            expected: "environment variable name",
        });
    }
    Ok(())
}

fn is_shell_meta(ch: char) -> bool {
    matches!(
        ch,
        '\n' | '\r' | '\0' | ';' | '|' | '&' | '`' | '$' | '<' | '>' | '(' | ')' | '\'' | '"'
    )
}
