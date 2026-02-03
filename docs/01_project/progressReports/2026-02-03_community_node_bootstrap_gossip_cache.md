# Community Node bootstrap Gossip/DHT キャッシュ統合

- 39000/39001 を P2P 受信時にキャッシュへ取り込み、stale フラグで HTTP 再取得を促す経路を追加
- bootstrap nodes/services の取得でキャッシュと HTTP 再取得（`next_refresh_at`）を統合し、addressable 置換ルールで整理
- CommunityNodeHandler のキャッシュ取り込み/取得テストを追加
