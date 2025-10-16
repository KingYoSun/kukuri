use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use tokio::runtime::Runtime;

fn benchmark_single_vs_batch_posts(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("post_retrieval");

    // 10, 50, 100個の投稿IDでテスト
    for size in [10, 50, 100].iter() {
        let post_ids: Vec<String> = (0..*size).map(|i| format!("post_{}", i)).collect();

        group.bench_with_input(BenchmarkId::new("individual", size), size, |b, _| {
            b.to_async(&rt).iter(|| async {
                // 個別取得のシミュレーション
                for id in &post_ids {
                    // DBアクセスのシミュレーション
                    tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;
                }
            });
        });

        group.bench_with_input(BenchmarkId::new("batch", size), size, |b, _| {
            b.to_async(&rt).iter(|| async {
                // バッチ取得のシミュレーション
                tokio::time::sleep(tokio::time::Duration::from_micros(200)).await;
            });
        });
    }

    group.finish();
}

fn benchmark_cache_vs_no_cache(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("cache_performance");

    group.bench_function("without_cache", |b| {
        b.to_async(&rt).iter(|| async {
            // DBアクセスのシミュレーション
            tokio::time::sleep(tokio::time::Duration::from_micros(500)).await;
            black_box("post_content");
        });
    });

    group.bench_function("with_cache_miss", |b| {
        b.to_async(&rt).iter(|| async {
            // キャッシュチェック + DBアクセス
            tokio::time::sleep(tokio::time::Duration::from_micros(10)).await; // キャッシュチェック
            tokio::time::sleep(tokio::time::Duration::from_micros(500)).await; // DBアクセス
            black_box("post_content");
        });
    });

    group.bench_function("with_cache_hit", |b| {
        b.to_async(&rt).iter(|| async {
            // キャッシュヒット（DBアクセスなし）
            tokio::time::sleep(tokio::time::Duration::from_micros(10)).await;
            black_box("cached_post_content");
        });
    });

    group.finish();
}

fn benchmark_parallel_npub_conversion(c: &mut Criterion) {
    use nostr_sdk::prelude::*;
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("npub_conversion");

    let pubkeys: Vec<String> = (0..100)
        .map(|i| {
            format!(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa{:03}",
                i
            )
        })
        .collect();

    group.bench_function("serial", |b| {
        b.iter(|| {
            for pubkey in &pubkeys {
                let _ = PublicKey::from_hex(pubkey)
                    .ok()
                    .and_then(|pk| pk.to_bech32().ok())
                    .unwrap_or_else(|| pubkey.clone());
            }
        });
    });

    group.bench_function("parallel", |b| {
        b.to_async(&rt).iter(|| async {
            use futures::future::join_all;

            let futures = pubkeys.iter().map(|pubkey| {
                let pk = pubkey.clone();
                async move {
                    tokio::task::spawn_blocking(move || {
                        PublicKey::from_hex(&pk)
                            .ok()
                            .and_then(|pk| pk.to_bech32().ok())
                            .unwrap_or(pk)
                    })
                    .await
                    .unwrap_or_else(|_| pubkey.clone())
                }
            });

            let _ = join_all(futures).await;
        });
    });

    group.finish();
}

fn benchmark_handler_reuse(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("handler_initialization");

    group.bench_function("new_handler_each_time", |b| {
        b.to_async(&rt).iter(|| async {
            // ハンドラーを毎回生成（最適化前）
            tokio::time::sleep(tokio::time::Duration::from_micros(50)).await;
            black_box("handler_created");
        });
    });

    group.bench_function("reused_handler", |b| {
        b.to_async(&rt).iter(|| async {
            // ハンドラーを再利用（最適化後）
            tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
            black_box("handler_reused");
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_single_vs_batch_posts,
    benchmark_cache_vs_no_cache,
    benchmark_parallel_npub_conversion,
    benchmark_handler_reuse
);
criterion_main!(benches);
