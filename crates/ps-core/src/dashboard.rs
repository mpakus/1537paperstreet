//! Local library and agent analytics for the Dashboard tab.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::agents::{
    AgentPermission, AgentPreset, AgentServer, AgentSessionStats, Agents, PromptHistoryEntry,
};
use crate::projects::Project;
use crate::tree::MarkdownCount;

/// A labeled number or short status on the Dashboard.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
pub struct DashboardMetric {
    /// User-visible label.
    pub label: String,
    /// Already formatted for display.
    pub value: String,
}

/// One row in a Dashboard list.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
pub struct DashboardRow {
    /// Primary text.
    pub title: String,
    /// Secondary text.
    pub detail: String,
}

/// One titled block on the Dashboard.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
pub struct DashboardSection {
    /// Section heading.
    pub title: String,
    /// KPI cards.
    pub metrics: Vec<DashboardMetric>,
    /// Detail rows under the cards.
    pub rows: Vec<DashboardRow>,
}

/// Preformatted Dashboard payload. The UI only renders these strings.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
pub struct DashboardSnapshot {
    /// Page heading.
    pub title: String,
    /// One-line explanation.
    pub lede: String,
    /// Ordered sections.
    pub sections: Vec<DashboardSection>,
}

/// Builds the Dashboard from registry, config, history, and session counters.
pub fn snapshot(
    projects: &[Project],
    agents: &Agents,
    history: &[PromptHistoryEntry],
    active: Option<&Project>,
    markdown: MarkdownCount,
    session: &AgentSessionStats,
) -> DashboardSnapshot {
    DashboardSnapshot {
        title: "Dashboard".into(),
        lede: "Local counts only. Nothing here is sent off this Mac.".into(),
        sections: vec![
            library_section(projects),
            project_section(active, markdown),
            agents_section(agents, history),
            session_section(session),
            recent_prompts_section(history, &agents.servers),
        ],
    }
}

fn library_section(projects: &[Project]) -> DashboardSection {
    let pinned = projects.iter().filter(|project| project.pinned).count();
    let unavailable = projects
        .iter()
        .filter(|project| project.available == Some(false))
        .count();
    let mut rows: Vec<DashboardRow> = projects
        .iter()
        .take(12)
        .map(|project| DashboardRow {
            title: project.name.clone(),
            detail: project
                .last_opened_at
                .as_deref()
                .map(short_stamp)
                .unwrap_or_else(|| "Not opened yet".into()),
        })
        .collect();
    if projects.len() > 12 {
        rows.push(DashboardRow {
            title: format!("{} more", projects.len() - 12),
            detail: "Open a project from the list".into(),
        });
    }
    DashboardSection {
        title: "Library".into(),
        metrics: vec![
            metric("Projects", count(projects.len())),
            metric("Pinned", count(pinned)),
            metric("Unavailable", count(unavailable)),
        ],
        rows,
    }
}

fn project_section(active: Option<&Project>, markdown: MarkdownCount) -> DashboardSection {
    let (name, path) = match active {
        Some(project) => (
            project.name.clone(),
            project.path.to_string_lossy().into_owned(),
        ),
        None => (
            "None".into(),
            "Open a folder to count Markdown files".into(),
        ),
    };
    let files = if markdown.capped {
        format!("{}+", markdown.files)
    } else {
        count(markdown.files as usize)
    };
    DashboardSection {
        title: "Open project".into(),
        metrics: vec![metric("Folder", name), metric("Markdown files", files)],
        rows: vec![DashboardRow {
            title: "Path".into(),
            detail: path,
        }],
    }
}

fn agents_section(agents: &Agents, history: &[PromptHistoryEntry]) -> DashboardSection {
    let enabled = agents
        .servers
        .iter()
        .filter(|server| server.enabled)
        .count();
    let default_name = agents
        .default_server_id
        .as_ref()
        .and_then(|id| agents.servers.iter().find(|server| server.id == *id))
        .map(|server| server.name.as_str())
        .unwrap_or("None");
    let rows = agents
        .servers
        .iter()
        .map(|server| DashboardRow {
            title: server.name.clone(),
            detail: agent_detail(server),
        })
        .collect();
    DashboardSection {
        title: "Agents".into(),
        metrics: vec![
            metric("Configured", count(agents.servers.len())),
            metric("Enabled", count(enabled)),
            metric("Permission", permission_label(agents.permission).into()),
            metric("Stored prompts", count(history.len())),
            metric("Default agent", default_name.into()),
        ],
        rows,
    }
}

fn session_section(session: &AgentSessionStats) -> DashboardSection {
    let agent = if session.agent_name.is_empty() {
        "None"
    } else {
        session.agent_name.as_str()
    };
    let status = if session.status.is_empty() {
        "No session"
    } else {
        session.status.as_str()
    };
    DashboardSection {
        title: "Live session".into(),
        metrics: vec![
            metric("Agent", agent.into()),
            metric("Status", status.into()),
            metric("Running", if session.live { "Yes" } else { "No" }.into()),
            metric("Prompts", count(session.prompts as usize)),
            metric("Tool updates", count(session.tools as usize)),
            metric("Permission asks", count(session.permission_asks as usize)),
        ],
        rows: Vec::new(),
    }
}

fn recent_prompts_section(
    history: &[PromptHistoryEntry],
    servers: &[AgentServer],
) -> DashboardSection {
    let rows = history
        .iter()
        .take(12)
        .map(|entry| {
            let agent = servers
                .iter()
                .find(|server| server.id == entry.server_id)
                .map(|server| server.name.as_str())
                .unwrap_or("Removed agent");
            DashboardRow {
                title: truncate_prompt(&entry.text),
                detail: format!("{} · {}", agent, short_stamp(&entry.ts)),
            }
        })
        .collect();
    DashboardSection {
        title: "Recent prompts".into(),
        metrics: Vec::new(),
        rows,
    }
}

fn agent_detail(server: &AgentServer) -> String {
    let preset = match server.preset {
        AgentPreset::Opencode => "OpenCode",
        AgentPreset::Claude => "Claude",
        AgentPreset::Codex => "Codex",
        AgentPreset::Custom => "Custom",
    };
    let state = if server.enabled { "on" } else { "off" };
    format!("{preset} · {state}")
}

fn permission_label(permission: AgentPermission) -> &'static str {
    match permission {
        AgentPermission::Full => "Full",
        AgentPermission::Plan => "Plan",
        AgentPermission::Allowance => "Allowance",
    }
}

fn metric(label: &str, value: String) -> DashboardMetric {
    DashboardMetric {
        label: label.into(),
        value,
    }
}

fn count(value: usize) -> String {
    value.to_string()
}

fn short_stamp(ts: &str) -> String {
    ts.get(..16).unwrap_or(ts).replace('T', " ")
}

fn truncate_prompt(text: &str) -> String {
    const MAX: usize = 72;
    let mut chars = text.chars();
    let taken: String = chars.by_ref().take(MAX).collect();
    if chars.next().is_some() {
        format!("{taken}…")
    } else {
        taken
    }
}
