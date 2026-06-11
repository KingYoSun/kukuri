# Third-party notices

kukuri preview builds include Rust crates, npm packages, and Tauri runtime components from
third-party authors.

Release owners must review third-party licenses before publishing a preview build. This file is a
distribution pointer, not a generated dependency inventory.

## Preview release review

1. Generate the Rust dependency list from the release tag:

   ```powershell
   cargo metadata --locked --format-version 1
   ```

2. Generate the desktop npm dependency list from the release tag:

   ```powershell
   cd apps/desktop
   npx pnpm@10.16.1 licenses list --prod
   ```

3. Confirm that every distributed dependency has an acceptable license for a preview build.
4. Attach the generated inventories or their archive location to the draft GitHub Release.
5. Update this notice if a dependency requires attribution text to be included directly in the
   distribution.

## Current distribution note

The first preview targets Windows installer distribution through GitHub Releases. Linux remains
source-run only for this preview scope. If Windows code signing is not configured, the release
notes must state that the preview is unsigned and that SmartScreen warnings are expected.
