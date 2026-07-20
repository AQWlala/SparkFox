//! sqlite-vec 向量扩展加载 — 清洁室重写 BaiLongma embedding.js
//!
//! 加载方式：通过 rusqlite::Connection::load_extension 加载
//! sqlite-vec.dll (Windows) / libsqlite_vec.so (Linux) / libsqlite_vec.dylib (macOS)
//! 路径优先级：
//!   1. 环境变量 SPARKFOX_SQLITE_VEC_PATH
//!   2. exe 同目录 sqlite-vec/ext.{so,dll,dylib}
//!   3. 用户数据目录 sparkfox/sqlite-vec/ext.{so,dll,dylib}

#![allow(unsafe_code)]  // FFI 加载扩展必须 unsafe

use std::path::PathBuf;

use rusqlite::Connection;

use sparkfox_core::{Error, Result};

pub fn load_vec_extension(conn: &Connection) -> Result<()> {
    let path = resolve_extension_path()
        .ok_or_else(|| Error::storage("sqlite-vec 扩展未找到".into(), "vec::load"))?;
    // SAFETY: sqlite-vec 是可信二进制（用户从官方 release 下载放置）。
    // load_extension 内部调用 SQLite C API 的 sqlite3_load_extension。
    unsafe {
        conn.load_extension(&path, None)
            .map_err(|e| Error::storage(format!("加载 sqlite-vec 失败: {e}"), "vec::load"))?;
    }
    // 验证 vec0 虚表可用
    conn.execute_batch("CREATE VIRTUAL TABLE IF NOT EXISTS __vec_probe USING vec0(x float[1]); DROP TABLE __vec_probe;")
        .map_err(|e| Error::storage(format!("vec0 验证失败: {e}"), "vec::load"))?;
    log::info!("sqlite-vec 扩展已加载: {}", path.display());
    Ok(())
}

fn resolve_extension_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("SPARKFOX_SQLITE_VEC_PATH") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    let ext = ext_for_platform();
    // exe 同目录
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("sqlite-vec").join(ext);
            if p.exists() {
                return Some(p);
            }
        }
    }
    // 用户数据目录
    if let Some(dir) = dirs_next::data_dir() {
        let p = dir.join("sparkfox").join("sqlite-vec").join(ext);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn ext_for_platform() -> &'static str {
    #[cfg(target_os = "windows")]
    { "sqlite_vec.dll" }
    #[cfg(target_os = "linux")]
    { "libsqlite_vec.so" }
    #[cfg(target_os = "macos")]
    { "libsqlite_vec.dylib" }
}
