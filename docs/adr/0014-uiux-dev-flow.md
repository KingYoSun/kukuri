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
- `apps/desktop/src/styles/tokens.css`は実行値、Storybook Foundationsは描画確認面であり、どちらも単独で設計契約を上書きしない。

### 2. 変更分類

実装前に次のいずれかを選び、小規模改善と再設計を同じ変更へ混ぜない。

| 分類 | 例 | 必要な事前成果物 |
|---|---|---|
| 不具合修正 | focus消失、誤ったempty、overflow、操作不能 | failing testまたは再現手順、期待結果、維持する既存挙動 |
| 既存画面の改善 | 文言、token、state追加、component改善 | 対象利用者、単一目的、主要操作、非目標、対象state |
| 新規画面・導線 | 新しいColumn、Dialog、multi-step flow | 短い設計契約、全state、responsive構造、戻る／回復、データ境界 |
| UI構造／design language再設計 | navigation、Column構造、theme体系 | 複数案、採用理由、移行範囲、DESIGN更新、UI review record、旧方針のsupersede |

### 3. 実装前brief

意味のある変更では、コードを触る前にPR本文、plan、Issueのいずれかへ次を残す。単一文言修正等、項目が該当しない小変更は理由を付けて対象外にできる。

- 対象利用者と利用文脈
- 画面またはflowの単一目的
- 主要操作と期待結果
- 維持する既存挙動、scope、focus、draft、scroll、戻る文脈
- 非目標
- component stateと画面state
- Desktop／Mobileの構造、入力手段、Tauri／WebView固有条件
- 取得データ、partial／offline／reconnecting／degraded時の扱い

着手前に`DESIGN.md`、関連ADR、既存token、代表component、Storybook、現行画面、対象platformを確認する。既存UIの全面置換を暗黙の前提にしない。

### 4. 実装規則

- fixtureは実内容に近い長さ、locale、stateを使い、lorem ipsumや理想的な短文だけで判断しない。
- 既存primitive、variant、composite、新規primitiveの順に検討する。新規tokenまたはshared componentは利用側より先に契約を定義する。
- data取得とpresentationを分け、見た目の都合でdomain／IPC contractを変えない。
- 独立取得は並列化し、partial failureを画面全体のfailureへ拡大しない。
- 重いStream／Metaverseは必要時に読み込み、画面外resourceの縮退とinput ownershipを設計する。
- localStorageはversion付きschemaとし、global listener、subscription、intervalはownerとcleanupを持つ。
- 外部skillや一般則は参考資料であり、既存ADR、kukuriの製品判断、Tauri／WebView制約を上書きしない。

### 5. 確認優先順

確認と修正は次の順で行う。後順位の装飾で前順位の欠陥を覆わない。

1. 安全性、データ整合性、Accessibility
2. Interactionと操作結果
3. State handling、状態継続、回復可能性
4. Responsive、platform、入力手段
5. Performance
6. Visual consistencyとkukuri固有性
7. Motionと装飾

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

視覚確認は「対象条件を一括確認 → 検出事項を一括修正 → 原則1回再確認」で終了する。再確認でAcceptance Criteriaを満たし、重大な未解決事項がなく、validationが成功したらpolishを停止する。

目的達成に不要な好みの変更、全面的な見た目調整、別surfaceの問題は同じ変更へ追加しない。安全性、データ損失、重大な回帰でなければ、file:lineと根拠を持つ追跡Issueへ分離する。

### 11. 例外

- 非該当項目は空欄にせず、対象外理由を書く。
- DESIGNまたは本ADRからの例外はPRに明記する。
- 受け入れ済みproduct directionを変える例外は、関連ADRとUI review recordを更新する。
- 外部skillのinstall数、実行有無、audit score自体を完了条件にしない。repository内の成果物と観測結果で判定する。

## Consequences

- UI変更の重さに応じて必要な証跡が変わり、小変更へ再設計と同じ手順を課さない。
- P2P／非同期state、境界content、実入力、Accessibility、性能が実装前briefとvalidationに現れる。
- PR template、UI review record、Storybook、Playwright、Tauri実機が同じ対象条件を共有する。
- 視覚確認に停止条件ができ、確認不足と終わりのないpolishの両方を避けられる。
