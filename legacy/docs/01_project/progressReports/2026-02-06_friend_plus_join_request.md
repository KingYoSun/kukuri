# friend_plus join.request（FoF）実装
日付: 2026年02月06日

## 概要
- friend_plus join.request を FoF(2-hop) 判定で検証できるように AccessControlService を拡張。
- friend_plus の join.request 受信→承認→key.envelope 配布→暗号投稿復号までのフローをテストで確認。

## 対応内容
- AccessControlService に UserRepository を注入し、相互フォロー(2-hop) 判定を追加。
- friend_plus join.request の検証/承認処理を更新し、key.envelope 配布まで通す。
- friend_plus の unit テスト（FoF/非該当/直接相互フォロー）と統合テストを追加。

## 検証
- `./scripts/test-docker.ps1 rust`
- `./scripts/test-docker.ps1 coverage`（coverage 40.94%、artefact: `docs/01_project/activeContext/artefacts/metrics/2026-02-06-024653-tarpaulin.*`）
- `gh act --workflows .github/workflows/test.yml --job format-check`
  - `tmp/logs/gh-act-format-check-20260206-024756.log`: rustfmt 差分で失敗
  - `tmp/logs/gh-act-format-check-20260206-024927.log`: 成功（git clone refs 更新警告 / pnpm approve-builds 警告）
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`
  - `tmp/logs/gh-act-native-test-linux-20260206-025046.log`: act/useRouter 警告、ENABLE_P2P_INTEGRATION 未設定による skip あり
