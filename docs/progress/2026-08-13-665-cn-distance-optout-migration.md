# Issue #665 Community Node distance opt-out 移行

最終更新日: 2026-08-13

## 完了した範囲

- 既存の`cn_trust.relation_optouts`と`PUT/DELETE /v1/relation/optout`を維持し、保存済み行を
  distance opt-out選択として再解釈した。破壊的DB migrationはない。
- `(viewerまたはtargetが選択済み) AND (proximityがnode policy未満または未観測)`のときだけ、
  当該nodeのuser / post surfacingを相互に抑制する共通判定へ移行した。
- client-facing relation read、neighbors、index search、discovery、recommendationsへ同じ判定を
  適用した。trust read、relation graph、内部pairwise計算は変更していない。
- 本人向け`GET /v1/relation/optout`を追加し、GET / PUT / DELETEで選択状態、設定時刻、
  `min_proximity`を同じ共有wire型で返すようにした。
- `COMMUNITY_NODE_RELATION_DISTANCE_OPTOUT_MIN_PROXIMITY`を追加した。index queryまたは
  trust/relation readを有効にするnodeでは`(0, 1]`の明示設定を必須とし、composeとGCP
  Terraform配布経路へ伝播した。

## 固定した振る舞い

- 双方とも未選択なら、距離だけで自動抑制しない。
- 片方が選択済みでも、node policy以上の近距離pairは抑制しない。
- 遠距離pairはviewer側・target/author側のどちらが選択しても相互に抑制する。
- pairwise proximityが未観測なら、選択済みpairについては距離境界外として扱う。
- DBまたは必要なrelation判定が失敗したsurfaceは可視扱いへfallbackしない。
- DELETE後はrelation graphを再構築せず直ちに表示が復帰する。
- distance opt-outはprivacy、block、graph離脱、P2P・別node・network全体での不可視性ではない。

## 検証

- `cargo test -p kukuri-cn-protocol`
- `cargo test -p kukuri-cn-core --test relation_optouts`
- `cargo test -p kukuri-cn-user-api --test trust_relation --test index_query`
- `cargo test -p kukuri-cn-user-api --lib`
- `cargo xtask cn-test`
- `cargo xtask cn-check`
- `cargo xtask check`
- `terraform -chdir=infra/terraform/envs/low-cost validate`
- `cargo xtask test`は、変更対象外の既存`kukuri-docs-sync` relayテスト2件が90秒timeoutで失敗した。
  同じ1件を個別再実行してもtimeoutを再現した。該当2件だけを除いたnextest 550件、serial
  harness 18件、doctest、desktop Vitest 633件はすべて成功した。

## 後続

desktop-runtime、Tauri command、desktop UIからの状態取得・設定導線はIssue #665のclient側移行として
別途実装する。
