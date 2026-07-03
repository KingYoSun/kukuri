# IPC codegen spike — ts-rs 評価と WP-H7 の判断(WP-S4 T3)

- 日付: 2026-07-03
- 目的: types.ts の生成物化(リファクタリング マスタープラン WP-H7)の実装方式判断。
- 実験: ts-rs 12.0.1 を app-api の dev-dependency として使い捨てブランチ
  `spike/s4-ts-rs-eval` で実測(プロダクションコード無変更。ブランチは merge しない)。
- 前提: WP-S4 T2(#448)で高頻度 8 型グループの fixture contract が導入済みであり、
  silent break の安全網はすでに存在する。H7 の価値は「手動ミラーの保守コスト削減」。

## 評価規準と実測結果

| 規準 | 結果 | 実測 |
|---|---|---|
| (a) `serialize_with`(core::Profile.picture_asset)の override | **可** | `#[ts(type = "string \| null")]` がフィールド単位で効く(出力実証済み) |
| (b) `[i64; 3]` のタプル出力 | **可** | `[number, number, number]`(`array_tuple_limit` 既定 64 以内) |
| (c) u64 / i64 の扱い | **可(要設定)** | 既定は `bigint`。**JSON-over-IPC の実行時値は JS number のため既定のままでは不適**。`Config::with_large_int("number")`(env: `TS_RS_LARGE_INT`)で一括解決できることを実証。2^53 超の精度問題(seq / bytes / world_version)は残るが現状の手書き型と同等 |
| (d) derive / feature 伝播コスト | **中〜高** | views.rs 到達型は core / store / transport の約 40+ 型。`derive(TS)` の追加が凍結境界隣接ファイルに広範囲に及ぶ。feature gate(例: `ts` feature、codegen 時のみ有効)で production ビルドへの影響は回避可能 |
| (e) 生成出力 vs 手書き types.ts | **層構造が必要** | 生成は `field: T \| null`(手書きの `?: T \| null` より wire に忠実で厳密 — 移行時に消費側の undefined 前提を修正)。front 専用フィールド(PostView の local_* 6 個)は `Generated & LocalFields` の交差型レイヤで維持。`ProfileAssetView.role` 等の literal 型は `#[ts(type)]` の個別指定 |

補助所見:

- unit enum は serde の rename 有無(PascalCase / snake_case 混在)をそのまま string union に写す(実証)。
- tag 付き enum・transparent newtype は ts-rs が serde 属性を解釈するため対応可能(一般仕様。全面適用時に要スポット確認)。
- specta / tauri-specta は command 定義の生成まで踏み込む(bindings 生成の枠組みが大きい)。
  types.ts の生成だけが目的なら ts-rs の方が導入面積が小さい。

## 判断: WP-H7 は「条件付き go(ts-rs 案)」・優先度は下げる

- **技術的には成立する**: 主要な懸念(bigint / serialize_with / タプル)は全て解決策を実証済み。
- **ただし急がない**: T2 の fixture contract により silent break は既に CI で検出される。
  H7 が解決するのは保守コスト(手動ミラーの同期作業)のみで、安全性は現状で確保済み。
- 実施時の骨子: (1) `TS_RS_LARGE_INT=number` を固定 (2) 対象 crate に feature-gated derive を追加
  (3) 生成型 + `LocalPostFields` 等の交差型で types.ts を再構成(import パスは維持)
  (4) T2 の fixture contract を「生成器の回帰テスト」として存続させる。
- 再評価トリガ: types.ts の手動同期ミス起因の不具合が実際に発生した場合、
  または views 型の追加頻度が上がった場合に優先度を上げる。

## 実験の再現

`spike/s4-ts-rs-eval` ブランチの `crates/app-api/src/tests/ts_rs_spike.rs`
(`cargo test -p kukuri-app-api ts_rs_spike -- --nocapture --ignored`)。
