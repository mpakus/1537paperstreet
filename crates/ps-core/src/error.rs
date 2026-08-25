//! Errors returned by core application services.

use std::io;
use std::path::PathBuf;

/// The error type shared by `ps-core` modules.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The application data directory could not be determined.
    #[error(
        "The application data directory could not be found. Set PAPERSTREET_HOME and try again."
    )]
    HomeDirectoryUnavailable,

    /// A filesystem operation on application data failed.
    #[error("Couldn't {action} at '{}'.", path.display())]
    Io {
        /// A user-readable description of the attempted operation.
        action: &'static str,
        /// The path involved in the operation.
        path: PathBuf,
        /// The underlying operating-system error.
        #[source]
        source: io::Error,
    },

    /// JSON could not be encoded or decoded.
    #[error("Couldn't {action} JSON at '{}'.", path.display())]
    Json {
        /// A user-readable description of the attempted operation.
        action: &'static str,
        /// The path involved in the operation.
        path: PathBuf,
        /// The underlying JSON error.
        #[source]
        source: serde_json::Error,
    },

    /// A stored schema is newer than the application supports or has no migration path.
    #[error(
        "This data uses schema version {found}, but this version of the app supports version {supported}."
    )]
    UnsupportedSchema {
        /// The schema version found on disk.
        found: u32,
        /// The newest supported schema version.
        supported: u32,
    },

    /// A configuration value is outside its supported range.
    #[error("The setting '{field}' must be between {min} and {max}; found {actual}.")]
    InvalidConfig {
        /// The invalid configuration field.
        field: &'static str,
        /// The smallest accepted value.
        min: i64,
        /// The largest accepted value.
        max: i64,
        /// The value found in the configuration.
        actual: i64,
    },

    /// A configuration string is not in the accepted format.
    #[error("The setting '{field}' must be a valid {expected}.")]
    InvalidConfigFormat {
        /// The invalid configuration field.
        field: &'static str,
        /// What the field should look like, in user-facing language.
        expected: &'static str,
    },

    /// A project record contains a value that cannot be stored safely.
    #[error("The project record is invalid: {reason}.")]
    InvalidProject {
        /// A user-readable explanation of the invalid value.
        reason: &'static str,
    },

    /// A project-relative path cannot be used safely.
    #[error("The path '{}' cannot be used: {reason}.", path.display())]
    UnsafePath {
        /// The rejected path.
        path: PathBuf,
        /// A user-readable explanation of why the path was rejected.
        reason: &'static str,
    },

    /// A resolved path points outside its project directory.
    #[error("The path '{}' points outside the project directory.", path.display())]
    PathOutsideProject {
        /// The rejected path.
        path: PathBuf,
    },

    /// A file or directory already uses the requested name.
    #[error(
        "A file or folder named '{}' already exists. Try '{suggested_name}'.",
        path.display()
    )]
    NameConflict {
        /// The occupied destination path.
        path: PathBuf,
        /// The next available file name in the destination directory.
        suggested_name: String,
    },

    /// Moving selected items to the operating system Trash failed.
    #[error("Couldn't move the selected items to Trash.")]
    Trash {
        /// The underlying Trash operation error.
        #[source]
        source: trash::Error,
    },

    /// Starting or configuring filesystem observation failed.
    #[error("Couldn't {action} for '{}'.", path.display())]
    Notify {
        /// A user-readable description of the attempted watcher operation.
        action: &'static str,
        /// The watched project path.
        path: PathBuf,
        /// The underlying notification backend error.
        #[source]
        source: notify::Error,
    },

    /// A requested project does not exist in the registry.
    #[error("The project '{id}' is no longer in the project list.")]
    ProjectNotFound {
        /// The missing project identifier.
        id: String,
    },

    /// A color theme file is missing required fields or uses an invalid color.
    #[error("The theme is invalid: {reason}.")]
    InvalidTheme {
        /// A user-readable explanation of the invalid theme.
        reason: &'static str,
    },

    /// An external link used a scheme that the app will not open.
    #[error("Only http and https links can be opened.")]
    UnsupportedUrl,

    /// The file on disk no longer matches the buffer that is being saved.
    #[error("This file changed on disk since it was opened. Your edits were not saved.")]
    DocumentConflict {
        /// Project-relative path of the document.
        path: PathBuf,
        /// Lowercase hexadecimal BLAKE3 hash of the bytes currently on disk.
        disk_hash: String,
    },

    /// A Finder drop contained no paths.
    #[error("Drop a Markdown file or a folder onto the window to open it.")]
    EmptyDrop,

    /// A dropped path is not a folder or a Markdown file.
    #[error(
        "Only Markdown files and folders can be opened. '{}' is not a Markdown file.",
        path.display()
    )]
    UnsupportedDrop {
        /// The rejected dropped path.
        path: PathBuf,
    },

    /// A project timestamp could not be represented in the storage format.
    #[error("Couldn't create a timestamp for the project record.")]
    TimeFormat {
        /// The underlying timestamp formatting error.
        #[source]
        source: time::error::Format,
    },

    /// GitHub did not return a recognizable release payload.
    #[error("GitHub did not return a release.")]
    InvalidUpdateRelease,

    /// The latest GitHub Release tag is not a semantic version.
    #[error("The latest GitHub release tag is not a version number.")]
    InvalidUpdateVersion,

    /// An external assistant could not be used.
    #[error("{message}")]
    Agent {
        /// User-visible explanation. Must not include prompt text or secrets.
        message: String,
    },
}

