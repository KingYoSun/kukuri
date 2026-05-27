# kukuri AIリファクタリング環境改善 計画書

## 目的

kukuri を「AIが安全にコードをきれいにできる環境」として強化する。

現在の kukuri は、`AGENTS.md`、`xtask`、ADR / runbook / progress / harness / CI によって、AIが機能追加・バグ修正を行う土台はかなり整っている。一方で、大規模リファクタリングや構造整理をAIに任せるには、以下が不足している。

- リファクタリング専用ルール
- PR分割の明文化
- path別 validation matrix
- source-of-truth 優先順位の統一
- PR taxonomy
- oversized file の運用ルール

本計画では、これらを小さなドキュメント・運用改善として追加し、AIエージェントがより安全にリファクタリングできる状態を作る。

---

## 実施方針

### 基本方針

- 実装コードの大規模変更は行わない。
- まずは AI 作業環境・ドキュメント・運用ルールを整備する。
- `REFACTORING.md` を新設し、リファクタリング専用ルールを分離する。
- `AGENTS.md` は短い入口として維持し、詳細は `REFACTORING.md` へ誘導する。
- `docs/README.md` と `AGENTS.md` の source-of-truth 優先順位を一致させる。
- validation matrix は、AIが「どこを触ったら何を実行すべきか」を判断できる粒度にする。

### 非目的

- この計画では実装リファクタリング自体は行わない。
- CI の必須/任意 job 設定変更は、必要な場合のみ後続タスクとする。
- crate 再分割やファイル分割などの構造変更は、この計画の対象外とする。
- `legacy/` の移植や削除は行わない。

---

## 作業全体像

```text
Phase 1: ドキュメント整合
  - AGENTS.md の source-of-truth 優先順位を docs/README.md に合わせる
  - AGENTS.md に REFACTORING.md へのポインタを追加する

Phase 2: REFACTORING.md 新設
  - Refactoring Mode を定義する
  - PR types / 禁止事項 / 作業手順 / review checklist を定義する

Phase 3: validation matrix 追加
  - path別に必要な validation を明文化する
  - AIが完了報告で validation 結果を出すルールを追加する

Phase 4: PR taxonomy 追加
  - PR title prefix と PR本文テンプレート方針を定義する
  - refactor / contract / scenario / fix / deps を分離する

Phase 5: oversized file 運用ルール追加
  - oversized report の扱いを明文化する
  - 既存大型ファイルを触る場合のルールを追加する

Phase 6: 検証
  - Markdown内容の整合確認
  - `cargo xtask doctor`
  - 必要に応じて `cargo xtask check`
```

---

## Phase 1: `AGENTS.md` の更新

### 目的

`AGENTS.md` をAIエージェントの短い入口として維持しつつ、詳細なリファクタリングルールは `REFACTORING.md` に委譲する。

### 変更内容

#### 1. `まず読む` に `REFACTORING.md` を追加

`AGENTS.md` の `まず読む` に以下を追加する。

```md
- `REFACTORING.md`（リファクタリング・構造整理・大きめの移動/抽出を行う場合）
```

#### 2. source-of-truth 優先順位のズレを修正

現在の `AGENTS.md` は `docs/progress/2026-03-10-foundation.md` を現行スコープの優先情報としている。一方、`docs/README.md` では `docs/progress/2026-04-16-mvp-builder-preview-plan.md` が優先参照順の先頭にある。

これを以下のように修正する。

```md
- 現行スコープの参照優先順位は `docs/README.md` に従う。
- foundation baseline は `docs/progress/2026-03-10-foundation.md`。
- builder preview / 配布 / 初回体験は `docs/progress/2026-04-16-mvp-builder-preview-plan.md`。
```

#### 3. `ガードレール` にリファクタリング時の参照ルールを追加

```md
- リファクタリング、ファイル分割、責務境界変更、dead code 削除を行う場合は、先に `REFACTORING.md` を読む。
- リファクタリングPRでは、機能追加・仕様変更・依存更新を混ぜない。
```

### 期待結果

