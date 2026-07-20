#![forbid(unsafe_code)]
//! CLIP 图片嵌入 — candle-transformers 占位
//!
//! v1.0.0：占位实现，返回空向量（不引入真实 CLIP 模型依赖）
//! v1.1.0+：集成 candle-transformers CLIP 模型，支持图文检索
//!
//! 用途：图片+文本对齐，支持图文混合检索（文本查图、图查图、图查文本）
//!
//! NOTICE: CLIP 模型 MIT License（OpenAI），candle-transformers MIT

use sparkfox_core::Result;

/// CLIP 嵌入维度（v1.0.0 占位常量，对应 openai/clip-vit-base-patch32）
const CLIP_DIM_V1: usize = 512;

/// CLIP 嵌入器（v1.0.0 占位）
///
/// v1.0.0 不依赖 candle-transformers 的 CLIP 模型，所有 embed_* 返回空 Vec；
/// v1.1.0+ 计划加载 `openai/clip-vit-base-patch32`（或社区中文 CLIP 变体），
/// 实现 `embed_image` / `embed_text` 的真实推理。
///
/// 上层（sparkfox-knowledge）调用前应先通过 [`ClipEmbedder::is_available`]
/// 检查是否可用，避免在 v1.0.0 误用空向量污染向量库。
pub struct ClipEmbedder {
    /// 嵌入维度（v1.0.0 永远为 512）
    dim: usize,
}

impl ClipEmbedder {
    /// 加载 CLIP 模型
    ///
    /// v1.0.0 占位：不实际加载模型，仅返回带默认维度的实例
    /// v1.1.0+ 从 HF Hub 或本地缓存加载 CLIP 权重
    pub fn load() -> Result<Self> {
        Ok(Self {
            dim: CLIP_DIM_V1,
        })
    }

    /// 图片嵌入
    ///
    /// v1.0.0 占位：返回空向量（调用方应先通过 [`is_available`](Self::is_available) 检查）
    /// v1.1.0+ 加载真实 CLIP 视觉编码器，输出 L2 归一化后的 512 维向量
    ///
    /// # 参数
    /// - `_path`：图片文件路径（v1.0.0 未使用）
    pub fn embed_image(&self, _path: &str) -> Result<Vec<f32>> {
        Ok(vec![])
    }

    /// 文本嵌入
    ///
    /// v1.0.0 占位：返回空向量
    /// v1.1.0+ 加载真实 CLIP 文本编码器，输出 L2 归一化后的 512 维向量
    ///
    /// # 参数
    /// - `_text`：待编码文本（v1.0.0 未使用）
    pub fn embed_text(&self, _text: &str) -> Result<Vec<f32>> {
        Ok(vec![])
    }

    /// 嵌入维度
    ///
    /// v1.0.0 永远返回 512（clip-vit-base-patch32 的标准维度）
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// 检测 CLIP 模型是否可用
    ///
    /// v1.0.0 永远返回 `false`（未集成真实模型）
    /// v1.1.0+ 检测模型权重是否已下载 + candle-transformers CLIP 是否可加载
    pub fn is_available(&self) -> bool {
        false
    }
}

impl Default for ClipEmbedder {
    fn default() -> Self {
        // default 委托给 load，保持维度一致；load 在 v1.0.0 不会失败
        Self::load().expect("ClipEmbedder::load 在 v1.0.0 占位实现中不应失败")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_returns_ok() {
        let result = ClipEmbedder::load();
        assert!(result.is_ok(), "v1.0.0 占位 load 必须返回 Ok");
    }

    #[test]
    fn test_dim_is_512() {
        let clip = ClipEmbedder::load().expect("load 应成功");
        assert_eq!(clip.dim(), CLIP_DIM_V1);
        assert_eq!(clip.dim(), 512, "v1.0.0 占位 CLIP 维度应为 512");
    }

    #[test]
    fn test_is_available_v1_returns_false() {
        let clip = ClipEmbedder::load().expect("load 应成功");
        assert!(
            !clip.is_available(),
            "v1.0.0 占位实现 is_available 必须为 false"
        );
    }

    #[test]
    fn test_embed_image_returns_empty() {
        let clip = ClipEmbedder::load().expect("load 应成功");
        let result = clip.embed_image("dummy.png").expect("占位 embed_image 应返回 Ok");
        assert!(
            result.is_empty(),
            "v1.0.0 占位 embed_image 必须返回空 Vec"
        );
    }

    #[test]
    fn test_embed_text_returns_empty() {
        let clip = ClipEmbedder::load().expect("load 应成功");
        let result = clip.embed_text("一只猫").expect("占位 embed_text 应返回 Ok");
        assert!(
            result.is_empty(),
            "v1.0.0 占位 embed_text 必须返回空 Vec"
        );
    }

    #[test]
    fn test_default_eq_load() {
        let a = ClipEmbedder::load().expect("load 应成功");
        let b = ClipEmbedder::default();
        assert_eq!(a.dim(), b.dim());
        assert_eq!(a.is_available(), b.is_available());
    }
}