impl Error {
    pub(crate) fn io(action: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            action,
            path: path.into(),
            source,
        }
    }

    pub(crate) fn json(
        action: &'static str,
        path: impl Into<PathBuf>,
        source: serde_json::Error,
    ) -> Self {
        Self::Json {
            action,
            path: path.into(),
            source,
        }
    }
}

/// A result returned by `ps-core`.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_are_written_for_the_user() {
        assert!(
            Error::HomeDirectoryUnavailable
                .to_string()
                .contains("PAPERSTREET_HOME")
        );
        assert_eq!(
            Error::UnsupportedUrl.to_string(),
            "Only http and https links can be opened."
        );
        assert!(
            Error::EmptyDrop
                .to_string()
                .contains("Drop a Markdown file")
        );
        assert!(
            Error::ProjectNotFound { id: "abc".into() }
                .to_string()
                .contains("abc")
        );
        assert!(
            Error::InvalidTheme {
                reason: "the name is empty"
            }
            .to_string()
            .contains("the name is empty")
        );
        assert!(
            Error::UnsupportedDrop {
                path: PathBuf::from("cover.png")
            }
            .to_string()
            .contains("cover.png")
        );
        assert!(
            Error::InvalidConfig {
                field: "font_size",
                min: 10,
                max: 32,
                actual: 9,
            }
            .to_string()
            .contains("font_size")
        );
        assert!(
            Error::InvalidConfigFormat {
                field: "preview_bg",
                expected: "#RRGGBB color",
            }
            .to_string()
            .contains("preview_bg")
        );
        assert!(
            Error::PathOutsideProject {
                path: PathBuf::from("/tmp/x")
            }
            .to_string()
            .contains("outside")
        );
        assert!(
            Error::DocumentConflict {
                path: PathBuf::from("note.md"),
                disk_hash: "abc".into(),
            }
            .to_string()
            .contains("changed on disk")
        );
        assert_eq!(
            Error::InvalidUpdateRelease.to_string(),
            "GitHub did not return a release."
        );
        assert!(
            Error::InvalidUpdateVersion
                .to_string()
                .contains("version number")
        );
        assert_eq!(
            Error::Agent {
                message: "Open a folder first.".into()
            }
            .to_string(),
            "Open a folder first."
        );
        let json = Error::json(
            "decode",
            "config.json",
            serde_json::from_str::<u8>("not-json").expect_err("json"),
        );
        assert!(json.to_string().contains("config.json"));
        let io = Error::io(
            "read the file",
            "note.md",
            io::Error::new(io::ErrorKind::NotFound, "missing"),
        );
        assert!(io.to_string().contains("note.md"));
    }
}
