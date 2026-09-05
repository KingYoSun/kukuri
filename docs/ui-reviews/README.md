# UI Review Records

このdirectoryは、採用済みUI decisionと、その判断に使った条件・証跡を保存する。media archiveではなく、後続変更が「何を維持し、何をsupersedeするか」を判断するためのdurable recordである。

## Recordを追加する条件

追加の要否は[ADR 0014のUI review record](../adr/0014-uiux-dev-flow.md#9-ui-review-record)に従う。本書は採用記録のschemaと履歴管理を所有する。単一文言や既存契約内の局所修正はPRの証跡で足りる。

## File naming

`YYYY-MM-DD-slug.md`を使う。

## Statusとsupersede

- `current`: 記録した判断が現在も有効。
- `superseded`: 後続recordが同じ対象の判断を置き換えた。
- `supersedes`: このrecordが置き換えるrecordへの相対link。なければ`None`。
- `superseded-by`: `superseded`へ変更するとき、置換先recordへの相対linkを追加する。

既存recordの当時の判断、証跡、例外は書き換えない。置換時はstatusと`superseded-by`だけを追記し、新record側から`supersedes`で参照する。対象surfaceやdecisionが異なるrecordを、日付が古いという理由だけでsupersedeしない。

## 必須項目

対象変更に適用される確認結果は、同条件のPR証跡へのリンクで示してよい。非該当項目は省略できるが、必要な確認の未実施・失敗・例外は理由付きで残す。

- Status、Supersedes、Superseded by
- PR linkまたはidentifier
- PR上で読めるpreview imageまたは短い動画
- 対象surface、利用者、単一目的、採用した変更の要約
- Platform、viewport、theme、locale、state
- Accessibilityと実操作の結果
- 性能影響と計測、または対象外理由
- Storybook、Vitest、Playwright、`cargo xtask`、Tauri実機等のvalidation
- 未確認事項
- review resultと承認された例外

## Template

```md
# YYYY-MM-DD slug

- Status: current
- Supersedes: None
- Superseded by: None
- PR:
- Preview:
- Surface / user / purpose:
- Summary:
- Conditions:
  - Platform:
  - Viewport:
  - Theme:
  - Locale:
  - State:
- Accessibility / interaction:
- Performance:
- Validation:
- Not verified:
- Review result:
- Exceptions:
```

previewは原則PRに置く。repository内assetを残す場合も、recordから相対linkで到達できるようにする。
