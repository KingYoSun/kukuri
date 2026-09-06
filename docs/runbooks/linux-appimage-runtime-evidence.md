# AppImage同梱物と公開時の引渡し条件（#889）

## 対象と判定

2026-09-07に、検証用AppImageのAppDir・build host・Tauri cache・上流配布情報を照合した。
対象生成はUbuntu 22.04／Tauri CLI 2.11.4、最新通知版のAppImage SHA-256は
`68dd6ae6bb7dd06fbd9b97ce42bcbd23d307f78a92618a2f622e14aef3cbcab9`。
実行libraryと非ELFの範囲・出所は以下の有限集合として記録する。
これは公開Releaseの法的承認や全codecの動作保証ではない。

## 同梱構成

| 集合 | 観測・出所 | 証拠と扱い |
| --- | --- | --- |
| AppDir ELF 175件 | first-party実行file、build ID一致の116 binary packages（90 source packages）、AppRun.wrapped | `appimage_runtime_inventory.py`で版・source版・copyright原文を保存。first-partyのRust/npmは`docs/THIRD_PARTY_NOTICES.md`を併用 |
| AppRun.wrapped | SHA-256 `f30140a43a0a59e46db21bdefdf749b9e9f2c6946e92afabbacf98b8ae73fb4f` | [Tauri公式mirror](https://github.com/tauri-apps/binary-releases/releases/tag/apprun-old)のasset digestと一致。AppImageKit AppRunのMIT notice対象。厳密なupstream commitは未確定と明記 |
| 外側runtime | バイナリ内の`type2-runtime/commit/75849dc` | [固定source](https://github.com/AppImage/type2-runtime/tree/75849dc)のLICENSE・Dockerfile・依存取得script・Makefileと照合。AppDir ELFとは別に扱う |
| GTK hook生成元 | Git blob `e274632a440985239e0d1f419a4f3f32067463f3` | [公式commit b5eb8d05](https://github.com/tauri-apps/linuxdeploy-plugin-gtk/tree/b5eb8d05b4c0ed40107fe2158c5d8527f94568ef)のscriptと一致。MIT原文を保持 |
| xdg-mime | xdg-utils `1.1.3-4.1ubuntu3~22.04.1` | host fileとSHA-256 `13fabfe59c74aa2f52521b4403ff1a512a152de4d15d98fe9b61c7537dd54d8d`が一致。script内のMIT／Expat noticeを保持 |

非ELF 67件は、AppRun wrapper 1、GTK hook 1、xdg-mime 1、first-party icon 2、
生成desktop entry 1、GTK／pixbuf生成cache 2、package copyright 17、schema関連42。
schema42はbuild hostの対応fileと全件hash一致し、`gschemas.compiled`はその集合の生成物。
追加の非ELF ownerは次のとおり。collectorはschema／xdg-mimeの内容一致を検査し、
対応packageのcopyright・source版をELFと同じartifactへ保存する。

| owner | 観測したbuild host版 | 対象 |
| --- | --- | --- |
| gsettings-desktop-schemas | 42.0-1ubuntu1 | GNOME schemas（LGPL-2.1+） |
| libgtk-3-common | 3.24.33-1ubuntu2.2 | GTK schemas |
| libgtk-4-common | 4.6.9+ds-0ubuntu0.22.04.2 | GTK4 schemas |
| tilix | 1.9.4-2build1 | 公開schema（MPL-2.0）。kukuri固有機能や秘密情報ではない |
| libglib2.0-dev | 既存ELF inventoryの版 | DTD |

symlink33件はicon／desktop alias 3件とGTK input／pixbuf／printbackend library alias30件。
すべてAppDir内の相対参照で、別の独立libraryを追加するものではない。
GStreamer core／base library10件は含むがcodec plugin directoryはない。
libc、GL／EGL／DRM等のホスト解決と実codec利用可否は、同梱済み・動作保証と区別する。

## 外側runtimeのstatic依存

[Makefile](https://github.com/AppImage/type2-runtime/blob/75849dc/src/runtime/Makefile)は
squashfuse、zstd、zlib、fuse3、mimallocをstaticリンクし、Dockerfileはmuslを使用する。
バイナリからsquashfuse 0.5.2、libfuse 3.15.0、zstd 1.5.6、zlib 1.3.2を識別した。
musl／mimallocの厳密なバイナリ版は未確定で、ビルド記述から版を推定確定しない。
**mimallocは上流runtimeのLICENSE列挙だけでは漏れるため、独立したMIT notice対象とする。**
[mimallocの参照原文](https://github.com/microsoft/mimalloc/blob/v2.1.7/LICENSE)の版を
実バイナリ版として記録しない。

## #890で公開前に満たす条件

- AppRun／linuxdeploy wrapper／GTK hook／runtimeとstatic依存（mimallocを含む）のcopyright・license本文を公開成果物へ添付する。ELF用collectorだけでstatic依存網羅としない。
- runtime-evidenceの全packageについて、同梱fileに適用される条件と対応source package／版を基にsource提供物・方法を確定する。単なるpackage内GPLラベルで全libraryを一律に分類しない。
- LGPL対象には必要な対応sourceと変更・再リンク可能な提供形態を用意する。特に[libfuse 3.15.0](https://github.com/libfuse/libfuse/blob/fuse-3.15.0/LGPL2.txt)のsource、runtime `75849dc` の適用patch・source・build手順を含める。上流リンク集だけで提供完了とはしない。
- MPL対象Tilix schemaとLGPL対象GNOME／GTK schemaのsource XML、notice、licenseを保持する。
- musl／mimallocの実配布物に対応するnoticeを照合し、不確実性が残る場合は公開前に対応source/build provenanceを確保する。必要なら由来を固定してruntimeを再生成する。
- 公開する最終artifactで同じ収集・照合を行い、#889の検証専用一時鍵の成果物をそのまま利用者向け更新へ転用しない。

#889は構成検証・生成入口・引渡し条件の明示まで、#890は公開する最終成果物への
notice／source添付とRelease集約を所有する。公開準備の未実施を隠さず、
`redistribution_approved: false`を自動的な承認へ変更しない。
