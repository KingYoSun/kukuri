# 2026-09-11 接続診断の説明と回復導線

- Status: current
- Supersedes: None（#915の翻訳と#943の設定導線を継承）
- Superseded by: None
- PR: [#974](https://github.com/KingYoSun/kukuri/pull/974)
- Preview: [Ubuntu変更前](../progress/assets/issue-959/ubuntu-before.jpg)、[Ubuntu変更後](../progress/assets/issue-959/ubuntu-after.jpg)、[Windows変更後](../progress/assets/issue-959/windows-after.jpg)
- Surface / user / purpose: 接続不調を調べる利用者向け。Control Center、接続全体・topic、discoveryの意味を揃え、現在状態と次の操作を理解する。
- Summary: 接続候補と実績、リアルタイム接続と保存データ配送、過去エラーと現在状態、未取得と未購読を区別。日本語等の要約と回復CTAを主面に置き、raw error / IDはDeveloper modeの詳細に残す。
- Conditions:
  - Platform: Ubuntu 24.04.5 / WebKitGTK 2.52.6（`ssh local2`の`~/kukuri`、既存RDP session）、Windows / WebView2。
  - Viewport: native windowは1280×800相当。browserは1280×844 / 390×844、通常モードの200% zoom。
  - Theme / Locale: nativeはdark / ja、browserは3locale × 2theme。
  - State: nativeはCN ready / peer0 / configured candidate / docs-assist / 初回join timeoutの固定mock。Storyは初回未取得・取得中・初回失敗・前回snapshot・未接続・配送回復・同期履歴・Live・topic欠落・discoveryの10状態と既存Ready / NarrowErrorの計12状態。
- Accessibility / interaction: nativeで更新、設定移動、Tab / Enter / Escapeと戻り先を確認。Ubuntuで移動先の設定navとControl Centerへのfocus表示を確認。browserのpending→失敗→再試行→Live、raw詳細の開閉、未保存ticket保持、mutation API call0を確認。
- Performance: 新規polling / worker / IPCを追加しない。既存snapshotの小さい表示変換と取得stateのみ。重い一覧・media描画への変更は対象外。新規全store購読なし。
- Validation: [作業記録](../progress/2026-09-11-959-connectivity-diagnostics.md)へ最終command・結果を集約。最初の狭幅testではCTAのnowrapによるはみ出しを検出し、狭幅の縦配置・折り返しで同じ15件のbrowser testを成功させた。
- Not verified: 初回join timeout自体の実ネットワーク再現・修復、Debian13。Tauriのmock表示をnetwork成功とは扱わない。実API通常起動は既存profileの規約・年齢確認待ちで診断面へ未到達（同意や申告は変更しない）。screen readerの読み上げ実測・touch端末実測は未実施（今回の操作は既存のbutton / detailsを使用）。
- Review result: コードdelta独立確認PASS。72条件のa11y違反0・要確認0、Linux visual26件成功。最終CIとmerge判定は作業記録・PRに集約する。
- Exceptions: なし。token値は維持。ライトテーマの既存metricラベルcontrast不足を再現し、色付き背景上のラベルだけ既存foreground色へ変更して全72条件を再測定した。通常面の同期診断原文は隠し、既存の別操作エラーも同期状態とは分けて保持する。