- AIが `AGENTS.md` から迷わず `REFACTORING.md` に移動できる。
- source-of-truth の優先順位が `docs/README.md` と矛盾しない。
- リファクタリング作業だけ詳細文書を参照する構造になる。

---

## Phase 2: `REFACTORING.md` 新設

### 目的

AIがリファクタリング作業を安全に進めるための専用ルールを定義する。

### 配置

```text
REFACTORING.md
```

root 直下に置く。理由は、AIエージェントが作業開始時に最も見つけやすく、`AGENTS.md` から直接参照しやすいため。

### 推奨構成

```md
# REFACTORING.md

## Purpose
## Refactoring Mode
## Non-Goals
## Core Principles
## PR Types
## Forbidden Mixed Changes
## Source-of-Truth Rules
## Path-Based Validation Matrix
## Oversized File Policy
## Legacy Migration Rules
## Required AI Workflow
## Review Checklist
## Completion Report Format
```

---

## `REFACTORING.md` に入れる主要内容

### 1. Purpose

```md
# REFACTORING.md

This document defines how AI agents and humans should perform refactoring work in kukuri.

Refactoring in this repository means improving internal structure while preserving external behavior, protocol contracts, storage semantics, and user-visible behavior unless a task explicitly says otherwise.
```

日本語でもよいが、Codexなど英語圏モデルに読ませるなら英語主体でもよい。既存 `AGENTS.md` は日本語なので、日本語主体 + 重要語だけ英語でも問題ない。

### 2. Refactoring Mode

```md
## Refactoring Mode

リファクタリング作業では、機能追加・仕様変更・依存更新を混ぜない。

リファクタリングとは、外部挙動を維持したまま、内部構造・命名・責務境界・重複・ファイル構成を改善する作業である。

外部挙動を変更する場合は、それは refactor ではなく feature / fix / migration として扱う。
```

### 3. Core Principles

```md
## Core Principles

- 1 PR = 1 intent。
- rename / move / extraction / behavior change を同じ PR に混ぜない。
- public API、protocol object、storage schema、docs/blobs canonical source、community-node endpoint contract は、明示指示なしに変更しない。
- 既存テストを削除しない。削除が必要な場合は、削除理由と代替テストを示す。
- 振る舞いが変わる可能性がある場合は、先に characterization test / contract / scenario を追加する。
- `legacy/` からの移植は、必要最小限の概念移植に限定し、ファイル単位コピーは禁止する。
- 大規模な抽象化を導入する前に、既存の責務境界と呼び出し方向を調査する。
- 便利そうな共通化より、境界をまたぐ結合を増やさないことを優先する。
```

### 4. PR Types

```md
## PR Types

Use one of these labels/prefixes in the PR title or task summary.

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
```

### 5. Forbidden Mixed Changes

```md
## Forbidden Mixed Changes

以下を同じPRに混ぜない。

- rename + logic change
- file move + behavior change
- dependency update + refactor
- storage migration + UI refactor
- protocol shape change + internal cleanup
- test deletion + implementation change without replacement
- `legacy/` copy + active implementation rewrite
- formatting-only change + semantic change
```

### 6. Source-of-Truth Rules

```md
## Source-of-Truth Rules

- Current documentation priority follows `docs/README.md`.
- ADRs define accepted protocol/product decisions.
- Runbooks define execution and operational procedures.
- Progress documents describe current milestone state.
- Tests and `harness/scenarios/` define executable behavior.
- If documents conflict, update the older/stale document or explicitly mention the conflict in the completion report.
```

### 7. Path-Based Validation Matrix

