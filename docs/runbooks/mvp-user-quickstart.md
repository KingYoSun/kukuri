# MVP User Quickstart

## Audience

- builder preview を触る desktop user 向け
- 現在の packaged target は Windows
- Linux は source-run のみ

## 3 Minute Path

1. GitHub Releases から Windows installer を取得して起動する。
2. 初回起動後、settings を開かずに数秒待ち、Community Node が `ready` になるのを待つ。
3. starter topic のどれかを開く。
   - `kukuri:topic:demo`
   - `kukuri:topic:iroh`
   - `kukuri:topic:nostr`
   - `kukuri:topic:operators`
4. public post か thread reply を 1 本試す。
5. 同じ topic 配下で private channel を作るか参加する。
6. settings の Community Node diagnostics を確認し、feedback を送る。
7. `Settings -> Release` で更新状態を確認し、診断レポートをコピーまたは書き出す。

## What To Notice

- topic が主軸で、channel は topic 配下の audience になっているか
- Community Node が relay ではなく bootstrap / auth / connectivity assist として見えるか
- 自動認証と自動 consent accept の導線が前面に出過ぎず、それでも friction を減らせているか

## Source Run On Linux

```bash
cargo xtask doctor

cd apps/desktop
npx pnpm@10.16.1 install
npx pnpm@10.16.1 dev
```

起動後の見るポイントは Windows preview と同じです。

## Feedback

- `Settings -> Release` の diagnostic report と一緒に GitHub へ feedback を送る
- diagnostic report は secret key、auth token、private channel secret、invite/share token、DM 本文、ローカル DB path を既定で含まない
- 特に聞きたいのは次の 3 点です
  - topic-first の感触が最初に伝わったか
  - topic 配下の channel が自然に感じられたか
  - Community Node の役割境界が理解しやすかったか

## Updates

- preview の更新確認は `Settings -> Release -> Check` で行う。
- 更新が見つかったら `Install` を押し、インストール完了後にアプリを再起動する。
- 更新後も identity、local DB、Iroh data、Community Node 設定、private channel capability、通知 inbox が残っていることを確認する。

## Data Safety

- `Settings -> Release` links to the release runbook and third-party notices.
- Preview update smoke must confirm identity, local DB, Iroh data, Community Node settings, private channel capability, and notification inbox state are preserved.
- Before uninstall or reset, keep the app data directory if the user needs to retain local state.
