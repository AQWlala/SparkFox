//! 强类型 Id — 防止不同实体 Id 混用
//!
//! 设计参考 OpenAkita 的 Id 系统（AGPL，清洁室重写）

use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Error;

/// Id 类型标记 trait
pub trait IdKind: Copy + 'static {
    const PREFIX: &'static str;
}

/// 强类型 Id
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Id<T: IdKind>(pub Uuid, PhantomData<T>);

impl<T: IdKind> Id<T> {
    pub fn new() -> Self {
        Self(Uuid::new_v4(), PhantomData)
    }

    pub fn from_uuid(u: Uuid) -> Self {
        Self(u, PhantomData)
    }
}

impl<T: IdKind> Default for Id<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: IdKind> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}_{}", T::PREFIX, self.0.simple())
    }
}

impl<T: IdKind> FromStr for Id<T> {
    type Err = Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let (prefix, rest) = s
            .split_once('_')
            .ok_or_else(|| Error::parse(format!("Id 缺少下划线: {s}"), "Id::from_str"))?;
        if prefix != T::PREFIX {
            return Err(Error::parse(
                format!("期望前缀 {} 实际 {prefix}", T::PREFIX),
                "Id::from_str",
            ));
        }
        let u = Uuid::parse_str(rest)
            .map_err(|e| Error::parse(format!("UUID 解析失败: {e}"), "Id::from_str"))?;
        Ok(Self(u, PhantomData))
    }
}

// IdKind 实现
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryId;
impl IdKind for MemoryId {
    const PREFIX: &'static str = "mem";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentId;
impl IdKind for AgentId {
    const PREFIX: &'static str = "agent";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionId;
impl IdKind for SessionId {
    const PREFIX: &'static str = "sess";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageId;
impl IdKind for MessageId {
    const PREFIX: &'static str = "msg";
}
