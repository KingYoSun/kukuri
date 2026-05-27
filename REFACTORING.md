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
- 明示依頼がない限り、`legacy/` の移植や削除を行わない。

## 基本原則

- 1 PR = 1意図。
- rename / move / extraction / behavior change を同じ PR に混ぜない。
- 明示指示なしに public API、protocol object、storage schema、docs/blobs canonical source、community-node endpoint contract を変更しない。
- 既存テストを削除しない。削除が必要な場合は、削除理由と代替カバレッジを示す。
- 振る舞いが変わる可能性がある場合は、先に characterization test / contract / scenario を追加する。
- `legacy/` からの移植は、必要最小限の概念または実装片に限定する。ファイル全体をコピーしない。
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
- `legacy/` copy + active implementation rewrite
- formatting-only change + semantic change

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
| `legacy/**` | 原則編集しない。明示的な migration タスクが必要 |

必須validationの全実行がローカルで重すぎる場合は、最も狭い関連 command を実行し、実行しなかった内容と理由を明確に報告する。実行していない validation を passed と報告しない。

PR 作成前または `main` merge 前は、可能なら `cargo xtask check` + `cargo xtask test` を推奨する。

## 大型ファイルポリシー

`cargo xtask oversized-files` は大型の手書きファイルを報告する。

ルール:

- 明示的な正当化なしに、1000行を超える新規手書きファイルを追加しない。
- 既存の大型ファイルを編集する場合は、差分を最小に保つ。
- 大型ファイルの中で大きなロジック変更を行う場合は、先に分割計画を提案する。
- 1500行を超えるファイルで、かつ複数責務に触る場合は、後続の分割計画を作成する。
- formatting-only change と semantic change を混ぜない。
- generated file、lock file、icon、`legacy/` は明示的に対象化されない限り、この方針の対象外とする。

## legacy 移植ルール

- `legacy/` は参照専用である。
- `legacy/` から現行 workspace へファイル全体をコピーしない。
- 明示依頼がある場合に限り、概念、contract、小さな実装片を移植する。
- `legacy/` から移植する前に、現行実装側の対応箇所と承認済み ADR / progress document を特定する。
- 現行コードへ振る舞いを移す前に、tests / contracts / scenarios を追加または更新する。

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
- `legacy/` からのコピーを避けているか。
- 大型ファイルを慎重に扱っているか。

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
