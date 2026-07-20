//! instant-distance PoC — Windows MSVC 编译与性能验证
//!
//! Sub-Step 10.15.1 GREEN 阶段：验证 instant-distance 在 Windows MSVC 下能否编译 + 1k/10k/100k 性能。
//!
//! 注意：instant-distance 0.6 API 要求 Builder::build(points) 一次性建图，
//! 不支持增量 insert。本 PoC 模拟"批量建图 + 查询"场景以验证性能与编译。
//!
//! 运行：`cargo run --release --bin instant_distance_poc -- 1000`
//! 参数：向量数（默认 1000），维度固定 768

use std::env;
use std::time::Instant;

use instant_distance::{Builder, HnswMap, Point, Search};

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

/// 适配 instant-distance 的 Point trait
#[derive(Clone)]
struct VecPoint(Vec<f32>);

impl Point for VecPoint {
    fn distance(&self, other: &Self) -> f32 {
        // cosine distance = 1 - cosine similarity
        let a = &self.0;
        let b = &other.0;
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if na == 0.0 || nb == 0.0 {
            return 1.0;
        }
        1.0 - (dot / (na * nb))
    }
}

fn main() {
    let dim = 768usize;
    let n: usize = env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_000);

    println!("[instant-distance PoC] dim={dim}, n={n}");

    // 生成 n 个向量 + 配套 value（用索引作为 value）
    let mut seed: u64 = 0xDEAD_BEEF_CAFE;
    let mut points = Vec::with_capacity(n);
    let mut values = Vec::with_capacity(n);
    for i in 0..n {
        points.push(VecPoint(gen_vector(&mut seed, dim)));
        values.push(i as u64);
    }

    // 一次性建图（instant-distance API 限制：不支持增量 insert）
    // Builder::build(points, values) -> HnswMap<P, V>
    let t0 = Instant::now();
    let builder = Builder::default();
    let hnsw: HnswMap<VecPoint, u64> = builder.build(points, values);
    let build_ms = t0.elapsed().as_millis();
    println!("[OK] build HnswMap (n={n}): {build_ms} ms");

    // 单次查询 top-10
    let query = VecPoint(gen_vector(&mut seed, dim));
    let mut search = Search::default();
    let t1 = Instant::now();
    let mut hits = 0usize;
    for _item in hnsw.search(&query, &mut search) {
        hits += 1;
        if hits >= 10 {
            break;
        }
    }
    let query_ms = t1.elapsed().as_millis();
    println!("[OK] search top-10: {query_ms} ms, hits={hits}");

    // 100 次查询取平均
    let t2 = Instant::now();
    for _ in 0..100 {
        let q = VecPoint(gen_vector(&mut seed, dim));
        let mut s = Search::default();
        let _ = hnsw.search(&q, &mut s);
    }
    let avg_query_ms = t2.elapsed().as_millis() as f64 / 100.0;
    println!("[OK] avg query (100x): {avg_query_ms:.3} ms");

    // 内存占用（粗略估计：n * dim * 4 字节 + 图结构）
    let mem_mb = (n * dim * 4) as f64 / 1024.0 / 1024.0;
    println!("[INFO] estimated raw vector memory: {mem_mb:.1} MB");

    println!("[DONE] instant-distance PoC complete");
}
