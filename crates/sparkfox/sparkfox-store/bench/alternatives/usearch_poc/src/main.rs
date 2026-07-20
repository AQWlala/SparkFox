//! usearch-rs PoC — Windows MSVC 编译与性能验证
//!
//! Sub-Step 10.15.1 GREEN 阶段：验证 usearch-rs 在 Windows MSVC 下能否编译 + 1k/10k/100k 性能。
//!
//! usearch v0.13 API 位于 `usearch::ffi` 模块下：
//! - `new_cos(dimensions, quantization, connectivity, expansion_add, expansion_search)` 创建 cosine 索引
//! - `Index::add(label: u32, vector: &[f32], thread: usize)` 插入向量
//! - `Index::search(query: &[f32], count: usize, thread: usize)` 检索最近邻
//! - `Index::reserve(capacity)` 预分配容量
//! - 返回 `Matches { count, labels: Vec<u32>, distances: Vec<f32> }`
//!
//! 运行：`cargo run --release --bin usearch_poc -- 1000`
//! 参数：向量数（默认 1000），维度固定 768

use std::env;
use std::time::Instant;

use usearch::ffi::{new_cos, Matches};

/// xorshift64 PRNG — 测试用，避免引入 rand 依赖
fn xorshift(seed: &mut u64) -> u64 {
    let mut x = *seed;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *seed = x;
    x
}

/// 生成 768 维随机向量（值域 [-1, 1]）
fn gen_vector(seed: &mut u64, dim: usize) -> Vec<f32> {
    let mut v = vec![0.0f32; dim];
    for el in v.iter_mut() {
        let r = xorshift(seed);
        *el = (r as f32 / u64::MAX as f32) * 2.0 - 1.0;
    }
    v
}

fn main() {
    let dim = 768usize;
    let n: usize = env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_000);

    println!("[usearch-rs PoC] dim={dim}, n={n}");

    // 创建 usearch cosine 索引（参数：dim, quantization="f32", connectivity=16, expansion_add=200, expansion_search=64）
    let index = match new_cos(dim, "f32", 16, 200, 64) {
        Ok(idx) => idx,
        Err(e) => {
            eprintln!("[FAIL] new_cos failed: {e:?}");
            std::process::exit(2);
        }
    };

    if let Err(e) = index.reserve(n) {
        eprintln!("[FAIL] reserve({n}) failed: {e:?}");
        std::process::exit(3);
    }

    // 插入 n 个向量（label = 0..n as u32）
    let mut seed: u64 = 0xDEAD_BEEF_CAFE;
    let t0 = Instant::now();
    for i in 0..n as u32 {
        let v = gen_vector(&mut seed, dim);
        if let Err(e) = index.add(i, &v) {
            eprintln!("[FAIL] add({i}) failed: {e:?}");
            std::process::exit(4);
        }
    }
    let insert_ms = t0.elapsed().as_millis();
    println!("[OK] insert {n} vectors: {insert_ms} ms");

    // 单次查询 top-10
    let query = gen_vector(&mut seed, dim);
    let t1 = Instant::now();
    let matches: Matches = match index.search(&query, 10) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[FAIL] search failed: {e:?}");
            std::process::exit(5);
        }
    };
    let query_ms = t1.elapsed().as_millis();
    println!("[OK] search top-10: {query_ms} ms, hits={}", matches.count);

    // 100 次查询取平均
    let t2 = Instant::now();
    for _ in 0..100 {
        let q = gen_vector(&mut seed, dim);
        let _ = index.search(&q, 10);
    }
    let avg_query_ms = t2.elapsed().as_millis() as f64 / 100.0;
    println!("[OK] avg query (100x): {avg_query_ms:.3} ms");

    // 内存占用（粗略估计：n * dim * 4 字节 + 图结构 ~10%）
    let mem_mb = (n * dim * 4) as f64 / 1024.0 / 1024.0;
    println!("[INFO] estimated raw vector memory: {mem_mb:.1} MB");

    println!("[DONE] usearch-rs PoC complete");
}
