use std::path::PathBuf;

use chrono::NaiveDate;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[arg(
        long = "ledger-dir",
        value_name = "DIR",
        help = "Directory containing weekly ledger files (overrides config file)"
    )]
    pub ledger_dir: Option<PathBuf>,
    #[arg(
        long = "week-number",
        value_name = "DATE",
        num_args = 0..=1,
        value_parser = parse_date,
        help = "Print ISO week number for today or an optional YYYY-MM-DD date"
    )]
    pub week_number: Option<Option<NaiveDate>>,
}

impl Cli {
    pub fn requested_week_number_date(&self, today: NaiveDate) -> Option<NaiveDate> {
        match self.week_number.as_ref() {
            None => None,
            Some(Some(date)) => Some(*date),
            Some(None) => Some(today),
        }
    }
}

fn parse_date(input: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(input, "%Y-%m-%d")
        .map_err(|_| "expected date in YYYY-MM-DD format".to_string())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use chrono::NaiveDate;
    use clap::{CommandFactory, Parser};

    #[test]
    fn cli_uses_default_ledger_dir_when_flag_missing() {
        let cli = crate::cli::Cli::try_parse_from(["time-ledger"]).expect("default should parse");
        assert_eq!(cli.ledger_dir, None);
    }

    #[test]
    fn cli_supports_space_delimited_ledger_dir() {
        let cli =
            crate::cli::Cli::try_parse_from(["time-ledger", "--ledger-dir", "/tmp/my-ledgers"])
                .expect("flag should parse");
        assert_eq!(cli.ledger_dir, Some(PathBuf::from("/tmp/my-ledgers")));
    }

    #[test]
    fn cli_supports_equals_form_ledger_dir() {
        let cli = crate::cli::Cli::try_parse_from(["time-ledger", "--ledger-dir=/tmp/my-ledgers"])
            .expect("flag should parse");
        assert_eq!(cli.ledger_dir, Some(PathBuf::from("/tmp/my-ledgers")));
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
        assert!(help.contains("--week-number"));
        assert!(help.contains("DATE"));
        assert!(help.contains("-h, --help"));
    }

    #[test]
    fn cli_week_number_without_date_sets_flag() {
        let cli = crate::cli::Cli::try_parse_from(["time-ledger", "--week-number"])
            .expect("flag should parse");
        assert_eq!(cli.week_number, Some(None));
    }

    #[test]
    fn cli_week_number_with_date_sets_date() {
        let cli = crate::cli::Cli::try_parse_from(["time-ledger", "--week-number", "2026-02-08"])
            .expect("flag with date should parse");
        assert_eq!(
            cli.week_number,
            Some(Some(
                NaiveDate::from_ymd_opt(2026, 2, 8).expect("valid test date")
            ))
        );
    }

    #[test]
    fn cli_week_number_with_date_equals_form_sets_date() {
        let cli = crate::cli::Cli::try_parse_from(["time-ledger", "--week-number=2026-02-08"])
            .expect("flag with date should parse");
        assert_eq!(
            cli.week_number,
            Some(Some(
                NaiveDate::from_ymd_opt(2026, 2, 8).expect("valid test date")
            ))
        );
    }

    #[test]
    fn cli_week_number_rejects_invalid_date_format() {
        let err = crate::cli::Cli::try_parse_from(["time-ledger", "--week-number", "02/08/2026"])
            .expect_err("invalid date format should error");
        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
    }

    #[test]
    fn cli_requested_week_number_date_uses_today_when_value_missing() {
        let cli = crate::cli::Cli::try_parse_from(["time-ledger", "--week-number"])
            .expect("flag should parse");
        let today = NaiveDate::from_ymd_opt(2026, 2, 8).expect("valid test date");
        assert_eq!(cli.requested_week_number_date(today), Some(today));
    }

    #[test]
    fn cli_requested_week_number_date_uses_explicit_date_when_present() {
        let cli = crate::cli::Cli::try_parse_from(["time-ledger", "--week-number", "2026-02-01"])
            .expect("flag should parse");
        assert_eq!(
            cli.requested_week_number_date(
                NaiveDate::from_ymd_opt(2026, 2, 8).expect("valid test date")
            ),
            Some(NaiveDate::from_ymd_opt(2026, 2, 1).expect("valid test date"))
        );
    }

    #[test]
    fn cli_requested_week_number_date_is_none_when_flag_absent() {
        let cli = crate::cli::Cli::try_parse_from(["time-ledger"]).expect("default should parse");
        assert_eq!(
            cli.requested_week_number_date(
                NaiveDate::from_ymd_opt(2026, 2, 8).expect("valid test date")
            ),
            None
        );
    }
}
