use ps_core::agents::{
    AgentPermission, AgentPreset, AgentServer, AgentSessionStats, Agents, PromptHistory,
};
use ps_core::dashboard;
use ps_core::projects::Project;
use ps_core::tree::{self, MarkdownCount};

#[test]
fn snapshot_formats_library_agents_and_session_without_ui_math() {
    let mut project = sample_project("Notes");
    project.pinned = true;
    project.last_opened_at = Some("2026-08-23T12:00:00Z".into());
    project.available = Some(true);

    let mut agents = Agents {
        permission: AgentPermission::Allowance,
        default_server_id: None,
        servers: vec![AgentServer::from_preset(AgentPreset::Opencode)],
    };
    agents.default_server_id = Some(agents.servers[0].id.clone());

    let mut history = PromptHistory::default();
    history
        .push(agents.servers[0].id.clone(), "Summarize FLOW.md")
        .expect("prompt");

    let session = AgentSessionStats {
        live: true,
        agent_name: "OpenCode".into(),
        status: "Streaming".into(),
        prompts: 2,
        tools: 4,
        permission_asks: 1,
    };

    let snap = dashboard::snapshot(
        &[project],
        &agents,
        &history.entries,
        None,
        MarkdownCount {
            files: 3,
            capped: false,
        },
        &session,
    );

    assert_eq!(snap.title, "Dashboard");
    let library = &snap.sections[0];
    assert_eq!(library.title, "Library");
    assert_eq!(library.metrics[0].value, "1");
    assert_eq!(library.metrics[1].value, "1");
    assert_eq!(library.rows[0].title, "Notes");
    assert_eq!(library.rows[0].detail, "2026-08-23 12:00");

    let agents_section = snap
        .sections
        .iter()
        .find(|section| section.title == "Agents")
        .expect("agents");
    assert_eq!(agents_section.metrics[0].value, "1");
    assert_eq!(agents_section.metrics[2].value, "Allowance");
    assert_eq!(agents_section.rows[0].detail, "OpenCode · on");

    let live = snap
        .sections
        .iter()
        .find(|section| section.title == "Live session")
        .expect("session");
    assert_eq!(live.metrics[3].value, "2");
    assert_eq!(live.metrics[4].value, "4");

    let prompts = snap
        .sections
        .iter()
        .find(|section| section.title == "Recent prompts")
        .expect("prompts");
    assert_eq!(prompts.rows[0].title, "Summarize FLOW.md");
}

#[test]
fn markdown_count_reports_files_in_a_temp_project() {
    let temp = tempfile::tempdir().expect("temp");
    let root = temp.path().join("project");
    std::fs::create_dir(&root).expect("project");
    std::fs::write(root.join("a.md"), b"# A").expect("a");
    std::fs::write(root.join("b.md"), b"# B").expect("b");
    std::fs::write(root.join("skip.txt"), b"no").expect("txt");
    let count = tree::count_markdown(&root, false).expect("count");
    assert_eq!(count.files, 2);
    assert!(!count.capped);
}

fn sample_project(name: &str) -> Project {
    Project {
        id: ulid::Ulid::generate().to_string(),
        name: name.into(),
        path: "/tmp/notes".into(),
        added_at: "2026-08-01T00:00:00Z".into(),
        last_opened_at: None,
        pinned: false,
        accent: None,
        last_file: None,
        available: None,
    }
}
