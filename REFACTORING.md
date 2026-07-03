# REFACTORING.md

この文書は、AIエージェントと人間が kukuri でリファクタリング作業を行うときのルールを定義する。

このリポジトリにおけるリファクタリングとは、タスクで明示されない限り、外部挙動、プロトコル契約、ストレージ意味論、ユーザーに見える挙動を維持したまま内部構造を改善する作業である。

## リファクタリングモード

リファクタリング作業では、機能追加・仕様変更・依存更新を混ぜない。

リファクタリングとは、外部挙動を維持したまま、内部構造・命名・責務境界・重複・ファイル構成を改善する作業である。

外部挙動を変更する場合は、それは refactor ではなく feature / fix / migration として扱う。

## 非目的

- リファクタリング作業を使ってプロダクト挙動を追加しない。
- タスクで明示されない限り、CI の必須 / 任意 job 設定を変更しない。
- ついでの整理として crate 分割や大型ファイル分割を行わない。

## 基本原則

- 1 PR = 1意図。
- rename / move / extraction / behavior change を同じ PR に混ぜない。
- 明示指示なしに public API、protocol object、storage schema、docs/blobs canonical source、community-node endpoint contract を変更しない(具体的な対象は「凍結境界」の章)。
- 既存テストを削除しない。削除が必要な場合は、削除理由と代替カバレッジを示す。
- 振る舞いが変わる可能性がある場合は、先に characterization test / contract / scenario を追加する。
- 大きな抽象化を導入する前に、現在の責務境界と呼び出し方向を調査する。
- 便利な境界横断の共通化より、明確な境界を保つことを優先する。

## PR種別

PRタイトルまたはタスク概要では、以下のラベル / prefix のいずれかを使う。

- `refactor:rename`: 名前変更のみ。ロジック変更禁止。
- `refactor:move`: ファイル移動のみ。ロジック変更禁止。
- `refactor:extract`: 関数・型・モジュール抽出。外部挙動変更禁止。
- `refactor:boundary`: crate / module 境界整理。先に計画を書く。
- `refactor:delete`: dead code 削除。参照経路調査を添える。
- `contract`: 仕様固定テスト追加。実装変更禁止。
- `scenario`: harness scenario 追加/更新。プロダクト実装変更は別PR。
- `fix`: バグ修正。先に failing test / contract / scenario を置く。
- `deps`: 依存更新。リファクタリングと混ぜない。
- `docs`: ドキュメント更新。実装変更と混ぜる場合は理由を書く。

PR を作成または説明するときは、タスク種別を明確にする。リファクタリングPRに機能追加や挙動変更を隠さない。

推奨 PRタイトル prefix:

```text
[codex][refactor:rename] ...
[codex][refactor:extract] ...
[codex][refactor:boundary] ...
[codex][refactor:delete] ...
[codex][contract] ...
[codex][scenario] ...
[codex][fix] ...
[codex][deps] ...
[codex][docs] ...
```

推奨 PR 本文項目:

```md
## 概要

## 種別

## 挙動変更

## 検証

## リスク

## 後続対応
```

## 禁止する混在変更

以下を同じPRに混ぜない。

- rename + logic change
- file move + behavior change
- dependency update + refactor
- storage migration + UI refactor
- protocol shape change + internal cleanup
- test deletion + implementation change without replacement
- formatting-only change + semantic change

## 凍結境界(変更 = 挙動破壊)

以下は「1 バイトの変更で既存データ・既存署名・ネットワーク互換・契約が壊れる」凍結対象である。
明示的なタスク(migration 計画込み)なしに変更しない。多くは contract テストが検出装置として
固定している(fail した場合、テストではなく変更側の互換影響を評価する)。
詳細な調査記録は `.claude/plans/2026-07-02-refactoring_master_plan.md` §3.1。

1. **署名 canonical 3 系統**: envelope の 6 要素配列(`crates/core/src/envelope.rs` の
   `canonical_envelope_payload`)、DM frame/ack の canonical 配列(`crates/core/src/direct_messages.rs`)、
   moderation event のキー辞書順 canonical(`crates/cn-safety/src/event.rs`)。
   固定: `crates/core/src/tests/signing_canonical.rs` / `crates/cn-safety/tests/domain_model.rs` /
   `crates/cn-safety-runtime/tests/signer.rs`。
