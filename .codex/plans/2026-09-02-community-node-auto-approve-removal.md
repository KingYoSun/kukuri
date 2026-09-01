# Implementation Plan

## Goal
実際の接続制御に使われていない `auto_approve` フラグを Community Node の設定、実行時状態、IPC、画面、配布設定、文書から削除し、明示同意済みの設定 Node は自動的にセッション維持するという単一契約へ整理する。

## Non-goals
- Issue #857 が定める Node 別・文書別の明示同意、同意前通信禁止、重要変更時の再同意は変更しない。
- default Community Node の候補 URL、初回だけ配布設定を保存する挙動、削除・置換後に復活しない挙動は変更しない。
- Community Node の認証、登録、接続維持、参加審査、HTTP endpoint の契約は変更しない。
- 手動接続モードや代替フラグは追加しない。

## Assumptions
- 現行 runtime は `auto_approve` を接続可否の分岐に使っておらず、設定済み Node はローカル同意成立後に同じセッション維持経路へ入る。
- `CommunityNodeConfig` は未知の JSON フィールドを拒否していないため、旧 `community-node.json` の `auto_approve` は専用 migration を追加せず読み捨てられる。
- Community Node の Rust 型と TypeScript 型は同一 desktop binary 内の IPC 契約であり、両側を同一変更で更新できる。
- 配布設定からフラグを除いても default Node の初回 preload 判定には影響しない。

## Definition of Done
- product source、配布設定、生成 IPC 型、画面文言、現行文書から `auto_approve` / `autoApprove` と「自動承認」の概念がなくなる。旧設定互換テストの入力 fixture に限り旧キーを残してよい。
- 旧 `auto_approve` を含む保存済み設定を読み込め、再保存した JSON から旧フィールドが除去される。
- Community Node 設定画面から自動承認の入力・診断表示がなくなり、Node の保存 request は base URL だけを送る。
- 未同意 Node への通信が発生せず、明示同意後だけセッションが確立し、policy 更新時には黙って再同意しない既存契約が成功する。
- 配布 Node の初回 preload、利用者による空設定・置換設定の優先、Community Node 関連の desktop/runtime 回帰が成功する。

## Plan
| ID | Task | Outcome | Files / Areas | Acceptance Criteria | Validation | Depends On |
|---|---|---|---|---|---|---|
| T1 | Rust の設定・状態・入力型から不要フラグを削除し、旧設定の読み捨て互換を固定する | runtime とローカル設定の Node モデルが `base_url` と接続解決状態だけを持ち、重複正規化から不要な真偽値統合が消える | `crates/desktop-runtime/src/community_node/{mod.rs,config_support.rs,session_runtime_support.rs}`、`crates/desktop-runtime/src/runtime/community_node_api.rs`、`crates/desktop-runtime/src/tests/**`、`crates/harness/src/scenarios/**` | `CommunityNodeNodeConfig`、`CommunityNodeNodeStatus`、設定入力からフラグが消える。旧キー入り JSON は同じ Node URL と解決済み接続情報を保持して読み込め、保存後は旧キーを出力しない。未同意、同意済み、policy 更新の各セッション契約は従来どおり成立する | 旧設定の load/save 互換テスト、対象 community-node runtime tests、`cargo xtask ipc-types`、`cargo xtask rust-test` | None |
| T2 | 配布設定と Tauri / frontend の設定契約・画面からフラグを削除する | 利用者が意味のない自動承認設定を操作・診断できなくなり、default Node は URL だけで preload される | `apps/desktop/src-tauri/distribution/community-nodes.json`、`apps/desktop/src-tauri/src/state.rs`、`apps/desktop/src/lib/api/{types.ts,types.generated.ts}`、`apps/desktop/src/shell/**`、`apps/desktop/src/components/settings/**`、`apps/desktop/src/mocks/**`、関連 story/test/i18n | 配布 JSON と設定 request に旧フィールドがない。設定画面・診断・story・mock に自動承認の表示や入力がない。fresh install、空設定、置換設定の優先契約が維持され、Rust 生成型と TypeScript 利用側が一致する | `cargo xtask ipc-types`、Tauri 配布設定 test、関連 Vitest、`cargo xtask tauri-check`、`cargo xtask desktop-ui-check` | T1 |
| T3 | 現行文書とテスト名称を明示同意後の自動接続契約へ合わせる | `auto_approve` が法的同意を自動化するように読める記述がなくなり、Issue #857 との責任境界が一貫する | `docs/progress/2026-04-16-mvp-builder-preview-plan.md`、`docs/adr/0024-community-node-admission-data-classification.md`、`docs/runbooks/mvp-troubleshooting.md`、関連コードコメント・テスト名 | 文書は「配布候補」「利用者の明示同意」「同意後の自動セッション確立」を区別する。認証前の公開 policy 照合、サーバ同意記録への同期、重要変更時の再同意を自動的な法的同意と表現しない | `rg` による旧用語残存確認、文書内の command/path 確認、更新したテストの実行 | T1、T2 |
| T4 | path 別の必須検証と全体回帰を完走する | フラグ削除が desktop 起動、設定永続化、IPC、画面、Community Node セッションを壊していないことを判定できる | workspace 全体 | `auto_approve` / `autoApprove` は旧設定互換 fixture 以外に残らず、生成差分が確定し、必須検証がすべて成功する。意図した画面差分で視覚回帰が失敗した場合は Linux/Chromium の正規手順で baseline を更新する | `cargo xtask check`、`cargo xtask test`、`cargo xtask rust-test`、`cargo xtask tauri-check`、`cargo xtask desktop-ui-check`、`cargo xtask e2e-smoke` | T1、T2、T3 |

## Decision Needed / Blockers
None

## Out of Scope
- Community Node 同意モーダルのデザイン変更。
- app-level 利用規約・プライバシーポリシー同意の変更。
- Community Node server 側の consent データや API の移行。

## Single Next Action
PR を作成し、CI 成功後にマージする。

## Progress
- 2026-09-02: T1-T3 を実装。Rust / IPC / frontend / 配布設定 / 現行文書からフラグを削除し、旧設定の読み捨て・再保存互換テストを追加。
- 2026-09-02: `cargo xtask check`、`cargo xtask test`、`cargo xtask rust-test`、`cargo xtask tauri-check`、`cargo xtask desktop-ui-check`、`cargo xtask e2e-smoke` が成功。
