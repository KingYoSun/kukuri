# Issue #665 Community Node trust / relation UI review

- Branch: `codex/issue-665-trust-relation-ui`
- Preview: [著者詳細の advisory / relation / neighbors](assets/2026-08-14-issue-665/community-node-advisory.png)
- Preview: [Community Node ごとの distance opt-out](assets/2026-08-14-issue-665/distance-optout.png)
- Summary: 著者を選択したあと、ユーザーが対象 Community Node を選んで明示的に読み込んだ場合だけ trust advisory、relation proximity、近いユーザーを表示する。distance opt-out は Community Node 設定内で現在値と node-local 境界を読み込んでから設定・解除する。
- Review result: 承認。420px の著者ペインで長い URL / pubkey と basis 展開が横にはみ出さないこと、連続値が固定バケットへ変換されないこと、node-local / 非 network-wide の説明、opt-out がプライバシー・block・graph 離脱ではない説明を確認した。
- Exceptions: なし。
- Validation: Storybook `Core/AuthorDetailCard/CommunityNodeAdvisory`、`Settings/CommunityNodePanel/DistanceOptout`、Vitest、typecheck、lint、Storybook build、client-perspective harness。

## User flow

1. 著者詳細を開く。既定では Community Node read を実行せず、投稿・著者を隠さない。
2. 購読・設定中の Community Node を選び、「情報を読み込む」を実行する。
3. trust の連続値と issuer / category / severity を確認し、basis を展開して confidence / visibility / appeal status / expiry / decay / relation weight / contribution を確認する。
4. relation proximity とその basis、同じ操作で取得した「近いユーザー」を確認する。これらは timeline / search / recommendation の自動絞り込みには使わない。
5. Community Node 設定では、distance opt-out の説明を読み、現在値と node-local の最小 proximity を取得してから有効化する。必要なら同じ場所で解除する。
6. `RELATION_NOT_FOUND` は原因を推測せず generic unavailable と表示する。trust 未設定・未有効はそれぞれ回復可能な案内へ分ける。

## Shneiderman checklist

- Consistency: 既存の Card / Notice / Button / SettingsActionRow と node 単位の設定カードを再利用した。
- Shortcuts: 著者詳細を閉じずに node 選択・再読み込み・basis 展開まで完結する。
- Informative feedback: loading、連続値、basis、未提供・未有効・unavailable、opt-out 現在値と境界をその場に表示する。
- Dialog closure: 読み込み結果は同じ著者カード内、設定結果は同じ node カード内に収め、操作完了を明確にする。
- Error prevention: opt-out は現在値を読み込むまで Enable を無効化し、未保存 node / editor dirty 時は node 操作を無効化する。
- Easy reversal: opt-out は Enable と Disable を同じ場所に置き、解除後の状態を即時表示する。
- Internal locus of control: advisory / relation / neighbors はユーザーの明示操作だけで取得し、既定の自動抑制へ接続しない。
- Reduce short-term memory load: node-local 性、非プライバシー性、境界、根拠を操作箇所の近くへ常時表示する。
