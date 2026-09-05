# ADR 0014: 本番向けUI/UX開発フロー

## Status

Accepted

## Context

kukuri desktopはTauri、React、Tailwind、shadcn/ui、Storybookで構成され、Vitest、Playwright、視覚回帰、localization layout検査、Tauri／WebView実機確認を使い分ける。P2Pアプリでは通常のloading／errorに加え、partial、offline、reconnecting、degraded、local cache保持を誤解なく示す必要がある。

抽象的な原則や一律のchecklistだけでは、実装前の目的、既存挙動、境界state、入力手段、性能、確認済み範囲が残らない。逆に、すべてのUI変更へ同じ重さの確認を課すと、小変更を停滞させ、終わりのないpolishを招く。

## Decision

### 1. 責務

- root [`DESIGN.md`](../../DESIGN.md)は、利用者へ何をどう見せ、どの品質を守るかを定義する製品固有の設計契約とする。
- [`docs/architecture/desktop-ui-implementation.md`](../architecture/desktop-ui-implementation.md)は、CSS、state、data、review surfaceの実装配置を定義する。
- 本ADRは、UI変更をどう作り、どの証拠をもって確認済みとするかを定義する。
- 承認、リスク区分、修正前の再現と独立監査は[Issue運用手順](../runbooks/issue-lifecycle.md)、path別commandの選定は[REFACTORING.md](../../REFACTORING.md)に従う。本ADRはUI固有の成果物と確認条件を補う。
- `apps/desktop/src/styles/tokens.css`は実行値、Storybook Foundationsは描画確認面であり、どちらも単独で設計契約を上書きしない。

### 2. 変更分類

実装前に次のいずれかを選び、小規模改善と再設計を同じ変更へ混ぜない。

| 分類 | 例 | 必要な事前成果物 |
|---|---|---|
| 不具合修正 | focus消失、誤ったempty、overflow、操作不能 | Issue運用手順の「修正前の再現」に従う失敗testまたは実機・視覚の再現証拠、期待結果、維持する既存挙動 |
| 既存画面の改善 | 文言、token、state追加、component改善 | 対象利用者、単一目的、主要操作、非目標、対象state |
| 新規画面・導線 | 新しいColumn、Dialog、multi-step flow | 短い設計契約、全state、responsive構造、戻る／回復、データ境界 |
| UI構造／design language再設計 | navigation、Column構造、theme体系 | 複数案、採用理由、移行範囲、DESIGN更新、UI review record、旧方針のsupersede |

### 3. 実装前brief

操作、状態、layout、設計規則に影響する変更では、コードを触る前にPR本文、plan、Issueのいずれかへ該当項目を残す。単一文言修正等は対象と期待結果を短く示し、非該当欄を省略できる。UI変更分類とIssueのA/B/Cは別の軸であり、文言でも同意等の境界契約を変える場合は区分Cとする。

- 対象利用者と利用文脈
- 画面またはflowの単一目的
- 主要操作と期待結果
- 維持する既存挙動、scope、focus、draft、scroll、戻る文脈
- 非目標
- component stateと画面state
- Desktop／Mobileの構造、入力手段、Tauri／WebView固有条件
- 取得データ、partial／offline／reconnecting／degraded時の扱い

着手前に`DESIGN.md`と関連ADRの適用箇所を読み、影響するtoken、component、Storybook、現行画面、platformを確認する。全surfaceの棚卸しや既存UIの全面置換を小変更の前提にしない。

### 4. 実装規則

- fixtureは実内容に近い長さ、locale、stateを使い、lorem ipsumや理想的な短文だけで判断しない。
- component選定、非同期state、性能とresourceの契約は[`DESIGN.md`](../../DESIGN.md)の4・8・10節、責務配置は[UI実装配置](../architecture/desktop-ui-implementation.md)に従う。新規tokenまたはshared componentは利用側より先に契約を定義する。
- 外部skillや一般則は参考資料であり、既存ADR、kukuriの製品判断、Tauri／WebView制約を上書きしない。

### 5. 確認優先順

確認と修正は[`DESIGN.md`](../../DESIGN.md)冒頭の変更判断の優先順位に従う。装飾より安全性・操作結果を先に確認する要約であり、別の順位表を管理しない。

### 6. 確認方法と証跡

#### 静的確認

semantic HTML、label、accessible name、focus、overflow、locale、event listener、store購読、storage version等を確認する。検出事項は`file:line`、重大度、観測根拠、最小修正を記録する。

#### 描画確認

対象viewport × theme × locale × stateを先に列挙し、まとめて確認する。clipping、overlap、意図しないdocument-level overflow、overlay stacking、focus visibility、背景の誤操作、長文、空値を観測する。

#### 手動操作

変更内容に応じてkeyboard、実pointer、touch、focus順と復元、screen reader、200% zoom／reflow、Windows High Contrast、reduced motionを確認する。drag／swipe／shortcutは実入力と代替手段の両方で同じ結果へ到達することを確認する。

#### 自動確認

- Vitest: component、UI logic、state、CSS／design contract
- Storybookと`@storybook/addon-a11y`: reusable componentの全stateと確認面
- Playwright browser test: component境界をまたぐflow、実pointer／keyboard、state継続
- Playwright visual test: 主要surfaceのlayout／theme回帰
- `localization-layout.spec.ts`: localeとlayoutの組合せ
- Tauri／WebView実機: browserで表現できないfullscreen、native input、resource縮退
- `cargo xtask check`／`cargo xtask test`: 日常integration gate
- `cargo xtask desktop-ui-check`: lint、typecheck、Vitest、Storybook build、browser test、visual test

自動検査の成功だけをAccessibility適合や実機動作の証明としない。実行commandと視覚回帰baselineの手順は[`docs/runbooks/dev.md`](../runbooks/dev.md)を正とする。

### 7. 変更種別別の最低限validation

| 変更 | 最低限必要な確認 |
|---|---|
| 文言／単一token | 対象locale／theme、実contrast pair、対象test、PR preview |
| reusable component／state追加 | 全state Story、keyboard／pointer、addon-a11y、Vitest |
| layout／navigation／multi-component flow | 同条件のbefore／after、複数viewport、Playwright実操作、視覚回帰、state継続 |
| drag／swipe／shortcut | 実pointer／touch／keyboard、代替手段、gesture競合、focus復元 |
| Stream／Metaverse／Tauri-WebView依存 | browser testに加え、対象OS／WebView実機、resource縮退、fullscreen／input ownership |
| design language再設計 | 複数案と採用理由、DESIGN更新、review record、移行範囲、旧方針のsupersede |

表は影響する観点を選ぶ。文字列だけの修正で色・themeが不変ならcontrastの再測定は不要で、表示先localeと文言・overflowを確認する。tokenや色の変更は実contrast pairと影響するthemeを確認する。path別commandとCI条件はREFACTORING.mdに従い、必要な実機確認を未実施のまま確認済みとはしない。

### 8. PR証跡

UI変更のPRには、該当範囲について次を残す。

- 変更分類、対象画面、対象利用者、単一目的、主要操作、非目標
- 対象platform、viewport、theme、locale、state
- 同条件のbefore／after、または新規画面の画像／短い動画
- keyboard、pointer、touch、screen reader、Accessibility検査の結果
- 性能影響と、長い一覧／重いsurfaceの計測または対象外理由
- 実行した自動validation
- 未確認事項と理由
- DESIGNまたは本ADRからの例外理由

public PR readerが特別な権限なしに変更と確認範囲を理解できることを必須とする。

### 9. UI review record

大きなuser-facing behavior／layout変更、再利用可能な設計規則の変更、例外付き承認、design language再設計では`docs/ui-reviews/`へrecordを残す。recordは`current`／`superseded`とsupersede関係、対象条件、証跡、未確認事項を持つ。詳細schemaは`docs/ui-reviews/README.md`に置く。

### 10. 停止条件

視覚確認は「対象条件を一括確認 → 検出事項を一括修正 → 再確認」で行う。原則1回の再確認を目安とするが、失敗が残る場合は必要な修正と影響部分の確認を続ける。Acceptance Criteriaを満たし、重大な未解決事項がなく、validationが成功したらpolishを停止する。回数を満たしただけで成功扱いにしない。

追加発見は[Issue運用手順](../runbooks/issue-lifecycle.md)のScope freezeに従って分類する。固定条件の不足と今回のRegressionは修正し、新要件は別Issue、目的達成に不要なpolishは対象外とする。好みの調整のために追跡Issueを増やさない。

### 11. 例外

- 非該当項目は省略できる。必要な確認の未実施・失敗や、適用される規則からの例外は省略せず理由と影響をPRに明記する。
- 本ADRの原則からの手順調整は担当者が理由と代替証拠を示して判断する。製品契約の変更や必要な検証の免除は、Issue運用手順の承認範囲に従う。
- 受け入れ済みproduct directionを変える例外は、関連ADRとUI review recordを更新する。
- 外部skillのinstall数、実行有無、audit score自体を完了条件にしない。repository内の成果物と観測結果で判定する。

## Consequences

- UI変更の重さに応じて必要な証跡が変わり、小変更へ再設計と同じ手順を課さない。
- P2P／非同期state、境界content、実入力、Accessibility、性能が実装前briefとvalidationに現れる。
- PR template、UI review record、Storybook、Playwright、Tauri実機が同じ対象条件を共有する。
- 視覚確認に停止条件ができ、確認不足と終わりのないpolishの両方を避けられる。
