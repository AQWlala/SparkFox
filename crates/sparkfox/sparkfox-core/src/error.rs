//! SparkFox 统一错误类型 — 跨 crate 共享

use std::fmt;

#[derive(Debug)]
pub enum Error {
    Storage { msg: String, ctx: String },
    Parse { msg: String, ctx: String },
    Io(std::io::Error),
    Db(rusqlite::Error),
    Crdt(String),
    Crypto(String),
    Llm(String),
    NotFound { kind: String, id: String },
    InvalidArgument { msg: String, ctx: String },
    Internal(String),
}

impl Error {
    pub fn storage(msg: String, ctx: &str) -> Self {
        Self::Storage {
            msg,
            ctx: ctx.to_string(),
        }
    }
    pub fn parse(msg: String, ctx: &str) -> Self {
        Self::Parse {
            msg,
            ctx: ctx.to_string(),
        }
    }
    pub fn not_found(kind: &str, id: impl fmt::Display) -> Self {
        Self::NotFound {
            kind: kind.to_string(),
            id: id.to_string(),
        }
    }
    pub fn invalid_argument(msg: String, ctx: &str) -> Self {
        Self::InvalidArgument {
            msg,
            ctx: ctx.to_string(),
        }
    }
    pub fn crdt(msg: impl Into<String>) -> Self {
        Self::Crdt(msg.into())
    }
    pub fn crypto(msg: impl Into<String>) -> Self {
        Self::Crypto(msg.into())
    }
    pub fn llm(msg: impl Into<String>) -> Self {
        Self::Llm(msg.into())
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage { msg, ctx } => write!(f, "[storage:{ctx}] {msg}"),
            Self::Parse { msg, ctx } => write!(f, "[parse:{ctx}] {msg}"),
            Self::Io(e) => write!(f, "[io] {e}"),
            Self::Db(e) => write!(f, "[db] {e}"),
            Self::Crdt(m) => write!(f, "[crdt] {m}"),
            Self::Crypto(m) => write!(f, "[crypto] {m}"),
            Self::Llm(m) => write!(f, "[llm] {m}"),
            Self::NotFound { kind, id } => write!(f, "[not_found] {kind}={id}"),
            Self::InvalidArgument { msg, ctx } => write!(f, "[invalid_arg:{ctx}] {msg}"),
            Self::Internal(m) => write!(f, "[internal] {m}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::Db(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
