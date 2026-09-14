# 2026-09-14 旧公開topicのローカルレプリカ削除

本番の旧 `demo / iroh / nostr / operators` 専用のオフライン保守補助。
ネットワークへ接続せず、既存の停止済み `docs.redb` のコピーから新しい出力fileを作る。
任意topicを削除する汎用commandではない。稼働中のストアを直接入力しない。

```sh
cargo test --locked --manifest-path scripts/community-node/retire-legacy-docs/Cargo.toml
cargo run --locked --manifest-path scripts/community-node/retire-legacy-docs/Cargo.toml -- inspect stopped-copy.redb preview.redb
cargo run --locked --manifest-path scripts/community-node/retire-legacy-docs/Cargo.toml -- apply stopped-copy.redb retired.redb
```

出力は新規fileに限る。inspectもコピー上でのみStoreを開く。applyは公式iroh-docs 0.101.0
の `remove_replica` を使い、対象外SignedEntry（署名・tombstone込み）と署名者鍵の保持、
原本hash不変、出力の再openを検証する。復旧用コピーとinstance identityは別に保持する。

管理画面の索引対象解除、Postgres/ArcadeDBの対象限定削除、indexer停止・反映・再開は
この補助の外側で行う。本番へ戻す前に出力hashと原本hashを確認する。
DB/volumeの丸ごと削除、topicの恒久禁止、他peerへの削除命令は行わない。
監査・実施証拠は [作業記録](../../../docs/progress/2026-09-14-retire-legacy-community-topics.md)。

同じtopic IDの再利用は同じP2P名前空間への再参加であり、他peerからの旧履歴再流入は防げない。
新しい履歴には新しいtopic IDを使う。