2. **rendezvous / 派生文字列**: replica 命名と `kukuri-docs:` 派生(`crates/docs-sync/src/replicas.rs`)、
   gossip topic id(`crates/transport/src/iroh/topics.rs` の `topic_to_gossip_id`)、
   rendezvous ドメイン(`crates/core/src/rendezvous.rs`)、DM の HKDF/AAD ドメイン群、
   wire prefix 定数(`crates/core/src/wire.rs`)。
   固定: `crates/core/src/tests/derivation_golden.rs` / `wire_constants.rs`、
   `crates/docs-sync/src/tests/replicas.rs`、transport の `topic_to_gossip_id_matches_golden`。
3. **serde フィールド名・variant 名 = wire 形式**: `GossipHint`(externally-tagged、受信側は
   デコード失敗を黙殺)、docs エントリ値の `KukuriEnvelope` / `*DocV1` 群、envelope kind/tag 文字列。
   固定: `crates/core/src/tests/wire_snapshot.rs`。
4. **`PayloadRef` の PascalCase タグ**: 周囲の snake_case と不整合だが署名済み content と
   SQLite に永続済み。「統一リファクタ」は全既存データの破壊。固定: 同上 wire_snapshot.rs。
5. **docs エントリ key スキーム**(`objects/{id}/state` 等): app-api の `stable_key` 生成と
   cn-indexer の prefix 走査の暗黙契約。固定: cn-indexer の ingestion_contracts(常時実行)。
6. **永続スキーマと identity**: SQLite / Postgres の migration 済みスキーマ、identity ファイル群
   (keyring account は db パス依存 — パス変更 = 鍵ロスト)、endpoint secret(iroh::SecretKey の
   serde 表現に直結)。固定: migration テスト(拡充は後続 WP)。
7. **community-node HTTP 契約と manifest**: cn-user-api の endpoint 群と `CommunityNodeManifest`
   (server 完全版 ⇔ client slim 版)。変更は後方互換な optional フィールド追加のみ可。
   固定: `crates/cn-user-api/tests/`、round-trip は
   `crates/desktop-runtime/src/community_node/manifest_support.rs` +
   `crates/cn-operator/tests/manifest_golden.rs`(**この 2 ファイルを触る PR は
   `cargo xtask rust-test` の round-trip テストと `cn-test` の golden の両方を実行する**)。
8. **cn-operator の `Availability::Planned` 3 capability**: 昇格は ADR 0027 §2.9 の条件付き
   decision であり、リファクタで表明を変えない。
9. **Tauri IPC 契約**(`crates/app-api/src/views.rs` ⇔ `apps/desktop/src/lib/api/types.ts`):
   同一バイナリ内契約のため両側同時変更なら改名可能だが、片側変更は silent break。
   固定: 未(WP-S4 で contract テスト / codegen を導入予定)。

## 真実の置き場所ルール

- 現行ドキュメントの優先順位は `docs/README.md` に従う。
- ADR は承認済みの protocol / product 方針を定義する。
- runbook は実行手順と運用手順を定義する。
- progress document は現在の milestone 状態を説明する。
- テストと `harness/scenarios/` は実行可能な振る舞いを定義する。
- 文書同士が矛盾する場合は、古い文書または古くなった文書を更新する。更新しない場合は、完了報告で矛盾を明示する。

## path別検証マトリクス

