//! sparkfox-core 类型契约测试
#![forbid(unsafe_code)]

use sparkfox_core::{AgentId, Error, Id, MemoryId, Result};

#[test]
fn id_generation_is_unique() {
    let a = Id::<MemoryId>::new();
    let b = Id::<MemoryId>::new();
    assert_ne!(a, b, "两个新生成的 Id 必须不同");
}

#[test]
fn id_string_roundtrip() {
    let id = Id::<AgentId>::new();
    let s = id.to_string();
    let parsed: Id<AgentId> = s.parse().expect("解析成功");
    assert_eq!(id, parsed);
}

#[test]
fn error_display_contains_context() {
    let err = Error::storage("表不存在".into(), "memory_l0");
    let s = format!("{err}");
    assert!(s.contains("memory_l0"), "错误信息必须含上下文");
    assert!(s.contains("表不存在"));
}

#[test]
fn result_alias_compiles() {
    fn _f() -> Result<u32> {
        Ok(42)
    }
}
