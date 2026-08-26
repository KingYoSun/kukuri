# MVP User Quickstart

## Audience

- builder preview を触る desktop user 向け
- 現在の packaged target は Windows
- Linux は source-run のみ

## 3 Minute Path

1. [最新の GitHub Release](https://github.com/KingYoSun/kukuri/releases/latest) から Windows installer を取得して起動する。
2. 初回起動後、settings を開かずに数秒待ち、Community Node が `ready` になるのを待つ。
3. 2番目のProfile Columnを開き、`Edit Profile`から表示名、名前、自己紹介、必要なら画像を設定する。
4. 先頭のTimeline Columnへ戻り、starter topic のどれかを開く。
   - `kukuri:topic:demo`
   - `kukuri:topic:iroh`
   - `kukuri:topic:nostr`
   - `kukuri:topic:operators`
5. public post か thread reply を 1 本試す。
6. 同じ topic 配下で private channel を作るか参加する。
7. Explore、Notifications、Messagesの各Columnを開き、主要機能の入口を確認する。
8. settings の Community Node diagnostics を確認し、feedback を送る。
9. `Settings -> Release` で更新状態を確認し、診断レポートをコピーまたは書き出す。

## What To Notice

- topic が主軸で、channel は topic 配下の audience になっているか
- 初期5 Columnからプロフィール設定、探索、通知、メッセージへ迷わず移動できるか
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

- `Settings -> Release` links to the latest release, this quickstart, the release runbook, third-party notices, and the default Community Node disclosures.
- Browser links: [release](https://github.com/KingYoSun/kukuri/releases/latest), [terms](https://api.kukuri.app/terms), [privacy](https://api.kukuri.app/privacy), [external transmission](https://api.kukuri.app/external-transmission), [abuse policy](https://api.kukuri.app/abuse-policy), and [data retention](https://api.kukuri.app/data-retention).
- Preview update smoke must confirm identity, local DB, Iroh data, Community Node settings, private channel capability, and notification inbox state are preserved.
- Before uninstall or reset, keep the app data directory if the user needs to retain local state.
