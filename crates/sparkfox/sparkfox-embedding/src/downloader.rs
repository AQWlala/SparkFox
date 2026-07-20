//! bge 模型下载 + SHA256 校验（防供应链攻击）
//!
//! 使用 hf-hub 0.4 同步 API 下载模型文件。
//! 模型默认缓存到 `%APPDATA%/sparkfox/models/`（Windows）或
//! `~/.local/share/sparkfox/models/`（Linux）。

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use sparkfox_core::{Error, Result};

/// 嵌入/重排模型变体
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelVariant {
    /// bge-small-zh-v1.5（120MB，512 维，默认）
    BgeSmallZh,
    /// bge-large-zh-v1.5（1.2GB，1024 维，可选）
    BgeLargeZh,
    /// bge-reranker-v2-m3（560MB，重排模型）
    BgeRerankerV2M3,
}

impl ModelVariant {
    pub fn repo_id(&self) -> &'static str {
        match self {
            Self::BgeSmallZh => "BAAI/bge-small-zh-v1.5",
            Self::BgeLargeZh => "BAAI/bge-large-zh-v1.5",
            Self::BgeRerankerV2M3 => "BAAI/bge-reranker-v2-m3",
        }
    }

    pub fn expected_files(&self) -> &'static [&'static str] {
        match self {
            Self::BgeSmallZh | Self::BgeLargeZh | Self::BgeRerankerV2M3 => &[
                "config.json",
                "tokenizer.json",
                "tokenizer_config.json",
                "model.safetensors",
                "special_tokens_map.json",
            ],
        }
    }

    pub fn dim(&self) -> usize {
        match self {
            Self::BgeSmallZh => 512,
            Self::BgeLargeZh => 1024,
            Self::BgeRerankerV2M3 => 1024, // reranker 输出维度，仅用于类型一致性
        }
    }

    pub fn size_mb(&self) -> usize {
        match self {
            Self::BgeSmallZh => 120,
            Self::BgeLargeZh => 1200,
            Self::BgeRerankerV2M3 => 560,
        }
    }
}

/// 模型缓存根目录
pub fn cache_dir() -> PathBuf {
    let base = dirs_next::data_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("sparkfox")
        .join("models");
    std::fs::create_dir_all(&base).ok();
    base
}

/// 单个模型变体的本地目录
pub fn model_dir(variant: &ModelVariant) -> PathBuf {
    cache_dir().join(variant.repo_id().replace('/', "_"))
}

/// 检查候选路径是否含全部 expected_files
fn is_complete_model_dir(dir: &Path, variant: &ModelVariant) -> bool {
    if !dir.is_dir() {
        return false;
    }
    variant
        .expected_files()
        .iter()
        .all(|f| dir.join(f).is_file())
}

/// 在多个候选路径中查找已预下载的模型目录
///
/// 查找顺序（首个匹配即返回）：
/// 1. `SPARKFOX_MODELS_DIR` 环境变量（开发/测试覆盖）
/// 2. 当前工作目录的 `.models/{repo_id_dir}/`（项目内预下载）
/// 3. `cache_dir()` 下的 `model_dir(variant)`（hf-hub 缓存目录）
///
/// 返回的目录保证含全部 expected_files，否则返回 None。
pub fn find_local_model_dir(variant: &ModelVariant) -> Option<PathBuf> {
    let repo_dir_name = variant.repo_id().replace('/', "_");

    // 1. 环境变量覆盖
    if let Ok(dir) = std::env::var("SPARKFOX_MODELS_DIR") {
        let candidate = PathBuf::from(dir).join(&repo_dir_name);
        if is_complete_model_dir(&candidate, variant) {
            log::debug!("命中 SPARKFOX_MODELS_DIR: {}", candidate.display());
            return Some(candidate);
        }
    }

    // 2. 当前工作目录的 .models/
    let cwd_candidate = std::env::current_dir()
        .ok()?
        .join(".models")
        .join(&repo_dir_name);
    if is_complete_model_dir(&cwd_candidate, variant) {
        log::debug!("命中 cwd .models/: {}", cwd_candidate.display());
        return Some(cwd_candidate);
    }

    // 3. cache_dir（hf-hub 缓存）
    let cache_candidate = model_dir(variant);
    if is_complete_model_dir(&cache_candidate, variant) {
        log::debug!("命中 cache_dir: {}", cache_candidate.display());
        return Some(cache_candidate);
    }

    None
}

