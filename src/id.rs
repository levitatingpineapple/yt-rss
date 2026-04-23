use std::{fmt, str::FromStr};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid channel ID")]
    InvalidChannelId,
    #[error("Invalid video ID")]
    InvalidVideoId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Handle(String);

impl FromStr for Handle {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.strip_prefix('@')
            .filter(|name| (3..=30).contains(&name.len()) && chars_allowed(name))
            .map(|name| Self(name.to_string()))
            .ok_or(Error::InvalidChannelId)
    }
}

impl fmt::Display for Handle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Video(String);

impl FromStr for Video {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 11 && chars_allowed(s) {
            Ok(Self(s.to_string()))
        } else {
            Err(Error::InvalidVideoId)
        }
    }
}

fn chars_allowed(s: &str) -> bool {
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

impl fmt::Display for Video {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
