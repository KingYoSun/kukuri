# アカウントメニューと初回プロフィール設定

- Status: current
- Supersedes: None
- Superseded by: None
- PR: https://github.com/KingYoSun/kukuri/pull/1006
- 対象: Control Center左隣のアカウントメニュー、追加／logout確認Dialog、初回プロフィールDialog。
- 目的: 通常画面から本人プロフィールとアカウント操作へ短く到達し、新規accountの初回設定をCN同意／skip後に案内する。
- 判断: ユーザー承認のScope revision v3を採用。新規作成とimportは追加Dialogに同居し、logoutは保存データを削除しない。別accountの内容を混ぜない。

## 条件と証跡

| 環境 | 観測内容 | 画像 |
| --- | --- | --- |
| Windows / WebView2、1280px、ja、dark/light | 初回CN skip→profile、保存、menu、追加、複数account、logout説明、last logout→restart→暗号化鍵再importで本人profile復帰 | [初回設定](assets/account-menu-1005/windows-initial-profile.jpg)、[追加](assets/account-menu-1005/windows-account-add.jpg)、[複数account](assets/account-menu-1005/windows-multiple-accounts.jpg)、[logout](assets/account-menu-1005/windows-logout-confirm.jpg)、[復帰](assets/account-menu-1005/windows-reimport-restored.jpg) |
| Ubuntu24 / WebKitGTK、1280px、ja、dark、Remote DesktopをComputer Useで操作 | CN skip→profile保存、新規作成→profile、logoutで直前へ復帰、last logout→restart→暗号化鍵再importで元の名前・ユーザー名を復帰 | [初回設定](assets/account-menu-1005/linux-initial-profile.jpg)、[追加](assets/account-menu-1005/linux-account-add.jpg)、[複数account](assets/account-menu-1005/linux-multiple-accounts.jpg)、[logout](assets/account-menu-1005/linux-logout-confirm.jpg)、[復帰](assets/account-menu-1005/linux-reimport-restored.jpg) |
| Linux/Chromium、ja dark 1280px / en light 390px | menuと追加Dialogのレイアウト、名前と操作の到達性 | `apps/desktop/tests/playwright/__screenshots__/visual.spec.ts/account-*.png`。workflow 34751037087生成 |

- 確認済み操作: pointer、keyboardによるmenu移動／選択／Escape、Dialogのfocus復元、本人カラムのfocusと重複防止、保存失敗時入力保持、CN規約中断・再試行・skipの順序。
- Accessibility: UIAのrole/accessible nameとbrowserのfocus assertを確認。音声読み上げを含む包括的な支援技術認証を行ったとは扱わない。
- 性能: menuは操作時にlocal profile/blobを取得し、非active runtime/endpointを起動しない。avatarは2MB以下の既存画像のみ読み、欠落はfallback。少数accountの実機操作で応答とpendingを確認し、大規模account件数のbenchmarkは対象外。
- 自動検証: AccountMenu/初回profile/下書きのVitest、Playwright操作・locale layout、Storybook、Linux視覚基準、Rust lifecycle/故障回復/未同意HTTP hit0、専用harness scenario。詳細は[実装記録](../progress/2026-09-13-1005-account-quick-menu-plan.md)。
- 例外: なし。通常のテキストprofile保存と共有menu primitiveのtokenを利用する。
- 最終判定: 製品コードの独立監査blockerは解消済み。最終headのCIと実機結果をPR上で対応付け、両方が揃ってからマージする。