```md
## Path-Based Validation Matrix

| Changed path | Required validation |
|---|---|
| `crates/core/**` | `cargo xtask rust-test` |
| `crates/store/**` | `cargo xtask rust-test` + relevant scenario if persistence behavior changes |
| `crates/transport/**` | `cargo xtask rust-test` + relevant connectivity scenario if peer behavior changes |
| `crates/docs-sync/**` | `cargo xtask rust-test` + `cargo xtask e2e-smoke` |
| `crates/blob-service/**` | `cargo xtask rust-test` + media/blob scenario if affected |
| `crates/app-api/**` | `cargo xtask rust-test` + frontend tests if payload shape changes |
| `crates/desktop-runtime/**` | `cargo xtask rust-test` + `cargo xtask e2e-smoke` |
| `crates/cn-*` | `cargo xtask cn-check` + `cargo xtask cn-test` |
| `harness/scenarios/**` | `cargo xtask scenario <changed-scenario>` |
| `apps/desktop/**` | `cargo xtask desktop-ui-check` |
| `apps/desktop/src-tauri/**` | `cargo xtask tauri-check` + `cargo xtask e2e-smoke` |
| `docs/adr/**` | corresponding tests/contracts/scenarios must be checked or updated |
| `docs/runbooks/**` | verify commands and paths mentioned in the runbook |
| `legacy/**` | normally no edits; explicit migration task required |
```

補足として以下も追加する。

```md
If a full required validation is too expensive locally, run the narrowest relevant command and clearly report what was not run and why. Do not claim validation passed if it was not executed.
```

### 8. Oversized File Policy

```md
## Oversized File Policy

`cargo xtask oversized-files` reports large hand-written files.

Rules:

- Do not add new hand-written files over 1000 lines unless explicitly justified.
- When editing an existing oversized file, keep the diff minimal.
- Do not perform large logic changes inside an oversized file without first proposing a split plan.
- If a file exceeds 1500 lines and the task touches multiple responsibilities, create a follow-up split plan.
- Generated files, lock files, icons, and `legacy/` are excluded from this policy unless explicitly targeted.
```

### 9. Legacy Migration Rules

```md
## Legacy Migration Rules

- `legacy/` is reference-only.
- Do not copy entire files from `legacy/` into the active workspace.
- Migrate concepts, contracts, or small implementation pieces only when explicitly requested.
- Before migrating from `legacy/`, identify the active current equivalent and the accepted ADR/progress document.
- Add or update tests/contracts/scenarios before moving behavior into active code.
```

### 10. Required AI Workflow

```md
## Required AI Workflow

For refactoring tasks, AI agents must follow this order:

1. Read `AGENTS.md` and this document.
2. Identify the refactoring type.
3. Identify affected paths and required validation.
4. Inspect current tests/contracts/scenarios.
5. If behavior is under-specified, add characterization tests first.
6. Make the smallest structural change that satisfies the task.
7. Run required validation or report why it was not run.
8. Provide a completion report.
```

### 11. Review Checklist

```md
## Review Checklist

Review refactoring PRs with these questions:

- Is this truly behavior-preserving?
- Is the PR limited to one intent?
- Are rename/move changes separated from logic changes?
- Are public API, protocol, storage, docs/blobs, and community-node contracts preserved?
- Are new abstractions justified by existing duplication or boundary pressure?
- Did the change reduce coupling, or merely move code around?
- Are tests/contracts/scenarios sufficient for the touched behavior?
- Were required validations run and reported?
- Did the PR avoid copying from `legacy/`?
- Are oversized files handled carefully?
```

### 12. Completion Report Format

```md
## Completion Report Format

AI agents must end refactoring work with:

- Change type:
- Goal:
- Changed paths:
- Behavior changes:
- Public API / protocol / storage changes:
- Tests/contracts/scenarios added or updated:
- Validation run:
- Validation not run:
- Risks:
- Suggested follow-ups:
```

---

## Phase 3: validation matrix の導入

### 目的

AIが path に応じて validation を自律的に選べるようにする。

### 実施内容

- `REFACTORING.md` に path-based validation matrix を追加する。
- `AGENTS.md` には詳細を重複させず、以下の短いポインタだけを置く。

```md
- 変更pathごとの必須validationは `REFACTORING.md` の Path-Based Validation Matrix に従う。
```

### 注意点

`cargo xtask check` と `cargo xtask test` を常に必須にするとAI作業が重くなりすぎる可能性があるため、matrix では targeted validation を中心にする。

ただし、PR作成前や main merge 前は `cargo xtask check` + `cargo xtask test` を推奨する。

---

## Phase 4: PR taxonomy の導入

### 目的

AI生成PRの性質をタイトル・本文から即座に判断できるようにする。

### 推奨PR title prefix

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

### `REFACTORING.md` に書くルール

```md
When creating or describing PRs, use a clear task type. A refactoring PR must not hide feature work or behavior changes.
```

### PR本文に含める項目

```md
## Summary

## Type

## Behavior Change

## Validation

## Risk

## Follow-up
```

---

## Phase 5: oversized file 運用ルール追加

### 目的

AIが巨大ファイルに大きな差分を作るリスクを下げる。

### 現状

`xtask` に `oversized-files` があり、CIでも oversized file report が実行されている。

### 追加する運用

`REFACTORING.md` に以下を明記する。

- 新規1000行超え手書きファイルを原則禁止
- 既存1000行超えファイルを触る場合は最小diff
- 1500行超えかつ複数責務に触る場合は先に分割計画
- formatting-only と semantic change を混ぜない

### 将来の後続タスク

必要であれば、後続で `cargo xtask oversized-files --deny-new` のような CI fail モードを追加する。ただし、この計画ではドキュメント運用に留める。

---

## Phase 6: 検証

### 実行する検証

ドキュメント変更中心なので、最低限以下を実行する。

```bash
cargo xtask doctor
```

余裕があれば以下も実行する。

```bash
cargo xtask check
```

Markdown専用lintが存在しない場合、目視で以下を確認する。

- `AGENTS.md` から `REFACTORING.md` へリンクできる
- `docs/README.md` の優先参照順と矛盾しない
- path-based validation matrix のコマンドが既存 `xtask` と一致する
- `legacy/` の扱いが既存方針と矛盾しない
- PR taxonomy が既存 `[codex]` 運用と衝突しない

---

## 実施時の制約

- プロダクトコードは変更しない。
- この計画では CI の挙動を変更しない。
- `legacy/` は編集しない。
- `docs/` 配下に長い新規ドキュメントを作らない。`REFACTORING.md` は必ず root 直下に作成する。
- `AGENTS.md` は簡潔に保つ。
- 既存の `AGENTS.md`、`docs/README.md`、`docs/runbooks/dev.md` の方針と矛盾する内容を追加しない。
- validation command を実行しなかった場合は、実行していないことと理由を正直に報告する。

---

## 実施後の報告形式

作業完了時は、以下の形式で報告する。

```md
- Changed files:
- Summary of changes:
- Validation run:
- Validation not run:
- Risks:
- Suggested follow-ups:
```

---

## 完了条件

この計画の完了条件は以下。

- `REFACTORING.md` が root に追加されている
- `AGENTS.md` に `REFACTORING.md` へのポインタがある
- `AGENTS.md` の source-of-truth 優先順位が `docs/README.md` と矛盾しない
- `REFACTORING.md` に Refactoring Mode が定義されている
- `REFACTORING.md` に PR Types が定義されている
- `REFACTORING.md` に Forbidden Mixed Changes が定義されている
- `REFACTORING.md` に Path-Based Validation Matrix がある
- `REFACTORING.md` に Oversized File Policy がある
- `REFACTORING.md` に Legacy Migration Rules がある
- `REFACTORING.md` に Completion Report Format がある
- `cargo xtask doctor` の結果が報告されている

---

## 後続タスク候補

今回の計画後に追加で検討するとよいもの。

### 1. PR template 追加

`.github/pull_request_template.md` を追加し、PR type / behavior change / validation / risk を必須化する。

### 2. oversized file deny mode

`cargo xtask oversized-files` に fail mode を追加する。

例:

```bash
cargo xtask oversized-files --deny-new
```

### 3. architecture boundary document

crate間の責務・依存方向を明文化する `docs/architecture/boundaries.md` を追加する。

ただし、これは `docs/` の長文ドキュメント追加になるため、`AGENTS.md` の「rootに長文ドキュメントを増やさない」方針との整合を取りながら行う。

### 4. scenario catalog

`harness/scenarios/` の各 scenario が何を保護しているかを一覧化する。

AIが変更影響に応じて scenario を選びやすくなる。

### 5. refactor backlog

巨大ファイル、責務が曖昧なcrate、重複実装などを `docs/progress/` または issue に整理し、AIに小分けで処理させる。

