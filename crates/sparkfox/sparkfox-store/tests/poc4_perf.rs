//! PoC-4 性能基线测试 — SparkFox + sqlite-vec 性能验证
#![forbid(unsafe_code)]

use std::time::Instant;

use sparkfox_memory::MemoryLayer;
use sparkfox_store::{Store, StoreConfig};

#[test]
fn poc4_cold_start_under_3s() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let t = Instant::now();
    let _store = Store::open(StoreConfig::for_path(tmp.path())).expect("打开 Store");
    let elapsed = t.elapsed();
    assert!(elapsed.as_secs_f64() < 3.0, "冷启动 {elapsed:?} 超过 3s 门槛");
}

#[test]
fn poc4_100k_vector_search_under_800ms() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let store = Store::open(StoreConfig::for_path(tmp.path())).unwrap();

    // 插入 10 万条 768 维向量（bge-large-zh 维度）
    let dim = 768;
    let total = 100_000;
    let batch = 1000;
    let mut rng_state: u64 = 0xDEAD_BEEF_CAFE;
    for i in 0..total {
        let mut v = vec![0.0f32; dim];
        for j in 0..dim {
            rng_state ^= rng_state << 13;
            rng_state ^= rng_state >> 7;
            rng_state ^= rng_state << 17;
            v[j] = (rng_state as f32 / u64::MAX as f32) * 2.0 - 1.0;
        }
        store.vector_insert(MemoryLayer::L0Raw, &i.to_string(), "bge-large-zh", &v).unwrap();
        if i % batch == 0 {
            store.vector_flush().unwrap();
        }
    }
    store.vector_flush().unwrap();

    // 检索延迟
    let query = vec![0.5f32; dim];
    let t = Instant::now();
    let hits = store.vector_search(MemoryLayer::L0Raw, &query, 10).unwrap();
    let elapsed = t.elapsed();
    assert_eq!(hits.len(), 10, "必须返回 top-10");
    assert!(elapsed.as_millis() < 800, "10 万向量检索 {elapsed:?} 超过 800ms");
}

#[test]
fn poc4_schema_migrate_idempotent() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let cfg = StoreConfig::for_path(tmp.path());
    let s1 = Store::open(cfg.clone()).unwrap();
    s1.migrate().unwrap();
    drop(s1);
    let s2 = Store::open(cfg).unwrap();
    s2.migrate().unwrap(); // 幂等
}
