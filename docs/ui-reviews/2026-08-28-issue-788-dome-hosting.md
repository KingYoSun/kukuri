# Issue #788 Dome Hosting UI review

## 対象

Metaverse room の Hosting panel。ownerだけに this-device start、Community Node delegate、closeを表示し、非ownerはread-only noticeとderived stateだけを表示する。

## Shneiderman checklist

- 一貫性: 既存 `Card`、`Button`、`Label`、`Input`、`Notice` と metaverse namespace を再利用。
- フィードバック: pending中はpanel全体を `aria-busy` にし、操作をdisable、失敗はdestructive noticeへ表示。
- 取り消し: Hosting closeと高epochの明示switch-backを提供。自動reclaimはしない。
- エラー防止: Node ID/API URLが空の間はdelegate不可、owner以外は変更操作不可。
- 状態可視性: derived state、lease epoch、expiry、session idを常時表示。
- アクセシビリティ: native label/input/button、keyboard操作、既存focus styleを維持。animationは追加していない。

## 検証

Vitestでhost-authoritative movement/prop input、remote snapshot適用、presence/chat継続、panel維持を確認する。desktop typecheckとStorybook buildをCI validationに含める。