/// 下载模型所有文件（若本地已存在则跳过）
///
/// 优先使用 `find_local_model_dir` 查找本地预下载的模型；
/// 若未找到，则通过 hf-hub 0.4 同步 API 下载到 `cache_dir()`。
pub fn download_model(variant: &ModelVariant) -> Result<PathBuf> {
    // 优先本地
    if let Some(local) = find_local_model_dir(variant) {
        log::info!("使用本地预下载模型: {}", local.display());
        return Ok(local);
    }

    // 退回 hf-hub
    let dir = model_dir(variant);
    std::fs::create_dir_all(&dir).map_err(Error::from)?;

    let api = hf_hub::api::sync::ApiBuilder::from_env()
        .with_cache_dir(cache_dir())
        .build()
        .map_err(|e| Error::internal(format!("hf-hub init 失败: {e}")))?;

    for filename in variant.expected_files() {
        log::info!("下载 {} / {}", variant.repo_id(), filename);
        let _path: PathBuf = api
            .model(variant.repo_id().to_string())
            .get(filename)
            .map_err(|e| Error::internal(format!("下载 {filename} 失败: {e}")))?;
    }

    log::info!(
        "模型 {} 已就绪: {}",
        variant.repo_id(),
        dir.display()
    );
    Ok(dir)
}

/// 校验文件 SHA256（防供应链攻击）
///
/// `expected` 为小写十六进制字符串（64 字符）。
pub fn verify_sha256(path: &Path, expected: &str) -> Result<()> {
    let mut hasher = Sha256::new();
    let bytes = std::fs::read(path).map_err(Error::from)?;
    hasher.update(&bytes);
    let actual = hex::encode(hasher.finalize());
    if actual != expected.to_lowercase() {
        return Err(Error::crypto(format!(
            "SHA256 校验失败: {} 期望 {expected} 实际 {actual}",
            path.display()
        )));
    }
    Ok(())
}

/// 【S-02 P0 修复】模型文件 SHA256 期望值常量表
///
/// 生产部署前必须填充。开发阶段可为空（仅记录 warn 日志，不阻塞加载）。
///
/// # 填充方法
///
/// 1. 运行 `cargo test -p sparkfox-embedding --lib print_model_sha256 -- --ignored`
///    打印实际 SHA256
/// 2. 将输出填入下表
/// 3. 重新编译
///
/// # 安全说明
///
/// - 常量表非空时，`verify_model_files` 会强制校验每个已配置文件
/// - 常量表为空时，仅记录 warn 日志，不阻塞加载（开发阶段）
/// - 生产部署前必须填充，否则供应链攻击无防御
pub const MODEL_SHA256: &[(ModelVariant, &str, &str)] = &[
    // (variant, filename, expected_sha256_lowercase_hex_64chars)
    // 生产部署前填充，示例：
    // (ModelVariant::BgeSmallZh, "model.safetensors", "a1b2c3d4..."),
    // (ModelVariant::BgeSmallZh, "tokenizer.json", "e5f6a7b8..."),
    // (ModelVariant::BgeSmallZh, "config.json", "..."),
    // (ModelVariant::BgeSmallZh, "tokenizer_config.json", "..."),
    // (ModelVariant::BgeSmallZh, "special_tokens_map.json", "..."),
    // (ModelVariant::BgeLargeZh, "model.safetensors", "..."),
    // (ModelVariant::BgeRerankerV2M3, "model.safetensors", "..."),
];

/// 查询模型文件的期望 SHA256
///
/// 返回 `None` 表示常量表未配置该文件的期望值（开发阶段）。
pub fn expected_sha256(variant: &ModelVariant, filename: &str) -> Option<&'static str> {
    MODEL_SHA256
        .iter()
        .find(|(v, f, _)| v == variant && *f == filename)
        .map(|(_, _, sha)| *sha)
}

