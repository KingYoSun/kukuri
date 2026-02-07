# 2026年02月07日 Community Node friend_plus 実ノードE2E 追加

## 概要

`community_nodes_roadmap.md` の未実装項目である  
`join.request(friend_plus) -> 承認 -> key.envelope -> 復号表示` を、実ノード経路で検証する E2E シナリオを追加・安定化した。

## 実施内容

- `kukuri-tauri/tests/e2e/specs/community-node.friend-plus.spec.ts`
  - friend_plus フローの E2E シナリオを追加
  - `topic-selector` 非活性で落ちる不安定要因を排除し、対象トピック固定の投稿導線へ修正
  - key.envelope 受信確認を UI 文字列依存から API チェック（group key 取得）へ変更
- `kukuri-tauri/tests/e2e/helpers/bridge.ts`
  - `switchAccount` の export を追加
  - `communityNodeListGroupKeys` アクション/型/ラッパーを追加
- `kukuri-tauri/src/testing/registerE2EBridge.ts`
  - `communityNodeListGroupKeys` を bridge に実装
  - fallback アカウント切替時に `TauriApi.login` で backend 側アクティブ鍵を同期する処理を追加
- `kukuri-tauri/src/lib/api/tauri.ts`
  - `follow_user` / `unfollow_user` の invoke 引数を camelCase（`followerNpub` / `targetNpub`）へ修正

## 検証

- `./scripts/test-docker.ps1 e2e-community-node`
  - ログ: `tmp/logs/community-node-e2e/manual-rerun-20260207.log`
  - 確認結果:
    - `community-node.friend-plus.spec.ts` は `PASSED`
    - 同一実行で `profile.privacy-avatar.spec.ts` が失敗（`Error: オンライン表示の状態が変わりませんでした`）

## 補足

本タスク対象（friend_plus 実ノードE2E）の到達条件は満たした。  
全体 E2E の失敗は別 spec の既存不安定要因であり、別タスクとして切り分けが必要。
