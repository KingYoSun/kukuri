# 2026-09-18 信頼評価による投稿の折りたたみと採用ノードの順位

- Status: current
- Supersedes: None
- Superseded by: None
- PR: 本 record を含む #1061 の PR3（`Refs #1061`）
- Issue / Scope revision: [#1061](https://github.com/kukuri-app/kukuri/issues/1061)、2026-09-15-r2
- Preview: Storybook（`Core/AuthorTrustGateNotice`、`Core/AuthorTrustDisplayExceptionField`、`Settings/CommunityNodeTrustPriorityField`）と Vitest の deterministic mock
- 対象 surface / 利用者 / 目的: タイムライン・スレッド・プロフィール・ブックマーク・live / game 一覧の閲覧者のうち、信頼評価を採用するコミュニティノードを選んだ人。評価の低い作者の投稿を既定で畳み、理由と採用ノードを示したうえで、その場で開く・作者ごとに解除する手段を残す。
- 変更分類: 既存画面の改善（ADR 0014 §2）。共有 component（`PostCard`）への state 追加と、設定画面・作者詳細への欄追加。
- 関連: [#1056 の record](2026-09-16-1056-timeline-advisory-and-node-adoption.md)（ノードごとの採用設定の置き方）と [#1108 の record](2026-09-17-1108-advisory-details-dialog.md)（一覧を圧迫しない代替表示）の方針を踏襲する。

## 採用した表示

一覧では投稿を畳み、断定ラベルは出さない。

- 折りたたんだ投稿は、カードの代わりに短い案内だけを出す。内容は「採用しているコミュニティノードの評価により折りたたんでいます」、理由の種類（リスク判定 / 関係の近い利用者のブロック・ミュート）、判断したノードの URL、「表示する」「作者を開く」。
- 引用・repost は、元投稿の作者が対象のときも畳む。案内の文言だけを「引用元の作者について」に変える（ADR 0022 の repost 非表示と同じ範囲）。
- 「表示する」はその投稿だけに効く。判断・設定・ブロック / ミュートは変わらない。
- live / game 一覧は、主催者が対象のときに同じ案内へ置き換える。「表示する」はその一覧の表示にだけ効く。
- 作者詳細に「この作者を常に表示する」を置く。設定するとその作者は畳まれなくなる。端末内の設定で、評価やブロック / ミュートは変えない。
- 設定 > コミュニティノードに「信頼値で投稿を折りたたむノード」を置く。ノードごとのチェックと上下の並べ替えで採用順位を決める。選ばなければ折りたたみは起きない。

理由は種類だけを示し、誰がブロック・ミュートしたかや件数は出さない（ADR 0026 §8.3 / §8.4）。

## 条件と証跡

- Platform: Chromium（Vitest + Testing Library、deterministic mock）。Windows WebView2 実機は未確認。
- Viewport: 1024 幅（shell 結合テスト）。
- Theme: 既定（dark）。
- 確認した state: 折りたたみ（理由の種類ごと・ノードの有無・引用元）、表示する、採用順位なし（照会も折りたたみもしない）、採用順位の単独・複数・編集中・ノード未設定、作者ごとの例外の未設定・設定済み・読めない・保存失敗。いずれも Story を持つ。

## 検証

- `apps/desktop/src/shell/DesktopShellPage.authorTrustGate.test.tsx`（4 件）
- `apps/desktop/src/shell/authorTrustGates.test.ts`（3 件）
- `apps/desktop/src/components/settings/CommunityNodeTrustPriorityField.test.tsx`（3 件）
- `apps/desktop/src/shell/data/useAuthorTrustGateLookup.test.tsx`（4 件。期限切れ・同意取消・待ち行列）
- `cargo xtask desktop-ui-check`（lint / typecheck / Vitest / Storybook / browser / visual）

## 残っている限界

- 折りたたみの判断は評価の期限（既定 600 秒）で捨てて照会し直す。期限内は同じ判断を使うため、ノード側の評価変更が即時には反映されない。採用順位・認証・必須同意の変更は、期限を待たずに判断を捨てる。
- 作者ごとの例外は端末内の設定で、別の端末には共有されない（mute と同じ扱い）。