/// 【S-02 P0 修复】校验模型目录下所有文件的 SHA256
///
/// - 常量表中已配置的文件：强制校验，失败返回 `Error::crypto`
/// - 常量表中未配置的文件：记录 warn 日志，不阻塞（开发阶段）
///
/// 生产部署前应确保常量表 `MODEL_SHA256` 已填充全部期望值。
pub fn verify_model_files(model_dir: &Path, variant: &ModelVariant) -> Result<()> {
    let mut configured = 0usize;
    let mut skipped = 0usize;

    for filename in variant.expected_files() {
        let path = model_dir.join(filename);
        match expected_sha256(variant, filename) {
            Some(expected) => {
                verify_sha256(&path, expected)?;
                configured += 1;
                log::debug!("SHA256 校验通过: {} / {}", variant.repo_id(), filename);
            }
            None => {
                log::warn!(
                    "SHA256 期望值未配置，跳过校验: {} / {}（生产部署前需填充 MODEL_SHA256 常量表）",
                    variant.repo_id(),
                    filename
                );
                skipped += 1;
            }
        }
    }

    log::info!(
        "模型 {} 完整性校验完成: {} 文件强制校验, {} 文件跳过（未配置期望值）",
        variant.repo_id(),
        configured,
        skipped
    );
    Ok(())
}

/// 计算模型目录下所有文件的 SHA256（用于填充常量表）
///
/// 生产部署前运行此函数获取 SHA256，填入 `MODEL_SHA256` 常量表。
/// 返回 `Vec<(filename, sha256_hex)>`。
pub fn compute_model_sha256(
    model_dir: &Path,
    variant: &ModelVariant,
) -> Result<Vec<(String, String)>> {
    let mut result = Vec::new();
    for filename in variant.expected_files() {
        let path = model_dir.join(filename);
        let bytes = std::fs::read(&path).map_err(Error::from)?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let sha = hex::encode(hasher.finalize());
        println!(
            "    (ModelVariant::{:?}, \"{}\", \"{}\"),",
            variant, filename, sha
        );
        result.push((filename.to_string(), sha));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_variant_repo_id_stable() {
        assert_eq!(ModelVariant::BgeSmallZh.repo_id(), "BAAI/bge-small-zh-v1.5");
        assert_eq!(ModelVariant::BgeLargeZh.repo_id(), "BAAI/bge-large-zh-v1.5");
        assert_eq!(
            ModelVariant::BgeRerankerV2M3.repo_id(),
            "BAAI/bge-reranker-v2-m3"
        );
    }

    #[test]
    fn model_variant_dim_correct() {
        assert_eq!(ModelVariant::BgeSmallZh.dim(), 512);
        assert_eq!(ModelVariant::BgeLargeZh.dim(), 1024);
    }

    #[test]
    fn cache_dir_under_data_dir() {
        let dir = cache_dir();
        // 路径形如 .../sparkfox/models
        assert!(
            dir.ends_with("models"),
            "cache_dir 应以 models 结尾，实际: {}",
            dir.display()
        );
        assert!(
            dir.parent()
                .map(|p| p.ends_with("sparkfox"))
                .unwrap_or(false),
            "cache_dir 父目录应为 sparkfox，实际: {}",
            dir.display()
        );
    }

    #[test]
    fn verify_sha256_rejects_mismatch() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"hello").unwrap();
        let result = verify_sha256(tmp.path(), "0000000000000000000000000000000000000000000000000000000000000000");
        assert!(result.is_err(), "不匹配的 SHA256 应返回错误");
    }

    #[test]
    fn verify_sha256_accepts_match() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"hello").unwrap();
        // echo -n "hello" | sha256sum
        let expected = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        let result = verify_sha256(tmp.path(), expected);
        assert!(result.is_ok(), "匹配的 SHA256 应通过: {:?}", result);
    }
}
