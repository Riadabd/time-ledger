use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimeAmount {
    minutes: i64,
}

#[derive(Debug)]
pub enum TimeError {
    Empty,
    InvalidToken(String),
    InvalidNumber(String),
    Negative,
}

impl TimeAmount {
    pub const MINUTES_PER_HOUR: i64 = 60;
    pub const MINUTES_PER_DAY: i64 = 8 * 60;

    pub fn from_minutes(minutes: i64) -> Result<Self, TimeError> {
        if minutes < 0 {
            return Err(TimeError::Negative);
        }
        Ok(Self { minutes })
    }

    pub fn minutes(self) -> i64 {
        self.minutes
    }

    pub fn parse(input: &str) -> Result<Self, TimeError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(TimeError::Empty);
        }

        let mut minutes: i64 = 0;
        for token in trimmed.split_whitespace() {
            if token.len() < 2 {
                return Err(TimeError::InvalidToken(token.to_string()));
            }
            let (value_part, unit) = token.split_at(token.len() - 1);
            let value: i64 = value_part
                .parse()
                .map_err(|_| TimeError::InvalidNumber(token.to_string()))?;
            match unit {
                "d" => minutes += value * Self::MINUTES_PER_DAY,
                "h" => minutes += value * Self::MINUTES_PER_HOUR,
                "m" => minutes += value,
                _ => return Err(TimeError::InvalidToken(token.to_string())),
            }
        }

        Self::from_minutes(minutes)
    }

    pub fn format(self) -> String {
        let mut remaining = self.minutes;
        let mut parts: Vec<String> = Vec::new();

        let days = remaining / Self::MINUTES_PER_DAY;
        remaining %= Self::MINUTES_PER_DAY;
        let hours = remaining / Self::MINUTES_PER_HOUR;
        let minutes = remaining % Self::MINUTES_PER_HOUR;

        if days > 0 {
            parts.push(format!("{days}d"));
        }
        if hours > 0 {
            parts.push(format!("{hours}h"));
        }
        if minutes > 0 || parts.is_empty() {
            parts.push(format!("{minutes}m"));
        }

        parts.join(" ")
    }
}

impl fmt::Display for TimeAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format())
    }
}

impl fmt::Display for TimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeError::Empty => f.write_str("empty time"),
            TimeError::InvalidToken(token) => write!(f, "invalid token '{token}'"),
            TimeError::InvalidNumber(token) => write!(f, "invalid number in '{token}'"),
            TimeError::Negative => f.write_str("negative time"),
        }
    }
}

impl std::error::Error for TimeError {}
