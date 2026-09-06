# Native corresponding sources

`ubuntu-source-manifest.json` maps the final AppDir packages to exact Ubuntu source
package versions and their `.dsc` / archive checksums. `static/` contains the fixed
runtime sources, dependencies, Alpine patches/recipes, and original notices.
The companion native-notices archive includes copyright texts, common licenses,
and the public schemas, wrappers, hooks and xdg-mime shipped in the AppDir.

The type2-runtime source is commit `75849dce7cc37e4319b633df1f116ca895c71a12`.
Its original `BUILD.md`, scripts and build workflow describe the runtime build.
For the recorded x86_64 build, generate `src/runtime/version` with
`https://github.com/AppImage/type2-runtime/commit/75849dc`, then follow
`ALPINE_ARCH=x86_64 scripts/chroot/chroot_build.sh`. Use the supplied source archives
and fixed Alpine recipes rather than resolving the moving continuous/stable refs.
The supplied libfuse 3.15.0 receives `patches/libfuse/mount.c.diff` with `patch -p1`
as recorded in `scripts/common/install-dependencies.sh`; all runtime source and
build files are supplied so a modified libfuse can be rebuilt and relinked.

The runtime provenance is tied to the reviewed binary by SHA-256, allowing only
the 16-byte `.digest_md5` section filled by AppImage assembly to differ.
The recorded upstream build is https://github.com/AppImage/type2-runtime/actions/runs/28063784345 .
Its Alpine index revision and source checksums are in `runtime-source-manifest.json`.

To inspect or replace bundled shared libraries, extract the AppImage with
`./kukuri_<version>_amd64.AppImage --appimage-extract`, modify/rebuild the relevant
library using its corresponding sources, and run `squashfs-root/AppRun`.
This distribution does not prohibit modification or reverse engineering for
debugging modifications permitted by the included library licenses.

Source and notice collection is not a claim that every collected package has the
same license. Refer to each component's original copyright/license files.
