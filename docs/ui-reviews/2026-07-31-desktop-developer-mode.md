# 2026-07-31 desktop developer mode (WIP 面とデバッグ情報の既定非表示)

- PR: developer mode toggle (issue #417)
- Summary: 設定ドロワーに「Developer」セクションを追加し、developer mode トグル（localStorage `kukuri.desktop.developer-mode`、既定 OFF）を置いた。OFF の間は WIP 面（Live / Game workspace）をタブ・hash route・内部リンクのすべてで隠して timeline へフォールバックし、設定パネル内の診断表示（Connectivity / Discovery / Community Node / Release の diagnostic list・metric grid・diagnostic report）とシェルヘッダーのステータスバッジを非表示にする。手動接続系入力（peer ticket import / 表示、seed peer 編集、community node 設定）と更新・OS 通知操作は OFF でも残す。ON では従来表示と完全一致。
- User flow summary: 設定ドロワー → Developer セクション → 「Enable developer mode」トグル ON で Live / Game タブとステータスバッジが出現し、OFF に戻すと Live / Game 表示中でも timeline へ戻り URL も `/timeline` に正規化される。`#/live` / `#/game` の直接リンクは OFF のとき timeline へフォールバックする。
- Preview: [`assets/2026-07-31-developer-mode/`](assets/2026-07-31-developer-mode/) — `developer-mode-off.png` / `developer-mode-on.png`（Developer セクション両状態）、`shell-developer-off.png` / `shell-developer-on.png`（シェル全体の両状態）。
- Shneiderman checklist:
  - Consistency: トグルは Release / Community Node パネルと同じ label + checkbox 様式。セクション構成・deep link (`?settings=developer`) も既存 settings セクションと同一パターン。
  - Shortcuts for frequent users: `?settings=developer` deep link で直接到達できる。
  - Informative feedback: トグル操作は即座にタブ・バッジ表示へ反映される。更新エラーは OFF でも翻訳済みメッセージを残す（raw detail のみ開発者向け）。
  - Dialog closure: トグルは即時反映で完結し、追加確認は不要。
  - Error prevention: OFF 時の live/game deep link はエラーにせず timeline へ正規化する。
  - Easy reversal: トグルはいつでも往復可能で、localStorage 永続化により再起動後も選択が保たれる。
  - Internal locus of control: 表示変更は user 自身のトグル操作でのみ発生し、hidden mode switch はない。
  - Reduce short-term memory load: WIP 面と診断の表示状態は Developer セクション 1 箇所に集約されている。
- Review result: 既定 OFF で一般ユーザーから WIP 面と細かな診断が隠れ、developer は設定 1 箇所で従来表示を取り戻せる。既存 Vitest / Playwright は test setup 側で developer mode を有効化して従来の前提を維持し、既定 OFF の挙動は専用テストで検証する。
- Validation: `cd apps/desktop && npx pnpm@10.16.1 test`（615 passed）、`npx pnpm@10.16.1 lint`、`npx pnpm@10.16.1 typecheck`、`npx pnpm@10.16.1 test:e2e:browser`（11 passed）、`npx pnpm@10.16.1 test:e2e:visual`（14 passed、baseline 変更なし）、`npx pnpm@10.16.1 storybook:build`、`cargo xtask check` passed.
