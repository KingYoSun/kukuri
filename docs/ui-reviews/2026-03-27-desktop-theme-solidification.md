# 2026-03-27 desktop theme solidification

- Figma: なし。既存 shell surface の theme token と settings drawer を更新する visual refinement として扱い、standalone Figma proposal は作成していない。例外として記録する。
- Storybook review surface: `Foundations/Tokens/DesktopWidth`, `UI/Button/Default`, `UI/Card/Default`, `Settings/AppearancePanel/Default`, `Shell/SettingsDrawer/AppearanceOpen`
- Summary: `apps/desktop` の shell token を `dark` / `light` の 2 系統へ再編し、panel・button・badge・notice・settings card・post/media chrome の背景を solid surface に統一した。settings drawer に `Appearance` section を追加し、theme 切り替えを local storage に保存するようにした。`DesktopApi` / Tauri invoke / Rust contract は変更していない。
- User flow summary: 初回起動時は dark theme を使い、settings drawer の `Appearance` から `Light` / `Dark` を即時切り替えできる。選択はこの desktop に保存され、reload 後も維持される。settings deep-link は `?settings=appearance` を受け付ける。
- Review result: shell background、accent panel、ghost/secondary button、settings diagnostics card、post/media placeholder の translucent or gradient background は廃止し、theme ごとの solid token へ置き換えた。Storybook preview には theme toolbar を追加し、light/dark を同じ review surface で確認できるようにした。
- Shneiderman: 一貫性は shared token layer と settings drawer への切り替え導線集約で担保した。informative feedback は active theme の selected state と即時反映で維持した。internal locus of control は automatic system override を入れず、明示的な local choice を保存することで高めた。short-term memory load は settings deep-link と persistent local choice で下げた。
- Validation: `cd apps/desktop && npx pnpm@10.16.1 test`; `cd apps/desktop && npx pnpm@10.16.1 typecheck`; `cd apps/desktop && npx pnpm@10.16.1 lint`; `cd apps/desktop && npx pnpm@10.16.1 storybook:build`; `cd apps/desktop && npx pnpm@10.16.1 test:e2e:browser`
