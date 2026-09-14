# Metaverse全画面の描画・高さ・補助面

- Status: current
- Supersedes: None（#1021のcamera方式、#1022の通常幅配置は維持）
- Superseded by: None
- PR: [#1029](https://github.com/KingYoSun/kukuri/pull/1029)
- 対象利用者・目的: 入室済みdesktop利用者が全画面の高さを3Dに使い、元のColumnへ文脈を保って戻る。
- 変更分類: 全画面描画の不具合修正、全画面の高さ・補助面配置改善。
- 採用: 同じscene/session/フォームを保持し、入室済みfullscreenだけ補助面を明示開閉する。discovery/管理と接続/hostingを上下の独立scroll領域として右側へ重ねる。閉じた補助内容はhiddenでTab対象外。通常時のDOMはdiscovery/管理→scene→接続/hostingの順。
- 比較: 常時下置きは3Dの高さを使えないため不採用。別画面移動は文脈保持を難しくするため不採用。#1024のカテゴリ・サークルメニューは本変更へ混ぜない。
- 条件: Windows Tauri/WebView2 Edge 152、通常1280×840 client、全画面2560×1440、日本語/light、world version7、owner-hosted。Ubuntu24 Remote Desktopでは本番frontend＋mock DesktopApiのWebKitGTK fixtureを併用。
- 操作: Windowsでメニュー全画面、pointer lock→Enter chat、実Escape退出、draft・focus・本文/Canvas scroll復元。browserはStream/Metaverse往復、tools開閉、Canvas identity、入力保持、通常Tab順、800×600でchat/閉じるへの到達を確認。
- 性能: 全画面ownerだけを可視とし、覆われたColumnは既存resource縮退へ渡す。network lifecycleとaudio focusは独立。resource budget/dpr契約を変えず、GPU定量測定は未実施。
- Validation: 詳細は[作業報告](../progress/2026-09-15-1023-metaverse-fullscreen.md#検証記録)。Storybook build、targeted Vitest、browser340件＋小画面1件、Windows visual smoke38件成功。全体suiteのtimeoutとCI結果は作業報告で区別する。
- 未確認: 実screen reader、物理touch、200%の実WebView zoom、Windows High Contrast、定量GPUメモリ、Linux実network入室。Linuxのキャプチャ範囲による寸法未確認をWindows/ブラウザ成功と混同しない。
- Review result: 初回独立監査の通常Tab順Regressionは修正。最終headのdelta監査とCIをmerge条件にする。
- Exceptions: 製品契約の免除なし。上記未確認事項を成功扱いにしない。

![Windows全画面](../progress/assets/2026-09-15-1023-fullscreen/windows-after-fullscreen.png)

![Windows復帰とdraft](../progress/assets/2026-09-15-1023-fullscreen/windows-after-return-draft.png)
