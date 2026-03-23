# UI Review Records

この directory は、採用済み UI review decision の短い record を保存する。

## Record を追加する条件
- merge された PR が user-facing UI behavior または layout を大きく変える。
- merge された PR が reusable design rule を追加または変更する。
- merge された PR が `docs/DESIGN.md` の例外付きで承認される。

## File Naming
- `YYYY-MM-DD-slug.md` を使う。

## 必須項目
- PR link または identifier
- 一次 review に使った Figma link
- 採用された変更の short summary
- review result
- 承認された例外
- Storybook、Vitest、Playwright、`cargo xtask` などの validation note

## Minimal Template
```md
# YYYY-MM-DD slug

- PR:
- Figma:
- Summary:
- Review result:
- Exceptions:
- Validation:
```

preview image は原則 PR に置く。この directory は media archive ではなく durable な decision record を置く。