| 変更path | 必須validation |
|---|---|
| `crates/core/**` | `cargo xtask rust-test` |
| `crates/store/**` | `cargo xtask rust-test` + 永続化の振る舞いが変わる場合は関連 scenario |
| `crates/transport/**` | `cargo xtask rust-test` + peer の振る舞いが変わる場合は関連 connectivity scenario |
| `crates/docs-sync/**` | `cargo xtask rust-test` + `cargo xtask e2e-smoke` |
| `crates/blob-service/**` | `cargo xtask rust-test` + media/blob に影響する場合は関連 scenario |
| `crates/app-api/**` | `cargo xtask rust-test` + payload 形状が変わる場合は frontend test |
| `crates/desktop-runtime/**` | `cargo xtask rust-test` + `cargo xtask e2e-smoke` |
| `crates/cn-*` | `cargo xtask cn-check` + `cargo xtask cn-test` |
| `harness/scenarios/**` | `cargo xtask scenario <changed-scenario>` |
| `apps/desktop/**` | `cargo xtask desktop-ui-check` |
| `apps/desktop/src-tauri/**` | `cargo xtask tauri-check` + `cargo xtask e2e-smoke` |
| `docs/adr/**` | 対応する tests / contracts / scenarios を確認または更新する |
| `docs/runbooks/**` | runbook 内の command と path を確認する |

必須validationの全実行がローカルで重すぎる場合は、最も狭い関連 command を実行し、実行しなかった内容と理由を明確に報告する。実行していない validation を passed と報告しない。

PR 作成前または `main` merge 前は、可能なら `cargo xtask check` + `cargo xtask test` を推奨する。

## 大型ファイルポリシー

`cargo xtask oversized-files` は大型(1000行以上)の手書きファイルを報告し、
`xtask/oversized-baseline.json` との比較で **ratchet 方式の CI ゲート**として動作する。

ゲートの判定:

- baseline に無い 1000 行以上の新規ファイル → **fail**。
- baseline 記載ファイルが記録行数を超えて成長 → **fail**。
- 行数不変・減少 → pass(減少時は baseline 更新推奨の note を表示)。
- baseline の更新は `cargo xtask oversized-files --update-baseline` で再生成し、
  正当化を PR 本文に書いた上でコミットする(レビューを通すことが ratchet の意図)。
- ファイルを分割して 1000 行未満にした場合も `--update-baseline` で baseline から除去する。

ルール:

- 明示的な正当化(= baseline 更新コミット)なしに、1000行以上の新規手書きファイルを追加しない。
- 既存の大型ファイルを編集する場合は、差分を最小に保つ(行数を増やす変更は baseline 更新が必要になる)。
- 大型ファイルの中で大きなロジック変更を行う場合は、先に分割計画を提案する。
- 1500行を超えるファイルで、かつ複数責務に触る場合は、後続の分割計画を作成する。
- formatting-only change と semantic change を混ぜない。
- generated file、lock file、icon は明示的に対象化されない限り、この方針の対象外とする。

## 必須AIワークフロー

リファクタリングタスクでは、AIエージェントは以下の順序に従う。

1. `AGENTS.md` とこの文書を読む。
2. リファクタリング種別を特定する。
3. 影響pathと必須validationを特定する。
4. 現在の tests / contracts / scenarios を確認する。
5. 振る舞いの仕様が不足している場合は、先に characterization test を追加する。
6. タスクを満たす最小の構造変更を行う。
7. 必須validationを実行する。実行しない場合は理由を報告する。
8. 完了報告を出す。

## レビューチェックリスト

リファクタリングPRは、以下の観点でレビューする。

- 本当に挙動維持になっているか。
- PR は 1意図に限定されているか。
- rename / move と logic change が分離されているか。
- public API、protocol、storage、docs/blobs、community-node contract が維持されているか。
- 新しい抽象化は、既存の重複または境界上の圧力によって正当化されているか。
- 結合を減らしているか。それとも単に code を移動しただけか。
- 触った振る舞いに対して tests / contracts / scenarios は十分か。
- 必須validationは実行され、報告されているか。
- 大型ファイルを慎重に扱っているか。
- diff 内の `#[serde` 属性・wire 文字列リテラル・canonical 実装の変更を目視したか(凍結境界の章を参照)。

## 完了報告形式

AIエージェントは、リファクタリング作業を以下の形式で終了する。

- 変更種別:
- 目的:
- 変更path:
- 挙動変更:
- Public API / protocol / storage の変更:
- 追加または更新した tests / contracts / scenarios:
- 実行したvalidation:
- 実行しなかったvalidation:
- リスク:
- 推奨後続対応:
