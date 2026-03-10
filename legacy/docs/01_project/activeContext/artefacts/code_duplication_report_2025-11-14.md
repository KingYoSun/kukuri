# コード重複率調査（2025年11月14日）

## 背景
- `docs/01_project/deprecated/refactoring_plan_2025-08-08_v3.md`（アーカイブ済み）に定義された「コード重複率30%削減」指標の現状を把握し、Phase 4 DRY 施策の残課題を絞り込むため、最新の重複率を計測した。

## 測定条件
- ツール: `jscpd 4.0.5`
- 最低検出サイズ: 5行（既定値）、50トークン
- 対象: フロントエンド `kukuri-tauri/src`、Rust バックエンド `kukuri-tauri/src-tauri/src`、CLI `kukuri-community-node/crates/cn-cli/src`
- レポート: `tmp/jscpd/{frontend,rust}/jscpd-report.json`（Git管理外。再計測時は以下コマンドを実行）

```bash
# フロントエンド
npx jscpd --format typescript,tsx,javascript --min-lines 5 \
  --reporters json --gitignore --absolute --silent \
  --output tmp/jscpd/frontend kukuri-tauri/src

# Rust（kukuri-tauri + cn-cli）
npx jscpd --format rust --min-lines 5 \
  --reporters json --gitignore --absolute --silent \
  --output tmp/jscpd/rust kukuri-tauri/src-tauri/src kukuri-community-node/crates/cn-cli/src
```

## 計測結果サマリー

### TypeScript（kukuri-tauri/src）
- 解析対象 362ファイル / 53,610行
- 重複行数 2,231行（4.16%）、クローン 161件
- 完全にテスト同士で発生している重複が 1,893行（全体の約85%）
- 本番コードに現れている重複は 338行（約15%）で、Zustandストア・フォーム系UI・Tauri API ラッパーに集中

| # | ファイル | 重複行数 | メモ |
| - | - | - | - |
| 1 | `src/tests/unit/components/posts/DraftManager.test.tsx` | 418 | 投稿フォームのセットアップ/検証ロジックがシナリオごとに複製 |
| 2 | `src/tests/unit/components/SyncStatusIndicator.test.tsx` | 234 | Zustand モックと状態遷移の繰り返し |
| 3 | `src/tests/unit/components/posts/QuoteForm.test.tsx` | 218 | `ReplyForm` テストとほぼ同一ケースが並存 |
| 4 | `src/tests/unit/components/posts/ReplyForm.test.tsx` | 216 | 入力バリデーション/送信後検証の重複 |
| 5 | `src/tests/unit/stores/authStore.test.ts` | 211 | ストア初期化ヘルパーが複数 describe でコピー |
| 6 | `src/tests/unit/hooks/useUserSearchQuery.test.tsx` | 176 | TanStack Query モック生成が重複 |

| # | 本番コード（抜粋） | 重複行数 | メモ |
| - | - | - | - |
| 1 | `src/stores/postStore.ts` | 130 | フィード/投稿詳細の state 初期化＋更新ロジックが自己複製 |
| 2 | `src/stores/topicStore.ts` | 102 | Topic キャッシュ管理で postStore と類似パターン |
| 3 | `src/routes/profile.$userId.tsx` | 80 | プロファイル読み込みと DM/Follow 反映処理が他画面と重複 |
| 4 | `src/lib/api/tauri.ts` | 70 | Tauri Invoke ラッパーで同種のエラーハンドリングを都度記述 |
| 5 | `src/components/posts/ReplyForm.tsx` / `QuoteForm.tsx` | 70 / 57 | 各フォームで入力部品とsubmitロジックが二重化 |

### Rust（kukuri-tauri/src-tauri + kukuri-community-node/crates/cn-cli/src）
- 解析対象 251ファイル / 34,447行
- 重複行数 1,555行（4.51%）、クローン 135件
- `kukuri-tauri/src-tauri` が 1,533行（99%）を占め、`cn-cli` 側の重複は 22行のみ
- アプリケーションサービスとプレゼンテーション層の Tauri コマンドに同型の処理が散在

| # | ファイル | 重複行数 | メモ |
| - | - | - | - |
| 1 | `infrastructure/database/sqlite_repository/users.rs` | 321 | DTO ↔ Domain 変換とフィルタ条件が自ファイル内で多重化 |
| 2 | `application/services/user_service.rs` | 307 | 上記 Repository と同等の入力検証/レスポンス組み立てを再実装 |
| 3 | `application/services/direct_message_service/tests.rs` | 254 | Kind4 フローのセットアップ/アサーションがシナリオごとに複写 |
| 4 | `presentation/commands/user_commands.rs` | 122 | Tauri Command 側で service 呼び出し前後のバリデーションが重複 |
| 5 | `presentation/commands/post_commands.rs` | 119 | 投稿関連コマンドで似たエラー処理とレスポンス整形 |
| 6 | `application/services/topic_service.rs` | 118 | Topic/Feed 切替時のクエリ条件を複数関数で重複実装 |

## 主要な重複パターン
- **テストヘルパー不足（TS）**: コンポーネント/ストアテストがそれぞれローカルモックを定義しており、`__utils__` 以外でも列挙的に複写されている。
- **Zustandストアの state/mutation パターン**: `postStore`・`topicStore`・`authStore` で `setState` ブロックや optimistic update が同じ構造のまま分散。
- **フォーム UI のロジック二重化**: `ReplyForm`・`QuoteForm` など shadcn/ui ベースのフォームがバリデーションと送信処理をそれぞれ保持。
- **Rustサービスとコマンド層の責務境界が曖昧**: Repository で既に行う検証/整形が Service や Command で再度実装され、Nostr Event 変換が重複。
- **Tauriコマンドの定型エラーハンドリング**: `*_commands.rs` で `match` の分岐や `map_err` がコマンドごとにコピーされている。

## 推奨アクション
1. **共通テストファクトリの整備**  
   - `src/tests/unit/components/posts` / `hooks` 配下に shared factory を作成し、フォーム入力・Zustand モック生成を再利用できるよう切り出す。
2. **Zustand ストアのジェネリック化**  
   - `postStore`・`topicStore` のフェッチ/キャッシュ/optimistic update を `createEntityStore`（仮）に統合し、state 形状と reducer のみ個別定義にする。
3. **フォームコンポーネントの抽象化**  
   - `ReplyForm` / `QuoteForm` で共通する `usePostComposer` 相当の hooks + `FormFooter` パーツを導入してロジックを一本化。
4. **Rustサービス層の Mapper/Validator 共通化**  
   - `users.rs` の Row→Domain 変換や入力検証を `application/shared/mappers.rs`（既存 plan 4.1）に移し、Service/Command 側から呼び出す。
5. **Tauri コマンドのラッパユーティリティ**  
   - `presentation/commands/*` で共通の `invoke_command(async move { ... })` ラッパを用意し、エラーハンドリング/ログ出力を一箇所に集約。
