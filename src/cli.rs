use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[arg(
        long = "ledger-dir",
        default_value = "data/",
        help = "Directory containing weekly ledger files"
    )]
    pub ledger_dir: PathBuf,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::{CommandFactory, Parser};

    #[test]
    fn cli_uses_default_ledger_dir_when_flag_missing() {
        let cli = crate::cli::Cli::try_parse_from(["time-ledger"]).expect("default should parse");
        assert_eq!(cli.ledger_dir, PathBuf::from("data"));
    }

    #[test]
    fn cli_supports_space_delimited_ledger_dir() {
        let cli =
            crate::cli::Cli::try_parse_from(["time-ledger", "--ledger-dir", "/tmp/my-ledgers"])
                .expect("flag should parse");
        assert_eq!(cli.ledger_dir, PathBuf::from("/tmp/my-ledgers"));
    }

    #[test]
    fn cli_supports_equals_form_ledger_dir() {
        let cli = crate::cli::Cli::try_parse_from(["time-ledger", "--ledger-dir=/tmp/my-ledgers"])
            .expect("flag should parse");
        assert_eq!(cli.ledger_dir, PathBuf::from("/tmp/my-ledgers"));
    }

    #[test]
    fn cli_errors_when_ledger_dir_value_missing() {
        let err = crate::cli::Cli::try_parse_from(["time-ledger", "--ledger-dir"])
            .expect_err("missing value should return an error");
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidValue);
    }

    #[test]
    fn cli_errors_for_unknown_flag() {
        let err = crate::cli::Cli::try_parse_from(["time-ledger", "--wat"])
            .expect_err("unknown flags should return an error");
        assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument);
    }

    #[test]
    fn cli_help_lists_ledger_flag_and_help_shortcut() {
        let mut command = crate::cli::Cli::command();
        let mut help = Vec::new();
        command
            .write_help(&mut help)
            .expect("writing help should succeed");
        let help = String::from_utf8(help).expect("help output should be utf8");

        assert!(help.contains("--ledger-dir"));
        assert!(help.contains("-h, --help"));
    }
}
