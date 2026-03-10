# Community Node 通報 UI/E2E
日付: 2026年02月03日

## 概要
- PostCard に通報 UI（理由選択 + submit）を追加し、`community_node_submit_report` を通す E2E を整備。

## 対応内容
- PostCard に通報メニューと理由選択ダイアログを追加し、Community Node へ通報を送信。
- report API 型を追加し、`submitReport` を型安全化。
- E2E に `community-node.report.spec.ts` を追加し、seed 投稿で通報送信を確認。
- PostCard の unit test に通報フロー検証を追加。

## 検証
- `./scripts/test-docker.ps1 ts`（PASS。React act/useRouter 警告あり）
- `./scripts/test-docker.ps1 e2e-community-node`（PASS。Spec Files: 15 passed, 15 total。ログ: `tmp/logs/community-node-e2e/20260203-032326.log`）
- `gh act --workflows .github/workflows/test.yml --job format-check`（PASS。git clone の non-terminating warning と pnpm の ignored build scripts 警告あり）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（PASS。React act/useRouter 警告・`ReactDOMTestUtils.act` 非推奨警告、`ENABLE_P2P_INTEGRATION` 未設定による skip と performance tests ignored あり）
