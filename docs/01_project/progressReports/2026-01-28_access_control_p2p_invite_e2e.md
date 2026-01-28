# Access Control P2P-only invite/join/key.envelope E2E
日付: 2026年01月28日

## 概要
- `access_control_issue_invite`/`access_control_request_join` を使った P2P-only の invite共有→join.request→key.envelope→暗号化投稿を統合テストで追加した。

## 対応内容
- `kukuri-tauri/src-tauri/tests/integration/access_control_p2p_invite.rs` を追加し、AccessControlService の inviter/requester を組み合わせてイベント往復を検証。
- TestKeyManager/TestGroupKeyStore/TestGossipService を用いて join.request と key.envelope の手動伝搬を再現。
- PostService で invite scope の暗号化投稿を生成し、`EncryptedPostPayload` 解析で scope/epoch を確認。
- `kukuri-tauri/src-tauri/src/lib.rs` の `test_support` に `presentation::dto` を re-export し、テストから DTO を利用可能にした。

## 検証
- `./scripts/test-docker.ps1 rust`
- `gh act --workflows .github/workflows/test.yml --job format-check`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`

## 補足
- `gh act` 実行時に git clone の `some refs were not updated`、pnpm `approve-builds` notice、React `act(...)` 警告、`useRouter` が RouterProvider 外の警告を確認（既知）。
