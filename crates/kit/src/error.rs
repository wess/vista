use std::fmt;

/// Every failure Vista's core can report. The message is what a person reads,
/// so variants carry enough context to say what was being attempted.
#[derive(Debug)]
pub enum Error {
  Io(std::io::Error),
  Db(sqlx::Error),
  Http(reqwest::Error),
  Json(serde_json::Error),
  Message(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
  pub fn msg(text: impl Into<String>) -> Self {
    Error::Message(text.into())
  }
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Error::Io(err) => write!(f, "{err}"),
      Error::Db(err) => write!(f, "library database: {err}"),
      Error::Http(err) => write!(f, "network: {err}"),
      Error::Json(err) => write!(f, "{err}"),
      Error::Message(text) => f.write_str(text),
    }
  }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
  fn from(err: std::io::Error) -> Self {
    Error::Io(err)
  }
}

impl From<sqlx::Error> for Error {
  fn from(err: sqlx::Error) -> Self {
    Error::Db(err)
  }
}

impl From<reqwest::Error> for Error {
  fn from(err: reqwest::Error) -> Self {
    Error::Http(err)
  }
}

impl From<serde_json::Error> for Error {
  fn from(err: serde_json::Error) -> Self {
    Error::Json(err)
  }
}
