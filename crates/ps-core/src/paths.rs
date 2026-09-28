//! Application-owned paths under `~/.1537paperstreet`.

use std::env;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// Environment variable that overrides the application data root.
pub const ROOT_OVERRIDE_ENV: &str = "PAPERSTREET_HOME";

/// Paths owned by 1537paperstreet outside user projects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppPaths {
    root: PathBuf,
}

impl AppPaths {
    /// Resolves the application root from [`ROOT_OVERRIDE_ENV`] or the current home directory.
    pub fn discover() -> Result<Self> {
        if let Some(root) = env::var_os(ROOT_OVERRIDE_ENV).filter(|value| !value.is_empty()) {
            return Ok(Self::from_root(root));
        }

        let home = home_dir()?;
        Ok(Self::from_root(home.join(".1537paperstreet")))
    }

    /// Creates a path set rooted at `root`.
    ///
    /// ```
    /// use ps_core::paths::AppPaths;
    ///
    /// let dir = tempfile::tempdir().unwrap();
    /// let paths = AppPaths::from_root(dir.path());
    /// assert!(paths.mermaid_cache().ends_with("cache/mermaid"));
    /// assert!(paths.log_file().ends_with("logs/app.log"));
    /// ```
    pub fn from_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Creates the application root and its required subdirectories.
    pub fn ensure(&self) -> Result<()> {
        for path in [
            self.root.clone(),
            self.themes(),
            self.cache(),
            self.mermaid_cache(),
            self.logs(),
            self.agents(),
        ] {
            std::fs::create_dir_all(&path)
                .map_err(|source| Error::io("create the application directory", path, source))?;
        }
        Ok(())
    }

    /// Returns the application data root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the user-theme directory.
    pub fn themes(&self) -> PathBuf {
        self.root.join("themes")
    }

    /// Returns the application cache directory.
    pub fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }

    /// Returns the on-disk Mermaid SVG cache directory.
    pub fn mermaid_cache(&self) -> PathBuf {
        self.cache().join("mermaid")
    }

    /// Returns the application log directory.
    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// Returns the rotating application log file.
    pub fn log_file(&self) -> PathBuf {
        self.logs().join("app.log")
    }

    /// Returns the single-instance lock file.
    pub fn instance_lock_file(&self) -> PathBuf {
        self.root.join("instance.lock")
    }

    /// Returns the configuration file path.
    pub fn config_file(&self) -> PathBuf {
        self.root.join("config.json")
    }

    /// Returns the project-list file path.
    pub fn projects_file(&self) -> PathBuf {
        self.root.join("projects.json")
    }

    /// Returns the persisted UI session file path.
    pub fn ui_state_file(&self) -> PathBuf {
        self.root.join("ui-state.json")
    }

    /// Returns the directory for assistant prompt history.
    pub fn agents(&self) -> PathBuf {
        self.root.join("agents")
    }

    /// Returns the prompt-history file path.
    pub fn agent_prompts_file(&self) -> PathBuf {
        self.agents().join("prompts.json")
    }
}

fn home_dir() -> Result<PathBuf> {
    home_from(env::var_os("HOME"), env::var_os("USERPROFILE"))
}

fn home_from(
    home: Option<std::ffi::OsString>,
    userprofile: Option<std::ffi::OsString>,
) -> Result<PathBuf> {
    for candidate in [home, userprofile] {
        if let Some(value) = candidate.filter(|value| !value.is_empty()) {
            return Ok(PathBuf::from(value));
        }
    }
    Err(Error::HomeDirectoryUnavailable)
}

#[cfg(test)]
mod tests {
    use super::home_from;
    use std::path::PathBuf;

    #[test]
    fn home_prefers_home_then_userprofile() {
        assert_eq!(
            home_from(Some("a".into()), Some("b".into())).expect("home"),
            PathBuf::from("a")
        );
        assert_eq!(
            home_from(None, Some("b".into())).expect("userprofile"),
            PathBuf::from("b")
        );
        assert!(home_from(Some("".into()), None).is_err());
    }
}
