# Issue #1003 primary塗りボタンとカラム上辺

- Status: current
- Supersedes: [Neutral / Teal](2026-09-13-issue-1001-neutral-teal-dark-theme.md)のprimary塗りボタン配色とカラム上辺の判断のみ。通知、通常accent、focus、light配色、Column構造は継承する。
- Superseded by: None
- PR: [#1004](https://github.com/KingYoSun/kukuri/pull/1004)。
- Preview: [Windows dark](assets/1003/windows-v2-dark.jpg) / [light](assets/1003/windows-v2-light.jpg)、[Ubuntu24 dark](assets/1003/ubuntu-v2-dark.jpg) / [light](assets/1003/ubuntu-v2-light.jpg)。Linux／Chromiumの同条件before／afterはPRのvisual baseline差分で確認できる。
- Surface / user / purpose: desktopで投稿・検索・閲覧する利用者。主操作のオレンジ色を保ち、カラム上辺の装飾を除く。
- Summary: primary塗りボタンの背景と文字だけ用途別tokenで定義し、dark／lightともオレンジにする。選択・固定カラムの上辺グローを削除する。未読通知数と枠は既存accentへ戻す。Scope revision v2が現在の採用判断。
- Conditions: browser dark／light、1600／390px、native Windows WebView2とUbuntu24 WebKitGTK、日本語。状態・操作と追加画像は作業記録へ集約する。
- Accessibility / interaction: ボタン文字contrastは通常5.84:1、hover4.89:1。既存focus・aria-current・固定ラベルを保持。theme切替・draft・focus復元のbrowser test成功。
- Performance: CSS値のみ。新たなDOM、handler、animation、購読、I/Oなし。
- Validation / Not verified / Review result: ユーザーがv2の変更を指定済み。対象browser 7件、styles contract 125件が成功。Vitest全1615件・Storybook buildが成功。browser全体再実行と最終CIを継続中。現在結果は[作業記録](../progress/2026-09-13-1003-orange-primary-buttons.md)を参照。
- Exceptions: v1画像はv2実装後の証拠へ流用しない。本番アカウント・通信E2Eとmockによるnative描画確認を区別する。
