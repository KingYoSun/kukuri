# Issue #664 Community Node indexing 申請 UI review

- PR: Issue #664 の実装ブランチ `codex/issue-664-community-indexing-request`
- Preview: [プライベートチャネル申請ダイアログ](assets/2026-08-13-issue-664/community-indexing-request-private.png)
- Summary: 公開トピック管理とプライベートチャネル設定から、認証・同意済みで `community_index` を提供する Community Node を選び、索引登録を申請できるようにした。プライベートチャネルでは read capability を node に渡す意味を明示し、毎回のチェック確認を必須にした。
- Review result: 承認。Storybook の 1280×720 dark theme で、警告文の折り返し、チェックボックスの配置、送信前 disabled 状態、主従ボタン、スクロール不要の収まりを確認した。初回確認で native checkbox が汎用 `.field input` の全幅指定を継承していたため、専用の native label レイアウトへ修正した。
- Exceptions: なし。
- Validation: Storybook `Core/CommunityIndexingRequestDialog`、Vitest の公開／プライベート申請、Playwright の topic 管理→申請と channel 作成→設定→確認→申請、`pnpm typecheck`、`pnpm lint`。

## User flow

1. 公開トピック行またはプライベートチャネル設定から「索引登録を申請」を開く。
2. configured・authenticated・consented・`community_index` available の node だけから送信先を選ぶ。
3. プライベートチャネルでは、node が内容を復号・索引可能になる警告を読み、read capability の開示へ明示同意する。
4. 申請後に `pending`、`approved`、`rejected` の現在状態を確認する。状態を再確認する場合も同じ POST を明示操作で行い、private は再度確認する。
5. 404 設定不足、409 capability conflict、認証・同意不足、一般通信失敗は別メッセージで回復方法を判断できる。

## Shneiderman checklist

- Consistency: 公開／プライベートで同じ node eligibility、node selector、結果表示を利用する。
- Shortcuts: トピック行とチャネル設定から対象を保持したまま申請画面へ進める。
- Informative feedback: 送信中 disabled、現在 status、既知の安定エラーを明示する。
- Dialog closure: Close と申請結果を同一ダイアログ内に置き、完了点を明確にする。
- Error prevention: private capability はチェック確認前に IPC command を呼ばず、Rust 側でも確認フラグを再検証する。
- Easy reversal: 申請は indexing を保証せず operator 審査対象であることを status で示す。client に永続状態を持たない。
- Internal locus of control: 自動申請・fan-out・polling を行わず、node 選択と各送信をユーザー操作に限定する。
- Reduce short-term memory load: 対象、送信先、開示影響、結果を一つのダイアログ内に表示する。
