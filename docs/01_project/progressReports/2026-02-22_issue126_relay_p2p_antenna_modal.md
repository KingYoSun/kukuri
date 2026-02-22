# Issue #126 Relay/P2P情報アンテナモーダル移設レポート

作成日: 2026年02月22日

## 概要

- 対象:
  - `kukuri-tauri/src/components/layout/AntennaStatusDialog.tsx`
  - `kukuri-tauri/src/components/layout/Header.tsx`
  - `kukuri-tauri/src/components/layout/Sidebar.tsx`
  - `kukuri-tauri/src/tests/unit/components/layout/Header.test.tsx`
  - `kukuri-tauri/src/tests/unit/components/layout/Sidebar.test.tsx`
- サイドバー下部に常設されていた Relay/P2P 情報を除去し、ヘッダーのアンテナアイコンから開くモーダルへ移設した。
- モーダル内で `RelayStatus` と `P2PStatus` を同時表示し、既存情報の閲覧内容を維持した。

## 実装詳細

- `AntennaStatusDialog` を新規追加。
  - ヘッダーに表示するアンテナボタンを実装。
  - ボタンクリックで `Dialog` を開き、`RelayStatus` と `P2PStatus` を表示。
  - テスト用に `open-network-status-button` / `network-status-modal` の `data-testid` を付与。
- `Header`
  - ステータスアイコン群に `AntennaStatusDialog` を追加。
- `Sidebar`
  - `RelayStatus` / `P2PStatus` の import と表示を削除。
  - 下部アクションは設定ボタンのみを維持。
- テスト更新
  - `Header.test.tsx` にアンテナボタン表示とモーダル開閉テストを追加。
  - `Sidebar.test.tsx` はサイドバー内ステータス表示前提を除去し、設定ボタン表示の確認へ更新。

## 検証

- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job format-check`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job native-test-linux`
- `NPM_CONFIG_PREFIX=/tmp/npm-global gh act --workflows .github/workflows/test.yml --job community-node-tests`

すべて pass。
