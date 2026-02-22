use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

const APP_CONFIG_DIR: &str = "time-ledger";
const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Debug)]
pub enum ConfigError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    MissingLedgerDir {
        searched_paths: Vec<PathBuf>,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Read { path, source } => {
                write!(
                    f,
                    "failed to read config file '{}': {source}",
                    path.display()
                )
            }
            ConfigError::Parse { path, source } => {
                write!(
                    f,
                    "failed to parse config file '{}': {source}",
                    path.display()
                )
            }
            ConfigError::MissingLedgerDir { searched_paths } => {
                if searched_paths.is_empty() {
                    write!(
                        f,
                        "no ledger directory configured. Pass --ledger-dir DIR, or set HOME/XDG_CONFIG_HOME and create a config file"
                    )
                } else {
                    write!(
                        f,
                        "no ledger directory configured. Pass --ledger-dir DIR, or create a config file in: {}",
                        format_config_paths_for_message(searched_paths)
                    )
                }
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Read { source, .. } => Some(source),
            ConfigError::Parse { source, .. } => Some(source),
            ConfigError::MissingLedgerDir { .. } => None,
        }
    }
}

fn format_config_paths_for_message(paths: &[PathBuf]) -> String {
    let rendered = paths
        .iter()
        .map(|path| format!("'{}'", path.display()))
        .collect::<Vec<String>>();

    match rendered.as_slice() {
        [] => String::new(),
        [single] => single.clone(),
        [first, second] => format!("{first} or {second}"),
        _ => {
            let (head, tail) = rendered.split_at(rendered.len().saturating_sub(1));
            format!(
                "{} or {}",
                head.join(", "),
                tail.first().cloned().unwrap_or_default()
            )
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileConfig {
    ledger_dir: PathBuf,
}

#[derive(Clone, Debug)]
struct ResolveContext {
    xdg_config_home: Option<PathBuf>,
    home_dir: Option<PathBuf>,
}

impl ResolveContext {
    fn from_env() -> Self {
        Self {
            xdg_config_home: env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            home_dir: env::var_os("HOME").map(PathBuf::from),
        }
    }

    fn config_candidates(&self) -> Vec<PathBuf> {
        let mut candidates = Vec::new();

        if let Some(xdg_home) = &self.xdg_config_home {
            candidates.push(xdg_home.join(APP_CONFIG_DIR).join(CONFIG_FILE_NAME));
        }

        if let Some(home_dir) = &self.home_dir {
            let home_candidate = home_dir
                .join(".config")
                .join(APP_CONFIG_DIR)
                .join(CONFIG_FILE_NAME);
            if !candidates
                .iter()
                .any(|candidate| candidate == &home_candidate)
            {
                candidates.push(home_candidate);
            }
        }

        candidates
    }
}

pub fn resolve_ledger_dir(cli_ledger_dir: Option<&Path>) -> Result<PathBuf, ConfigError> {
    let context = ResolveContext::from_env();
    resolve_ledger_dir_with_context(cli_ledger_dir, &context)
}

fn resolve_ledger_dir_with_context(
    cli_ledger_dir: Option<&Path>,
    context: &ResolveContext,
) -> Result<PathBuf, ConfigError> {
    if let Some(cli_dir) = cli_ledger_dir {
        return Ok(expand_home(cli_dir, context.home_dir.as_deref()));
    }

    let candidates = context.config_candidates();
    for config_path in &candidates {
        if !config_path.exists() {
            continue;
        }

        let file_content = fs::read_to_string(config_path).map_err(|source| ConfigError::Read {
            path: config_path.to_path_buf(),
            source,
        })?;
        let config =
            toml::from_str::<FileConfig>(&file_content).map_err(|source| ConfigError::Parse {
                path: config_path.to_path_buf(),
                source,
            })?;
        return Ok(expand_home(
            config.ledger_dir.as_path(),
            context.home_dir.as_deref(),
        ));
    }

    Err(ConfigError::MissingLedgerDir {
        searched_paths: candidates,
    })
}

fn expand_home(path: &Path, home_dir: Option<&Path>) -> PathBuf {
    let Some(home_dir) = home_dir else {
        return path.to_path_buf();
    };

    let Some(path_str) = path.to_str() else {
        return path.to_path_buf();
    };

    if path_str == "~" {
        return home_dir.to_path_buf();
    }

    if let Some(stripped) = path_str.strip_prefix("~/") {
        return home_dir.join(stripped);
    }

    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::config::{ConfigError, ResolveContext, resolve_ledger_dir_with_context};

    #[test]
    fn cli_flag_overrides_config_file() {
        let xdg_home = unique_temp_dir("xdg-override");
        write_config(&xdg_home, "ledger_dir = \"/tmp/from-config\"")
            .expect("config file should be written");
        let context = ResolveContext {
            xdg_config_home: Some(xdg_home.clone()),
            home_dir: None,
        };

        let ledger_dir =
            resolve_ledger_dir_with_context(Some(Path::new("/tmp/from-cli")), &context)
                .expect("cli value should win");
        assert_eq!(ledger_dir, Path::new("/tmp/from-cli"));

        let _ = fs::remove_dir_all(&xdg_home);
    }

    #[test]
    fn xdg_config_takes_precedence_over_home_config() {
        let xdg_home = unique_temp_dir("xdg-precedence");
        let home_dir = unique_temp_dir("home-precedence");
        write_config(&xdg_home, "ledger_dir = \"/tmp/from-xdg\"")
            .expect("xdg config file should be written");
        write_home_config(&home_dir, "ledger_dir = \"/tmp/from-home\"")
            .expect("home config file should be written");
        let context = ResolveContext {
            xdg_config_home: Some(xdg_home.clone()),
            home_dir: Some(home_dir.clone()),
        };

        let ledger_dir =
            resolve_ledger_dir_with_context(None, &context).expect("xdg config should load");
        assert_eq!(ledger_dir, Path::new("/tmp/from-xdg"));

        let _ = fs::remove_dir_all(&xdg_home);
        let _ = fs::remove_dir_all(&home_dir);
    }

    #[test]
    fn home_config_is_used_when_xdg_is_absent() {
        let home_dir = unique_temp_dir("home-only");
        write_home_config(&home_dir, "ledger_dir = \"/tmp/from-home\"")
            .expect("home config file should be written");
        let context = ResolveContext {
            xdg_config_home: None,
            home_dir: Some(home_dir.clone()),
        };

        let ledger_dir =
            resolve_ledger_dir_with_context(None, &context).expect("home config should load");
        assert_eq!(ledger_dir, Path::new("/tmp/from-home"));

        let _ = fs::remove_dir_all(&home_dir);
    }

    #[test]
    fn errors_when_no_cli_and_no_config_is_present() {
        let context = ResolveContext {
            xdg_config_home: None,
            home_dir: None,
        };

        let err = resolve_ledger_dir_with_context(None, &context)
            .expect_err("missing config should return an explicit error");
        match err {
            ConfigError::MissingLedgerDir { searched_paths } => {
                assert!(searched_paths.is_empty());
            }
            _ => panic!("expected missing ledger dir error"),
        }
    }

    #[test]
    fn parse_errors_include_config_path() {
        let xdg_home = unique_temp_dir("parse-error");
        let config_path =
            write_config(&xdg_home, "ledger_dir = [1, 2, 3]").expect("config file should exist");
        let context = ResolveContext {
            xdg_config_home: Some(xdg_home.clone()),
            home_dir: None,
        };

        let err = resolve_ledger_dir_with_context(None, &context).expect_err("parse should fail");
        match err {
            ConfigError::Parse { path, .. } => assert_eq!(path, config_path),
            _ => panic!("expected parse error"),
        }

        let _ = fs::remove_dir_all(&xdg_home);
    }

    #[test]
    fn tilde_paths_are_expanded_against_home_directory() {
        let home_dir = unique_temp_dir("tilde-expand");
        write_home_config(&home_dir, "ledger_dir = \"~/ledger-root\"")
            .expect("home config file should be written");
        let context = ResolveContext {
            xdg_config_home: None,
            home_dir: Some(home_dir.clone()),
        };

        let ledger_dir =
            resolve_ledger_dir_with_context(None, &context).expect("tilde path should expand");
        assert_eq!(ledger_dir, home_dir.join("ledger-root"));

        let _ = fs::remove_dir_all(&home_dir);
    }

    #[test]
    fn missing_ledger_dir_message_uses_or_between_config_paths() {
        let err = ConfigError::MissingLedgerDir {
            searched_paths: vec![
                Path::new("/tmp/xdg/time-ledger/config.toml").to_path_buf(),
                Path::new("/tmp/home/.config/time-ledger/config.toml").to_path_buf(),
            ],
        };

        let message = err.to_string();
        assert!(message.contains("create a config file in:"));
        assert!(message.contains(
            "'/tmp/xdg/time-ledger/config.toml' or '/tmp/home/.config/time-ledger/config.toml'"
        ));
    }

    fn write_config(root: &Path, content: &str) -> std::io::Result<std::path::PathBuf> {
        let config_dir = root.join("time-ledger");
        fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("config.toml");
        fs::write(&config_path, content)?;
        Ok(config_path)
    }

    fn write_home_config(home_dir: &Path, content: &str) -> std::io::Result<std::path::PathBuf> {
        let config_root = home_dir.join(".config");
        write_config(config_root.as_path(), content)
    }

    fn unique_temp_dir(label: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let dir = env::temp_dir().join(format!(
            "time-ledger-config-{label}-{}-{}",
            std::process::id(),
            stamp
        ));
        fs::create_dir_all(&dir).expect("temporary directory should be created");
        dir
    }
}
