//! Man page and shell completion generation

use std::{io, path::Path};

use clap::{CommandFactory as _, ValueEnum as _};
use clap_complete::Shell;

use crate::cl;

/// Generate man pages into the target directory
pub(crate) fn generate_man_pages(dir: &Path) -> anyhow::Result<()> {
    let cmd = cl::Args::command().name(env!("CARGO_BIN_NAME"));
    clap_mangen::generate_to(cmd, dir)?;
    Ok(())
}

/// Generate shell completions
///
/// If `shell` is specified, generates only for that shell.
/// If `dir` is specified, generates all completions into that directory.
pub(crate) fn generate_shell_completions(
    shell: Option<Shell>,
    dir: Option<&Path>,
) -> anyhow::Result<()> {
    let name = env!("CARGO_BIN_NAME");
    let mut cmd = cl::Args::command().name(name);

    if let Some(shell) = shell {
        clap_complete::generate(shell, &mut cmd, name, &mut io::stdout());
    } else if let Some(dir) = dir {
        let shells = Shell::value_variants();
        for shell_i in shells {
            clap_complete::generate_to(*shell_i, &mut cmd, name, dir)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// Read all man page content from a directory
    fn read_all_man_pages(dir: &Path) -> String {
        fs::read_dir(dir)
            .unwrap()
            .map(|e| fs::read_to_string(e.unwrap().path()).unwrap())
            .collect()
    }

    #[test]
    fn man_pages_generated() {
        let dir = tempfile::tempdir().unwrap();
        generate_man_pages(dir.path()).unwrap();
        let entries: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
        assert!(!entries.is_empty());
        for entry in entries {
            let path = entry.unwrap().path();
            assert!(path.extension().is_some_and(|e| e == "1"));
            let content = fs::read_to_string(&path).unwrap();
            assert!(!content.is_empty());
        }
    }

    #[test]
    fn shell_completions_generated() {
        let dir = tempfile::tempdir().unwrap();
        generate_shell_completions(None, Some(dir.path())).unwrap();
        let entries: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
        assert!(!entries.is_empty());
        for entry in entries {
            let content = fs::read_to_string(entry.unwrap().path()).unwrap();
            assert!(!content.is_empty());
        }
    }

    #[cfg(not(feature = "temp-log"))]
    #[test]
    fn man_pages_omit_temp_log_options() {
        let dir = tempfile::tempdir().unwrap();
        generate_man_pages(dir.path()).unwrap();
        let all_content = read_all_man_pages(dir.path());
        // roff escapes hyphens as \-
        assert!(!all_content.contains("temp\\-log"));
        assert!(!all_content.contains("temp\\-log\\-max\\-files"));
    }

    #[cfg(not(feature = "temp-log"))]
    #[test]
    fn shell_completions_omit_temp_log_options() {
        let dir = tempfile::tempdir().unwrap();
        generate_shell_completions(None, Some(dir.path())).unwrap();
        for entry in fs::read_dir(dir.path()).unwrap() {
            let content = fs::read_to_string(entry.unwrap().path()).unwrap();
            assert!(!content.contains("temp-log"));
            assert!(!content.contains("temp-log-max-files"));
        }
    }

    #[cfg(feature = "temp-log")]
    #[test]
    fn man_pages_include_temp_log_options() {
        let dir = tempfile::tempdir().unwrap();
        generate_man_pages(dir.path()).unwrap();
        let all_content = read_all_man_pages(dir.path());
        // roff escapes hyphens as \-
        assert!(all_content.contains("temp\\-log"));
        assert!(all_content.contains("temp\\-log\\-max\\-files"));
    }

    #[cfg(feature = "temp-log")]
    #[test]
    fn shell_completions_include_temp_log_options() {
        let dir = tempfile::tempdir().unwrap();
        generate_shell_completions(None, Some(dir.path())).unwrap();
        let mut all_content = String::new();
        for entry in fs::read_dir(dir.path()).unwrap() {
            all_content.push_str(&fs::read_to_string(entry.unwrap().path()).unwrap());
        }
        assert!(all_content.contains("temp-log"));
        assert!(all_content.contains("temp-log-max-files"));
    }
}
