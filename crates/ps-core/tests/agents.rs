use std::fs;
use std::os::unix::fs::PermissionsExt;

use ps_core::Error;
use ps_core::agents::{
    AgentChoice, AgentEnvVar, AgentPermission, AgentPreset, AgentServer, Agents,
    MAX_PROMPT_HISTORY, PermissionOutcome, PromptHistory, detect_presets, login_path,
    permission_outcome, preferred_plan_mode,
};
use ps_core::config::Config;

#[test]
fn missing_agents_defaults_without_a_schema_bump() {
    let config: Config = serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "appearance": {
            "theme": "paper-light",
            "theme_dark": "paper-dark",
            "follow_system": true,
            "accent": "#C1452F"
        },
        "typography": {
            "body_font": "New York",
            "mono_font": "JetBrains Mono",
            "font_size": 16,
            "line_height": 1.65,
            "measure_ch": 72
        }
    }))
    .expect("config");
    assert!(config.agents.servers.is_empty());
    assert_eq!(config.agents.permission, AgentPermission::Allowance);
    assert!(config.agents.default_server_id.is_none());
    config.validate().expect("valid without agents");
}

#[test]
fn rejects_shell_metacharacters_in_agent_commands() {
    let mut config = Config::default();
    let mut server = AgentServer::custom("Bad", "opencode;rm", vec!["acp".into()]);
    config.agents.servers.push(server.clone());
    assert!(matches!(
        config.validate(),
        Err(Error::InvalidConfigFormat {
            field: "agents.command",
            ..
        })
    ));

    server.command = "opencode".into();
    config.agents.servers[0] = server;
    config.validate().expect("plain command");
}

#[test]
fn rejects_relative_command_paths_and_empty_env_names() {
    let mut agents = Agents::default();
    agents
        .servers
        .push(AgentServer::custom("Rel", "./opencode", vec!["acp".into()]));
    assert!(matches!(
        agents.validate(),
        Err(Error::InvalidConfigFormat {
            field: "agents.command",
            ..
        })
    ));

    let mut ok = AgentServer::from_preset(AgentPreset::Opencode);
    ok.env.push(AgentEnvVar {
        name: String::new(),
        value: "x".into(),
    });
    agents.servers = vec![ok];
    assert!(matches!(
        agents.validate(),
        Err(Error::InvalidConfigFormat {
            field: "agents.env.name",
            ..
        })
    ));
}

#[test]
fn default_server_must_refer_to_a_configured_agent() {
    let mut agents = Agents {
        default_server_id: Some(ulid::Ulid::generate().to_string()),
        ..Agents::default()
    };
    assert!(matches!(
        agents.validate(),
        Err(Error::InvalidConfigFormat {
            field: "agents.default_server_id",
            ..
        })
    ));

    let server = AgentServer::from_preset(AgentPreset::Claude);
    agents.default_server_id = Some(server.id.clone());
    agents.servers.push(server);
    agents.validate().expect("known default");
}

#[test]
fn detects_preset_binaries_on_an_injected_path() {
    let temp = tempfile::tempdir().expect("path dir");
    let opencode = temp.path().join("opencode");
    fs::write(&opencode, b"#!/bin/sh\n").expect("write");
    let mut permissions = fs::metadata(&opencode).expect("meta").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&opencode, permissions).expect("chmod");

    let found = detect_presets(Some(temp.path().as_os_str()));
    let open = found
        .iter()
        .find(|preset| preset.preset == AgentPreset::Opencode)
        .expect("opencode preset");
    assert!(open.available);
    assert_eq!(open.command, "opencode");
    assert_eq!(open.args, vec!["acp"]);

    let claude = found
        .iter()
        .find(|preset| preset.preset == AgentPreset::Claude)
        .expect("claude preset");
    assert!(!claude.available);
}

#[test]
fn prefers_native_codex_app_server_over_the_adapter() {
    let temp = tempfile::tempdir().expect("path dir");
    for name in ["codex", "codex-acp"] {
        let bin = temp.path().join(name);
        fs::write(&bin, b"#!/bin/sh\n").expect("write");
        let mut permissions = fs::metadata(&bin).expect("meta").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&bin, permissions).expect("chmod");
    }

    let found = detect_presets(Some(temp.path().as_os_str()));
    let codex = found
        .iter()
        .find(|preset| preset.preset == AgentPreset::Codex)
        .expect("codex preset");
    assert!(codex.available);
    assert_eq!(codex.command, "codex");
    assert_eq!(codex.args, vec!["app-server", "--stdio"]);
}

#[test]
fn login_path_puts_local_bin_ahead_of_a_narrow_path() {
    let home = tempfile::tempdir().expect("home");
    let joined = login_path(Some(std::ffi::OsStr::new("/usr/bin")), Some(home.path()));
    let text = joined.to_string_lossy();
    let local = home.path().join(".local/bin");
    let local_at = text
        .find(local.to_string_lossy().as_ref())
        .expect("local bin");
    let usr_at = text.find("/usr/bin").expect("usr bin");
    assert!(local_at < usr_at);
}

#[test]
fn plan_permission_rejects_write_tools_and_prefers_plan_modes() {
    assert_eq!(
        permission_outcome(AgentPermission::Full, Some("edit")),
        PermissionOutcome::AllowAlways
    );
    assert_eq!(
        permission_outcome(AgentPermission::Allowance, Some("edit")),
        PermissionOutcome::Prompt
    );
    assert_eq!(
        permission_outcome(AgentPermission::Plan, Some("edit")),
        PermissionOutcome::RejectOnce
    );
    assert_eq!(
        permission_outcome(AgentPermission::Plan, Some("read")),
        PermissionOutcome::AllowOnce
    );

    let modes = vec![
        AgentChoice {
            id: "code".into(),
            name: "Code".into(),
        },
        AgentChoice {
            id: "architect".into(),
            name: "Architect".into(),
        },
    ];
    assert_eq!(preferred_plan_mode(&modes), Some("architect"));
}

#[test]
fn prompt_history_keeps_the_newest_entries_and_drops_the_oldest() {
    let mut history = PromptHistory::default();
    for index in 0..(MAX_PROMPT_HISTORY + 3) {
        history
            .push("server".into(), &format!("prompt {index}"))
            .expect("push");
    }
    assert_eq!(history.entries.len(), MAX_PROMPT_HISTORY);
    assert_eq!(
        history.entries[0].text,
        format!("prompt {}", MAX_PROMPT_HISTORY + 2)
    );
    assert!(history.push("server".into(), "   ").is_err());
}

#[test]
fn prompt_history_removes_one_entry_and_clears_the_rest() {
    let mut history = PromptHistory::default();
    let first = history.push("server".into(), "first").expect("push");
    let second = history.push("server".into(), "second").expect("push");
    assert_eq!(history.remove(&first.id).expect("remove").text, "first");
    assert_eq!(
        history
            .entries
            .iter()
            .map(|entry| entry.text.as_str())
            .collect::<Vec<_>>(),
        vec!["second"]
    );
    assert!(history.remove("missing").is_err());
    history.clear();
    assert!(history.entries.is_empty());
    assert_eq!(
        history.remove(&second.id).unwrap_err().to_string(),
        "That prompt is no longer in history."
    );
}
