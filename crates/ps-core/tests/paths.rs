use std::sync::Mutex;

use ps_core::paths::AppPaths;

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn ensure_creates_the_application_directory_tree() {
    let temp = tempfile::tempdir().expect("temporary directory");
    let paths = AppPaths::from_root(temp.path().join("paperstreet"));

    paths.ensure().expect("application directories");

    assert!(paths.root().is_dir());
    assert!(paths.themes().is_dir());
    assert!(paths.cache().is_dir());
    assert!(paths.logs().is_dir());
    assert_eq!(paths.config_file(), paths.root().join("config.json"));
    assert_eq!(paths.projects_file(), paths.root().join("projects.json"));
    assert_eq!(paths.ui_state_file(), paths.root().join("ui-state.json"));
    assert!(paths.mermaid_cache().is_dir());
    assert_eq!(paths.mermaid_cache(), paths.root().join("cache/mermaid"));
    assert_eq!(paths.log_file(), paths.root().join("logs/app.log"));
    assert!(paths.agents().is_dir());
    assert_eq!(
        paths.agent_prompts_file(),
        paths.root().join("agents/prompts.json")
    );
    assert_eq!(
        paths.instance_lock_file(),
        paths.root().join("instance.lock")
    );
}

#[test]
fn discover_reads_the_override_environment_variable() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let temp = tempfile::tempdir().expect("temporary directory");
    let previous = std::env::var_os(ps_core::paths::ROOT_OVERRIDE_ENV);
    // SAFETY: this test process is the only writer of PAPERSTREET_HOME here.
    unsafe {
        std::env::set_var(ps_core::paths::ROOT_OVERRIDE_ENV, temp.path());
    }
    let discovered = AppPaths::discover().expect("discover");
    match previous {
        Some(value) => unsafe { std::env::set_var(ps_core::paths::ROOT_OVERRIDE_ENV, value) },
        None => unsafe { std::env::remove_var(ps_core::paths::ROOT_OVERRIDE_ENV) },
    }
    assert_eq!(discovered.root(), temp.path());
}

#[test]
fn discover_falls_back_to_home_when_the_override_is_empty() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let previous_override = std::env::var_os(ps_core::paths::ROOT_OVERRIDE_ENV);
    let previous_home = std::env::var_os("HOME");
    let home = tempfile::tempdir().expect("home directory");
    unsafe {
        std::env::set_var(ps_core::paths::ROOT_OVERRIDE_ENV, "");
        std::env::set_var("HOME", home.path());
    }
    let discovered = AppPaths::discover().expect("discover from HOME");
    match previous_override {
        Some(value) => unsafe { std::env::set_var(ps_core::paths::ROOT_OVERRIDE_ENV, value) },
        None => unsafe { std::env::remove_var(ps_core::paths::ROOT_OVERRIDE_ENV) },
    }
    match previous_home {
        Some(value) => unsafe { std::env::set_var("HOME", value) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    assert_eq!(
        discovered.root(),
        home.path().join(".1537paperstreet").as_path()
    );
}

#[test]
fn discover_requires_a_home_directory_without_an_override() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let previous_override = std::env::var_os(ps_core::paths::ROOT_OVERRIDE_ENV);
    let previous_home = std::env::var_os("HOME");
    unsafe {
        std::env::remove_var(ps_core::paths::ROOT_OVERRIDE_ENV);
        std::env::remove_var("HOME");
    }
    let result = AppPaths::discover();
    match previous_override {
        Some(value) => unsafe { std::env::set_var(ps_core::paths::ROOT_OVERRIDE_ENV, value) },
        None => unsafe { std::env::remove_var(ps_core::paths::ROOT_OVERRIDE_ENV) },
    }
    match previous_home {
        Some(value) => unsafe { std::env::set_var("HOME", value) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    assert!(matches!(
        result,
        Err(ps_core::Error::HomeDirectoryUnavailable)
    ));
}
