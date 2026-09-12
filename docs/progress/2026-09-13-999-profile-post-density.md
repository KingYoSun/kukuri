# #999 プロフィール一覧・投稿・Columnの余白

- Scope revision: 2026-09-13-profile-post-density-v1
- 基準commit: bb500b3940fa329cf5af00fbce8b89df6dcac2bd
- 区分B、既存画面の改善。ユーザーの具体的変更要求に基づく。#991とは別件。
- 実装・最終CI: [PR #1000](https://github.com/KingYoSun/kukuri/pull/1000)。CIの最新判定はPRに集約する。

## 問題と変更

Column本文はpadding16pxに加え内側stackの幅から32pxを引いており、左右16px/48pxだった。stackをcontent boxの100%にして左右16pxへ揃えた。PostCardはヘッダーgap16px、margin-bottom8px、本文margin-top12pxを重ねていたため、4px単位へ統一しpaddingを12pxから8pxにした。

ProfileConnectionsPanelの総称見出しと重複名を除去し、状態バッジを最上段へ移した。右上menuには選択種別以外の2操作を表示し、主操作は情報行の右へ残した。following/followedはfollow、mutedはmute、blockingはblockの既存callbackを使う。既存のIDコピーcontext menuは維持。自己操作は出さない。

| 受入条件 | 実装・証跡 |
| --- | --- |
| AC-1 見出し除去・一覧間余白 | ProfileConnectionsPanel、panelのgrid gap8px、browserのlistGap検査 |
| AC-2 バッジ・副操作menu | profile-connection-header、名前付きIconButton、ContextActionMenu。component/browser menu検査 |
| AC-3 主操作を情報行右 | primaryActionの種別分岐。4種類のcallback/引数・self検査 |
| AC-4 半分以下の間隔 | bio gap12→4px、tab padding16→4/8px、tab gap12→4px。3幅の実効CSS検査 |
| AC-5 投稿の余白統一 | post padding8px、meta gap4px、meta/body margin4px。3幅の正の間隔検査 |
| AC-6 左右均等 | stack width100%。3幅・全Columnの左右差1px以内をbrowser検査 |
| INVAR-1 一覧・関係操作 | 既存shell profile testのBlock入口をmenuに移し、mutation結果とblocking tabの確認を維持 |
| INVAR-2 copy/keyboard/state | 既存IDコピーtest、menu矢印/Escape/focus復元、4タブ・self・state stories |
| INVAR-3 投稿・Column文脈 | PostCardロジック無変更。全体UI suiteとvisualで共有CSSの影響を確認 |

inventoryの変更は行menu入口追加と常設副操作の移動のみ。sinkは既存onToggleRelationship／onToggleMute／onToggleBlockで、API・guard無変更。ProfileConnectionsPanelの本番callerはDesktopShellPrimaryWorkspace。共有post CSSはPostCardとprofile行等、Column本文の幅は全Columnに影響するため全browser/visualを検証する。

## 検証

- 変更前: `ProfileConnectionsPanel.test.tsx -t 'keeps only'`が常設Muteボタンの残存で失敗した。
- 変更前後の画像と実測: [before](assets/999/before-metrics.json)、[after](assets/999/after-metrics.json)、[UI review](../ui-reviews/2026-09-13-profile-post-density.md)。
- 対象Vitest: ProfileConnectionsPanel、DesktopShellPage.profile、2ファイル19テスト成功。
- 対象Playwright: profile-density.spec.ts、1280/1024/390pxの3テスト成功。
- desktop-ui-check初回: lint成功、testにPlaywright専用`exact`オプションを使った型エラーで停止。Testing Libraryの既定一致へ修正し再実行する。
- 全体Vitestの初回は183ファイル中182成功、1611テスト中1610成功。残り1件はsocialGraph testが常設Muteを前提にしていたため、操作入口をmenuへ移し、Unmuteの確認もmenu内で維持。該当ファイル再実行は4テスト成功。製品側の追加変更は不要だった。
- Linux視覚baseline: [Kukuri Visual Baseline 34723248004](https://github.com/KingYoSun/kukuri/actions/runs/34723248004)成功。既存4画像と新規関係一覧3画像を取り込み、投稿・thread・author・exploreの差分を目視確認した。
- Accessibility: 390px、dark/light、通常/長文、panel/menuのaxe検査で違反・要確認0件。[結果](assets/999/a11y.json)。
- ネイティブ描画: 既存の隔離Tauri review hostに現行frontend + mockを表示。Windows WebView2 1280×840のmenu/escape/focus、Ubuntu24 WebKitGTK 1280×840の描画とAT-SPIでmenuの2操作を確認。[Windows](assets/999/windows-native.png)、[Linux](assets/999/linux-native.png)、[Linux menu](assets/999/linux-native-menu.png)。
- Windows hostの200% zoom（CSS viewport 640×420）でも行の横overflowなし。短いviewportでは既存の下部page indicatorが一部情報に重なるため、zoom時の全面的な非重複を確認済みとはしない。グローバルdock配置は今回変更していない。
- `git diff --check`と`cargo xtask oversized-files`成功。修正後の全体desktop-ui-checkとCIの結果はPRの検証記録へ集約する。

Windowsローカルでは視覚snapshot比較がskipされるため、Linux workflowでbaselineを生成・比較する。Computer Useは前回判明したconfiguration errorのため、WindowsはCDP、LinuxはSSH/AT-SPI/Gdkで確認した。RDPウィンドウへの直接入力とscreen reader、配布版backend、実データの関係mutationは未確認。UIロジック・layoutの変更でありbackendの契約は変更していない。
