# Issue #1001 Neutral / Teal dark theme

- Status: superseded
- Supersedes: [Graphite / Orange](2026-09-12-issue-996-graphite-orange-theme.md)のdark配色のみ。light、Column構造、明示選択・保存、solid surface、contrast品質の判断は継承する。
- Superseded by: [primary塗りボタンとカラム上辺（該当する判断のみ）](2026-09-13-issue-1003-orange-primary-buttons.md)。その他のdark配色・focus等は継承する。
- PR: [#1002](https://github.com/KingYoSun/kukuri/pull/1002)
- Preview: [dark変更前](../progress/assets/1001/before-columns-dark-1600.png) / [dark変更後](../progress/assets/1001/after-columns-dark-1600.png)、[light変更前](../progress/assets/1001/before-columns-light-1600.png) / [light変更後](../progress/assets/1001/after-columns-light-1600.png)
- Surface / user / purpose: desktopのdark配色／閲覧・投稿・設定を継続する利用者／無彩色の面と青緑の主要操作で読みやすい階層を示す。
- Summary: ユーザーが選択・承認したNeutral / Teal案を採用。背景`#121212`、パネル`#292929`、primary`#03dac5`、本文`#ffffff`。primary内は白でなく`#00332e`、補助文字`#b3b3b3`。既存Graphite / Orangeと初回deep-teal案の比較・採用理由は作業記録に集約。
- Conditions: dark/light、browser 1600×1000 / 390×844、日本語、既存visual/localizationケース。native OS・viewport・操作は作業記録を参照。
- Accessibility / interaction: 実用途ペアと合成ringを検証し、操作識別境界を装飾線から分離。dark/light切替、draft・Column文脈・focus復元を既存browser testで保護。意味色とlabelを維持。
- Performance: token値と確認面のみ。DOM・handler・motion・data取得の変更なし。
- Validation / Not verified / Review result: 配色案はユーザー承認済み。実contrast、270 browser tests、Linux視覚比較、Storybook描画、Windows/Ubuntu24の切替・保存復元・200%表示・同意未選択表示を確認した。ローカル全体testのtimeoutと対象再実行・CI補完、補助的な未確認範囲は[作業記録](../progress/2026-09-13-1001-neutral-teal-dark-theme.md)へ集約する。最終headの全CI成功後にmergeする。
- Exceptions: なし。生成画像を実装後の証拠へ流用しない。
