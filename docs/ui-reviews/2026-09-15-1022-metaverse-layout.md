# #1022 カラム実幅と作業文脈

- Status: current
- Supersedes: None（#1020の管理、#1021の入力契約を維持）
- Superseded by: None
- PR: 作成後に記録
- Preview: 検証後に同条件before/afterを追加
- Surface / user / purpose: Metaverse参加者・所有者がカラム幅変更後も同じ操作を続ける。
- 承認: 2026-09-15、推奨案Bと実装・PR・CI成功後のmergeをユーザー承認。

## 実装前の採用判断

外枠幅だけをclampする案Aと、同一DOMを実幅で再配置する案Bを比較しBを採用する。修正前browserで1列の本文横overflowが319px、`.metaverse-panel` が741pxまで拡大すること、1→3列後にheader操作部が画面外に残ることを失敗testで確認した。外枠だけの幅指定では内部gridのmin-contentとHUD/chatの重なりを扱えない。

- form／hosting／管理group上限40rem、作成人数欄8rem。実幅32rem未満はtitleと人数を縦積み。
- room card trackは最小 `min(100%, 16rem)`、上限24rem。短いbuttonは内容幅と折返しを基本とする。
- 接続slotは32rem以上で2列、未満で1列。方位順・actionは維持。
- scene内はbadge、toolbar、camera、recovery、HUD、chatを重ならないgrid領域へ割り当てる。実幅50rem未満では1列に積み、同じDOM・開閉状態を維持する。
- HUD上限18rem、chat上限27rem。狭幅は親幅以内、HUD内部は最大16remの縦scroll。送信と閉じるはchat本文のscrollから分離する。
- 3D canvasの既存min-heightは維持。UIを包含するための自然高は許容し、fullscreenの高さ最適化や黒画面修正は含めない。
- span変更の補正はCanvasが所有し、変更前に表示中のactive Columnだけを追従する。手動離脱・非active幅変更では位置を奪わない。

## 確認条件

Windows WebView2／Ubuntu24 WebKitGTK、1283×871相当、span 1/2/3、日本語lightを主条件とする。browserでlight/dark、ja/en/zh-CN、200%相当reflow、759/760境界、4列回帰を補う。

- Accessibility / interaction: 未実施。実pointer・keyboard・focus・hit test・axeを後記。
- Performance: frameごとのReact更新やnetwork追加なし。resize observerとCSS layoutのみを予定。
- Validation: 修正前browser 2件失敗（header非表示、内部横overflow）。修正後は未実施。
- Not verified: 修正後描画、Windows/Linux比較、全体suite、実screen reader、物理touch端末。
- Review result: 寸法と配置を実装前に固定。実装後の合否は別途記録。
- Exceptions: なし。
