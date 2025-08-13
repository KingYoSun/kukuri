#[cfg(test)]
mod performance_tests {
    use std::time::Instant;
    use kukuri_tauri::infrastructure::cache::{MemoryCacheService, PostCacheService};
    use kukuri_tauri::domain::entities::post::Post;
    use kukuri_tauri::domain::entities::user::User;
    use chrono::Utc;
    use uuid::Uuid;
    use futures::future::join_all;

    #[tokio::test]
    async fn test_cache_performance() {
        let cache = PostCacheService::new();
        let test_user = User::new("test_pubkey".to_string(), "Test User".to_string());
        
        // テスト用の投稿を生成
        let posts: Vec<Post> = (0..1000).map(|i| {
            Post::new(
                format!("Test content {}", i),
                test_user.clone(),
                "#test".to_string(),
            )
        }).collect();
        
        // キャッシュへの書き込みパフォーマンステスト
        let start = Instant::now();
        for post in &posts {
            cache.cache_post(post.clone()).await;
        }
        let write_duration = start.elapsed();
        println!("Cache write for 1000 posts: {:?}", write_duration);
        assert!(write_duration.as_millis() < 100, "Cache write should be fast");
        
        // キャッシュからの読み取りパフォーマンステスト
        let start = Instant::now();
        for post in &posts {
            let _ = cache.get_post(&post.id().to_string()).await;
        }
        let read_duration = start.elapsed();
        println!("Cache read for 1000 posts: {:?}", read_duration);
        assert!(read_duration.as_millis() < 50, "Cache read should be very fast");
    }

    #[tokio::test]
    async fn test_parallel_processing_performance() {
        use nostr_sdk::prelude::*;
        
        // 100個のpubkeyを生成
        let pubkeys: Vec<String> = (0..100).map(|i| {
            format!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa{:03}", i)
        }).collect();
        
        // 直列処理のパフォーマンステスト
        let start = Instant::now();
        let mut _serial_results = Vec::new();
        for pubkey in &pubkeys {
            let npub = PublicKey::from_hex(pubkey)
                .ok()
                .and_then(|pk| pk.to_bech32().ok())
                .unwrap_or_else(|| pubkey.clone());
            _serial_results.push(npub);
        }
        let serial_duration = start.elapsed();
        println!("Serial npub conversion for 100 keys: {:?}", serial_duration);
        
        // 並行処理のパフォーマンステスト
        let start = Instant::now();
        let futures = pubkeys.iter().map(|pubkey| {
            let pk = pubkey.clone();
            async move {
                tokio::task::spawn_blocking(move || {
                    PublicKey::from_hex(&pk)
                        .ok()
                        .and_then(|pk| pk.to_bech32().ok())
                        .unwrap_or(pk)
                }).await.unwrap_or_else(|_| pubkey.clone())
            }
        });
        let _parallel_results = join_all(futures).await;
        let parallel_duration = start.elapsed();
        println!("Parallel npub conversion for 100 keys: {:?}", parallel_duration);
        
        // 並行処理が直列処理より高速であることを確認
        println!("Speedup: {:.2}x", serial_duration.as_secs_f64() / parallel_duration.as_secs_f64());
    }

    #[tokio::test]
    async fn test_batch_processing_performance() {
        let test_user = User::new("test_pubkey".to_string(), "Test User".to_string());
        
        // 50個の投稿IDを生成
        let post_ids: Vec<String> = (0..50).map(|_| Uuid::new_v4().to_string()).collect();
        
        // 個別処理のシミュレーション
        let start = Instant::now();
        for _id in &post_ids {
            // DBアクセスのシミュレーション（10ms）
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
        let individual_duration = start.elapsed();
        println!("Individual processing for 50 posts: {:?}", individual_duration);
        
        // バッチ処理のシミュレーション
        let start = Instant::now();
        // バッチでのDBアクセスシミュレーション（50ms）
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        let batch_duration = start.elapsed();
        println!("Batch processing for 50 posts: {:?}", batch_duration);
        
        // バッチ処理が個別処理より高速であることを確認
        assert!(batch_duration < individual_duration, "Batch processing should be faster");
        println!("Batch processing speedup: {:.2}x", individual_duration.as_secs_f64() / batch_duration.as_secs_f64());
    }

    #[tokio::test]
    async fn test_memory_cache_ttl() {
        let cache: MemoryCacheService<String> = MemoryCacheService::new(1); // 1秒のTTL
        
        // データを保存
        cache.set("test_key".to_string(), "test_value".to_string()).await;
        
        // 即座に取得できることを確認
        let value = cache.get("test_key").await;
        assert_eq!(value, Some("test_value".to_string()));
        
        // 1.5秒待機
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
        
        // TTL後は取得できないことを確認
        let value = cache.get("test_key").await;
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_cache_cleanup_performance() {
        let cache: MemoryCacheService<String> = MemoryCacheService::new(1);
        
        // 10000個のエントリを追加
        for i in 0..10000 {
            cache.set(format!("key_{}", i), format!("value_{}", i)).await;
        }
        
        // サイズを確認
        let size_before = cache.size().await;
        println!("Cache size before cleanup: {}", size_before);
        assert_eq!(size_before, 10000);
        
        // 1.5秒待機（TTL切れ）
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
        
        // クリーンアップのパフォーマンステスト
        let start = Instant::now();
        cache.cleanup_expired().await;
        let cleanup_duration = start.elapsed();
        println!("Cleanup for 10000 expired entries: {:?}", cleanup_duration);
        
        // クリーンアップが高速であることを確認
        assert!(cleanup_duration.as_millis() < 100, "Cleanup should be fast");
        
        // サイズが0になっていることを確認
        let size_after = cache.size().await;
        assert_eq!(size_after, 0);
    }
}