# Issue #996 Graphite / Orange

- Status: current
- Supersedes: [旧配色の評価](2026-06-13-design-spec-baseline-evaluation.md)、[旧theme palette](2026-03-27-desktop-theme-solidification.md)、[旧light具体色](2026-08-31-issue-828-light-theme-contrast.md)。置換は配色の採用部分のみ。solid surface、明示的な選択と保存、contrast品質は維持する。
- Superseded by: None
- PR: [#997](https://github.com/KingYoSun/kukuri/pull/997)
- Preview: [dark変更前](../progress/assets/996/before-columns-dark-1600.png) / [dark変更後](../progress/assets/996/after-columns-dark-1600.png)、[light変更前](../progress/assets/996/before-columns-light-1600.png) / [light変更後](../progress/assets/996/after-columns-light-1600.png)
- Surface / user / purpose: 全desktopの配色／閲覧・投稿・会話・設定を継続する利用者／中立色の面とオレンジの主要操作で読みやすい階層を示す。
- Summary: Issueで4案比較後に採用された最初のA案「GRAPHITE / ORANGE」のlight／darkを適用。darkのカラム本文は`#212121`。後続の青みcharcoal案（目標`#1C2024`、生成画像`#1F2429`〜`#1F2529`）は不採用。一律グレーの選択色への置換も行わない。[採用参考画像](https://github.com/KingYoSun/kukuri/issues/996#issuecomment-5645455832)の画素揺らぎではなく、Issueの色表と役割を基準にした。
- Conditions: browserは日本語、dark／light、1600×1000／390×844、scale factor 1。Timeline＋Profile＋Explore、表示設定、投稿作成、同等のportal面としてフィードバックDialogを同条件比較。narrowは既存の1 Column＝1 viewportを維持。各OSのnative条件は作業記録を参照。
- Accessibility / interaction: 装飾境界を`--border-subtle`、input／select／textareaの識別境界を`--border-subtle-strong`へ分離。既存意味色・label・iconとkeyboard操作を維持。実使用pairとalpha合成ringの検査を両themeへ拡張。disabledの規格上の例外と、利用可能な操作の識別を区別する。
- Performance: token値と一部の境界参照のみ変更。DOM構造、取得データ、event handler、animationは追加しないため新たな性能計測は対象外。
- Validation / Not verified / Review result: 配色採用は承認済み。`desktop-ui-check`、実contrast、Linux視覚比較、Windows／Ubuntu24のComputer Useによる切替・保存復元・200%表示を確認した。[作業記録](../progress/2026-09-12-996-graphite-orange-theme.md)に実行結果と未実施の補助的確認を集約する。CI／mergeの現在判定はPRを参照。
- Exceptions: なし。生成画像を実装後の証拠へ流用しない。
