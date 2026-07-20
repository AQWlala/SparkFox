//! QueryCache 集成测试 — 与真实 Store（SQLite）交互
//!
//! 验证：init_schema / put / get / 维度校验 / clear / count

#![forbid(unsafe_code)]

use sparkfox_embedding::QueryCache;
use sparkfox_store::{Store, StoreConfig};

fn open_test_store() -> Store {
    let tmp = tempfile::NamedTempFile::new().expect("创建临时文件失败");
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp); // 让文件存在到测试结束
    let mut cfg = StoreConfig::for_path(&path);
    cfg.enable_vec = false; // 测试不需要向量功能
    Store::open(cfg).expect("打开 Store 失败")
}

#[test]
fn cache_init_schema_idempotent() {
    let store = open_test_store();
    let cache = QueryCache::new(&store);
    cache.init_schema().expect("首次 init_schema");
    cache.init_schema().expect("二次 init_schema（应幂等）");
    assert_eq!(cache.count().unwrap(), 0);
}

#[test]
fn cache_put_get_hit() {
    let store = open_test_store();
    let cache = QueryCache::new(&store);
    cache.init_schema().unwrap();

    let emb = vec![0.1f32, 0.2, 0.3, 0.4, 0.5];
    cache
        .put("你好世界", "BAAI/bge-small-zh-v1.5", &emb)
        .expect("put");

    let got = cache
        .get("你好世界", "BAAI/bge-small-zh-v1.5")
        .expect("get");
    assert!(got.is_some(), "应命中缓存");
    let got = got.unwrap();
    assert_eq!(got.len(), emb.len());
    for (a, b) in emb.iter().zip(got.iter()) {
        assert_eq!(a.to_bits(), b.to_bits(), "f32 bit 不一致");
    }
}

#[test]
fn cache_get_miss() {
    let store = open_test_store();
    let cache = QueryCache::new(&store);
    cache.init_schema().unwrap();

    let got = cache.get("不存在的查询", "BAAI/bge-small-zh-v1.5").unwrap();
    assert!(got.is_none(), "未写入的查询应未命中");
}

#[test]
fn cache_model_isolation() {
    let store = open_test_store();
    let cache = QueryCache::new(&store);
    cache.init_schema().unwrap();

    let emb_small = vec![1.0f32; 512];
    let emb_large = vec![2.0f32; 1024];

    cache.put("同一查询", "bge-small-zh", &emb_small).unwrap();
    cache.put("同一查询", "bge-large-zh", &emb_large).unwrap();

    let got_small = cache.get("同一查询", "bge-small-zh").unwrap().unwrap();
    let got_large = cache.get("同一查询", "bge-large-zh").unwrap().unwrap();

    assert_eq!(got_small.len(), 512);
    assert_eq!(got_large.len(), 1024);
    assert_eq!(got_small[0], 1.0);
    assert_eq!(got_large[0], 2.0);
}

#[test]
fn cache_dim_mismatch_treated_as_miss() {
    let store = open_test_store();
    let cache = QueryCache::new(&store);
    cache.init_schema().unwrap();

    let emb = vec![1.0f32; 512];
    cache.put("测试", "model-A", &emb).unwrap();

    // 直接改 dim 字段模拟脏数据
    store
        .conn()
        .execute(
            "UPDATE query_embedding_cache SET dim=1024 WHERE content_hash=?",
            rusqlite::params![sha256_hex("测试")],
        )
        .unwrap();

    let got = cache.get("测试", "model-A").unwrap();
    assert!(got.is_none(), "维度不匹配应视为未命中");
}

#[test]
fn cache_clear_and_count() {
    let store = open_test_store();
    let cache = QueryCache::new(&store);
    cache.init_schema().unwrap();

    cache.put("q1", "m", &[0.1]).unwrap();
    cache.put("q2", "m", &[0.2]).unwrap();
    cache.put("q3", "m", &[0.3]).unwrap();
    assert_eq!(cache.count().unwrap(), 3);

    cache.clear().unwrap();
    assert_eq!(cache.count().unwrap(), 0);
}

#[test]
fn cache_put_replaces_existing() {
    let store = open_test_store();
    let cache = QueryCache::new(&store);
    cache.init_schema().unwrap();

    let emb1 = vec![0.1f32, 0.2];
    let emb2 = vec![0.9f32, 0.8];

    cache.put("同一查询", "m", &emb1).unwrap();
    cache.put("同一查询", "m", &emb2).unwrap(); // REPLACE

    assert_eq!(cache.count().unwrap(), 1, "INSERT OR REPLACE 应覆盖");
    let got = cache.get("同一查询", "m").unwrap().unwrap();
    assert_eq!(got[0].to_bits(), 0.9f32.to_bits());
}

fn sha256_hex(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    hex::encode(h.finalize())
}
