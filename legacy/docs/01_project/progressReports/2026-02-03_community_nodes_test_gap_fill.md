# Community Nodes テスト不足の補完
日付: 2026年02月03日

## 概要
- Roadmap の「テスト不足の補完」に対応し、cn-kip-types/cn-user-api/cn-admin-api/kukuri-tauri の不足テストを追加。

## 対応内容
- cn-kip-types: 39001/39005/39010/39011/39020 の検証テストを追加。
- cn-user-api: `/v1/bootstrap/*` `/v1/reports` `/v1/search` の契約テストを追加。
- cn-admin-api: login/logout の契約テストを追加。
- kukuri-tauri: CommunityNodeHandler の単体テスト（設定正規化/信頼アンカー）を追加。

## 検証
- `./scripts/test-docker.ps1 rust`（PASS。ENABLE_P2P_INTEGRATION 未設定による skip あり）
- `gh act --workflows .github/workflows/test.yml --job format-check`（PASS。git clone の non-terminating warning と pnpm ignored build scripts 警告あり）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`（PASS。React act/useRouter 警告、ENABLE_P2P_INTEGRATION 未設定による skip、performance tests ignored あり）
