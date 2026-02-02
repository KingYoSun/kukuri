# Community Node 採用ノード設定（複数ノード/role別）対応 2026年02月02日

## 目的
- クライアントの採用ノード設定を複数ノード/role別に拡張し、label/trust/search/bootstrap をノード単位で切替できるようにする。

## 対応内容
- DTO/Handler を nodes 配列 + roles 構成へ移行し、認証/トークン/consent/keys/invite を base_url 指定で扱えるように整理。
- UI/Store を nodes/roles 前提へ更新し、採用ノードの追加/削除/切替と role トグルを追加。
- label/trust/search/bootstrap の有効判定を nodes/roles から算出するよう PostCard / Search を更新。
- E2E/Unit/Bridge を新しい data-testid と API 形状に合わせて更新。

## 検証
- `./scripts/test-docker.ps1 rust`
- `./scripts/test-docker.ps1 ts`
- `gh act --workflows .github/workflows/test.yml --job format-check`
- `gh act --workflows .github/workflows/test.yml --job native-test-linux`

## メモ
- Vitest 実行中に act/useRouter 警告が出たが、いずれもテストは成功。
- `gh act` は初回 format-check で rustfmt/Prettier 差分が出たため修正後に再実行し成功。
