//! Open a file, folder, or http(s) URL with the host desktop.

use std::path::Path;
use std::process::Command;

use ps_core::{Error, Result};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HostCommand {
    pub program: &'static str,
    pub args: Vec<String>,
    /// Explorer on Windows returns a non-zero status after it opens the window.
    ignore_status: bool,
}

pub(crate) fn reveal(path: &Path) -> Result<()> {
    let primary = reveal_command(path);
    match run(&primary, "reveal the item", path) {
        Ok(()) => Ok(()),
        Err(error) => match reveal_fallback(path) {
            Some(fallback) => run(&fallback, "reveal the item", path),
            None => Err(error),
        },
    }
}

pub(crate) fn launch(path: &Path) -> Result<()> {
    run(
        &launch_command(path),
        "open the file in an external editor",
        path,
    )
}

pub(crate) fn open_url(url: &str) -> Result<()> {
    run(
        &url_command(url),
        "open the link in a browser",
        Path::new(url),
    )
}

pub(crate) fn reveal_command(path: &Path) -> HostCommand {
    #[cfg(target_os = "macos")]
    {
        HostCommand {
            program: "/usr/bin/open",
            args: vec!["-R".to_owned(), "--".to_owned(), path_arg(path)],
            ignore_status: false,
        }
    }
    #[cfg(target_os = "windows")]
    {
        HostCommand {
            program: "explorer",
            args: vec![format!("/select,{}", path.display())],
            ignore_status: true,
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        HostCommand {
            program: "dbus-send",
            args: vec![
                "--session".to_owned(),
                "--dest=org.freedesktop.FileManager1".to_owned(),
                "--type=method_call".to_owned(),
                "/org/freedesktop/FileManager1".to_owned(),
                "org.freedesktop.FileManager1.ShowItems".to_owned(),
                format!("array:string:{}", file_uri(path)),
                "string:\"\"".to_owned(),
            ],
            ignore_status: false,
        }
    }
}

fn reveal_fallback(path: &Path) -> Option<HostCommand> {
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Some(launch_command(path))
    }
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        let _ = path;
        None
    }
}

pub(crate) fn launch_command(path: &Path) -> HostCommand {
    #[cfg(target_os = "macos")]
    {
        HostCommand {
            program: "/usr/bin/open",
            args: vec!["--".to_owned(), path_arg(path)],
            ignore_status: false,
        }
    }
    #[cfg(target_os = "windows")]
    {
        HostCommand {
            program: "rundll32",
            args: vec!["url.dll,FileProtocolHandler".to_owned(), path_arg(path)],
            ignore_status: false,
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        HostCommand {
            program: "xdg-open",
            args: vec![path_arg(path)],
            ignore_status: false,
        }
    }
}

pub(crate) fn url_command(url: &str) -> HostCommand {
    #[cfg(target_os = "macos")]
    {
        HostCommand {
            program: "/usr/bin/open",
            args: vec!["--".to_owned(), url.to_owned()],
            ignore_status: false,
        }
    }
    #[cfg(target_os = "windows")]
    {
        HostCommand {
            program: "rundll32",
            args: vec!["url.dll,FileProtocolHandler".to_owned(), url.to_owned()],
            ignore_status: false,
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        HostCommand {
            program: "xdg-open",
            args: vec![url.to_owned()],
            ignore_status: false,
        }
    }
}

fn path_arg(path: &Path) -> String {
    path.display().to_string()
}

/// Percent-encodes `path` as a `file://` URI for FileManager1.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for character in path_arg(path).chars() {
        match character {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '/' | '-' | '_' | '.' | '~' => uri.push(character),
            other => {
                let mut bytes = [0_u8; 4];
                for byte in other.encode_utf8(&mut bytes).bytes() {
                    uri.push_str(&format!("%{byte:02X}"));
                }
            }
        }
    }
    uri
}

fn run(command: &HostCommand, action: &'static str, path: &Path) -> Result<()> {
    let status = spawn(command).map_err(|source| Error::Io {
        action,
        path: path.to_path_buf(),
        source,
    })?;
    if status.success() || command.ignore_status {
        return Ok(());
    }
    Err(Error::Io {
        action,
        path: path.to_path_buf(),
        source: std::io::Error::other("the system could not open that item"),
    })
}

fn spawn(command: &HostCommand) -> std::io::Result<std::process::ExitStatus> {
    let mut child = Command::new(command.program);
    child.args(&command.args);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        child.creation_flags(CREATE_NO_WINDOW);
    }
    child.status()
}

#[cfg(test)]
mod tests {
    use super::{launch_command, reveal_command, url_command};
    use std::path::Path;

    #[test]
    fn reveal_uses_the_host_file_manager() {
        let path = Path::new("/tmp/notes/readme.md");
        let command = reveal_command(path);
        #[cfg(target_os = "macos")]
        {
            assert_eq!(command.program, "/usr/bin/open");
            assert_eq!(
                command.args,
                vec![
                    "-R".to_owned(),
                    "--".to_owned(),
                    "/tmp/notes/readme.md".to_owned()
                ]
            );
        }
        #[cfg(target_os = "windows")]
        {
            assert_eq!(command.program, "explorer");
            assert!(command.ignore_status);
            assert!(command.args[0].contains("readme.md"));
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            assert_eq!(command.program, "dbus-send");
            assert!(
                command
                    .args
                    .iter()
                    .any(|arg| arg == "array:string:file:///tmp/notes/readme.md")
            );
        }
    }

    #[test]
    fn launch_and_url_use_the_host_opener() {
        let path = Path::new("/tmp/notes/readme.md");
        let launch = launch_command(path);
        let url = url_command("https://example.com/notes");
        #[cfg(target_os = "macos")]
        {
            assert_eq!(launch.program, "/usr/bin/open");
            assert_eq!(url.program, "/usr/bin/open");
            assert_eq!(
                url.args.last().map(String::as_str),
                Some("https://example.com/notes")
            );
        }
        #[cfg(target_os = "windows")]
        {
            assert_eq!(launch.program, "rundll32");
            assert_eq!(url.program, "rundll32");
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            assert_eq!(launch.program, "xdg-open");
            assert_eq!(url.program, "xdg-open");
            assert_eq!(url.args, vec!["https://example.com/notes".to_owned()]);
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    #[test]
    fn file_uri_encodes_spaces_and_keeps_slashes() {
        let uri = super::file_uri(Path::new("/tmp/my notes/readme.md"));
        assert_eq!(uri, "file:///tmp/my%20notes/readme.md");
    }
}
