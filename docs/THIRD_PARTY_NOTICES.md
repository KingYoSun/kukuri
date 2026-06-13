# Third-party notices

kukuri preview builds include Rust crates, npm packages, and Tauri runtime components from third-party authors.

This file is generated from the locked Rust and desktop npm dependency inventories.

Regenerate it from the repository root with:

```powershell
./scripts/release/generate-third-party-notices.ps1
```

Release owners must review these inventories before publishing a preview build and update this generator if a dependency requires attribution text beyond the package-level license inventory.

## Current distribution note

The first preview targets Windows installer distribution through GitHub Releases. Linux remains source-run only for this preview scope. If Windows code signing is not configured, the release notes must state that the preview is unsigned and that SmartScreen warnings are expected.

## Rust crates

Total packages: 632

| Package | Version | License | Source |
| --- | --- | --- | --- |
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 | https://crates.io/crates/adler2 |
| aead | 0.5.2 | MIT OR Apache-2.0 | https://crates.io/crates/aead |
| aes | 0.8.4 | MIT OR Apache-2.0 | https://crates.io/crates/aes |
| aes-gcm | 0.10.3 | Apache-2.0 OR MIT | https://crates.io/crates/aes-gcm |
| ahash | 0.8.12 | MIT OR Apache-2.0 | https://crates.io/crates/ahash |
| aho-corasick | 1.1.4 | Unlicense OR MIT | https://crates.io/crates/aho-corasick |
| allocator-api2 | 0.2.21 | MIT OR Apache-2.0 | https://crates.io/crates/allocator-api2 |
| android_system_properties | 0.1.5 | MIT/Apache-2.0 | https://crates.io/crates/android_system_properties |
| anstream | 1.0.0 | MIT OR Apache-2.0 | https://crates.io/crates/anstream |
| anstyle | 1.0.14 | MIT OR Apache-2.0 | https://crates.io/crates/anstyle |
| anstyle-parse | 1.0.0 | MIT OR Apache-2.0 | https://crates.io/crates/anstyle-parse |
| anstyle-query | 1.1.5 | MIT OR Apache-2.0 | https://crates.io/crates/anstyle-query |
| anstyle-wincon | 3.0.11 | MIT OR Apache-2.0 | https://crates.io/crates/anstyle-wincon |
| anyhow | 1.0.102 | MIT OR Apache-2.0 | https://crates.io/crates/anyhow |
| arcstr | 1.2.0 | Apache-2.0 OR MIT OR Zlib | https://crates.io/crates/arcstr |
| arc-swap | 1.9.1 | MIT OR Apache-2.0 | https://crates.io/crates/arc-swap |
| arrayref | 0.3.9 | BSD-2-Clause | https://crates.io/crates/arrayref |
| arrayvec | 0.7.6 | MIT OR Apache-2.0 | https://crates.io/crates/arrayvec |
| asn1-rs | 0.7.2 | MIT OR Apache-2.0 | https://crates.io/crates/asn1-rs |
| asn1-rs-derive | 0.6.0 | MIT OR Apache-2.0 | https://crates.io/crates/asn1-rs-derive |
| asn1-rs-impl | 0.2.0 | MIT/Apache-2.0 | https://crates.io/crates/asn1-rs-impl |
| async_io_stream | 0.3.3 | Unlicense | https://crates.io/crates/async_io_stream |
| async-channel | 2.5.0 | Apache-2.0 OR MIT | https://crates.io/crates/async-channel |
| async-compat | 0.2.5 | Apache-2.0 OR MIT | https://crates.io/crates/async-compat |
| async-lock | 3.4.2 | Apache-2.0 OR MIT | https://crates.io/crates/async-lock |
| async-trait | 0.1.89 | MIT OR Apache-2.0 | https://crates.io/crates/async-trait |
| atoi | 2.0.0 | MIT | https://crates.io/crates/atoi |
| atomic-polyfill | 1.0.3 | MIT OR Apache-2.0 | https://crates.io/crates/atomic-polyfill |
| atomic-waker | 1.1.2 | Apache-2.0 OR MIT | https://crates.io/crates/atomic-waker |
| attohttpc | 0.30.1 | MPL-2.0 | https://crates.io/crates/attohttpc |
| autocfg | 1.5.1 | Apache-2.0 OR MIT | https://crates.io/crates/autocfg |
| aws-lc-rs | 1.17.0 | ISC AND (Apache-2.0 OR ISC) | https://crates.io/crates/aws-lc-rs |
| aws-lc-sys | 0.41.0 | ISC AND (Apache-2.0 OR ISC) AND Apache-2.0 AND MIT AND BSD-3-Clause AND (Apache-2.0 OR ISC OR MIT) AND (Apache-2.0 OR ISC OR MIT-0) | https://crates.io/crates/aws-lc-sys |
| axum | 0.8.9 | MIT | https://crates.io/crates/axum |
| axum-core | 0.5.6 | MIT | https://crates.io/crates/axum-core |
| backon | 1.6.0 | Apache-2.0 | https://crates.io/crates/backon |
| bao-tree | 0.16.0 | MIT OR Apache-2.0 | https://crates.io/crates/bao-tree |
| base16ct | 1.0.0 | Apache-2.0 OR MIT | https://crates.io/crates/base16ct |
| base32 | 0.5.1 | MIT OR Apache-2.0 | https://crates.io/crates/base32 |
| base64 | 0.22.1 | MIT OR Apache-2.0 | https://crates.io/crates/base64 |
| base64ct | 1.8.3 | Apache-2.0 OR MIT | https://crates.io/crates/base64ct |
| bech32 | 0.11.1 | MIT | https://crates.io/crates/bech32 |
| binary-merge | 0.1.2 | MIT OR Apache-2.0 | https://crates.io/crates/binary-merge |
| bitcoin_hashes | 0.14.100 | CC0-1.0 | https://crates.io/crates/bitcoin_hashes |
| bitcoin-io | 0.1.100 | CC0-1.0 | https://crates.io/crates/bitcoin-io |
| bitflags | 2.11.1 | MIT OR Apache-2.0 | https://crates.io/crates/bitflags |
| bit-vec | 0.9.1 | Apache-2.0 OR MIT | https://crates.io/crates/bit-vec |
| blake3 | 1.8.5 | CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception | https://crates.io/crates/blake3 |
| block2 | 0.6.2 | MIT | https://crates.io/crates/block2 |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 | https://crates.io/crates/block-buffer |
| block-buffer | 0.12.0 | MIT OR Apache-2.0 | https://crates.io/crates/block-buffer |
| bumpalo | 3.20.3 | MIT OR Apache-2.0 | https://crates.io/crates/bumpalo |
| bytemuck | 1.25.0 | Zlib OR Apache-2.0 OR MIT | https://crates.io/crates/bytemuck |
| byteorder | 1.5.0 | Unlicense OR MIT | https://crates.io/crates/byteorder |
| byteorder-lite | 0.1.0 | Unlicense OR MIT | https://crates.io/crates/byteorder-lite |
| bytes | 1.11.1 | MIT | https://crates.io/crates/bytes |
| cc | 1.2.62 | MIT OR Apache-2.0 | https://crates.io/crates/cc |
| cfg_aliases | 0.2.1 | MIT | https://crates.io/crates/cfg_aliases |
| cfg-if | 1.0.4 | MIT OR Apache-2.0 | https://crates.io/crates/cfg-if |
| chacha20 | 0.10.0 | MIT OR Apache-2.0 | https://crates.io/crates/chacha20 |
| chacha20 | 0.9.1 | Apache-2.0 OR MIT | https://crates.io/crates/chacha20 |
| chacha20poly1305 | 0.10.1 | Apache-2.0 OR MIT | https://crates.io/crates/chacha20poly1305 |
| chrono | 0.4.44 | MIT OR Apache-2.0 | https://crates.io/crates/chrono |
| cipher | 0.4.4 | MIT OR Apache-2.0 | https://crates.io/crates/cipher |
| clap | 4.6.1 | MIT OR Apache-2.0 | https://crates.io/crates/clap |
| clap_builder | 4.6.0 | MIT OR Apache-2.0 | https://crates.io/crates/clap_builder |
| clap_derive | 4.6.1 | MIT OR Apache-2.0 | https://crates.io/crates/clap_derive |
| clap_lex | 1.1.0 | MIT OR Apache-2.0 | https://crates.io/crates/clap_lex |
| cmake | 0.1.58 | MIT OR Apache-2.0 | https://crates.io/crates/cmake |
| cmov | 0.5.3 | Apache-2.0 OR MIT | https://crates.io/crates/cmov |
| cobs | 0.3.0 | MIT OR Apache-2.0 | https://crates.io/crates/cobs |
| color_quant | 1.1.0 | MIT | https://crates.io/crates/color_quant |
| colorchoice | 1.0.5 | MIT OR Apache-2.0 | https://crates.io/crates/colorchoice |
| combine | 4.6.7 | MIT | https://crates.io/crates/combine |
| concurrent-queue | 2.5.0 | Apache-2.0 OR MIT | https://crates.io/crates/concurrent-queue |
| constant_time_eq | 0.4.2 | CC0-1.0 OR MIT-0 OR Apache-2.0 | https://crates.io/crates/constant_time_eq |
| const-oid | 0.10.2 | Apache-2.0 OR MIT | https://crates.io/crates/const-oid |
| const-oid | 0.9.6 | Apache-2.0 OR MIT | https://crates.io/crates/const-oid |
| convert_case | 0.10.0 | MIT | https://crates.io/crates/convert_case |
| cordyceps | 0.3.4 | MIT | https://crates.io/crates/cordyceps |
| core-foundation | 0.10.1 | MIT OR Apache-2.0 | https://crates.io/crates/core-foundation |
| core-foundation | 0.9.4 | MIT OR Apache-2.0 | https://crates.io/crates/core-foundation |
| core-foundation-sys | 0.8.7 | MIT OR Apache-2.0 | https://crates.io/crates/core-foundation-sys |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 | https://crates.io/crates/cpufeatures |
| cpufeatures | 0.3.0 | MIT OR Apache-2.0 | https://crates.io/crates/cpufeatures |
| crc | 3.4.0 | MIT OR Apache-2.0 | https://crates.io/crates/crc |
| crc32fast | 1.5.0 | MIT OR Apache-2.0 | https://crates.io/crates/crc32fast |
| crc-catalog | 2.5.0 | MIT OR Apache-2.0 | https://crates.io/crates/crc-catalog |
| critical-section | 1.2.0 | MIT OR Apache-2.0 | https://crates.io/crates/critical-section |
| crossbeam-channel | 0.5.15 | MIT OR Apache-2.0 | https://crates.io/crates/crossbeam-channel |
| crossbeam-epoch | 0.9.18 | MIT OR Apache-2.0 | https://crates.io/crates/crossbeam-epoch |
| crossbeam-queue | 0.3.12 | MIT OR Apache-2.0 | https://crates.io/crates/crossbeam-queue |
| crossbeam-utils | 0.8.21 | MIT OR Apache-2.0 | https://crates.io/crates/crossbeam-utils |
| crypto-common | 0.1.7 | MIT OR Apache-2.0 | https://crates.io/crates/crypto-common |
| crypto-common | 0.2.2 | MIT OR Apache-2.0 | https://crates.io/crates/crypto-common |
| ctr | 0.9.2 | MIT OR Apache-2.0 | https://crates.io/crates/ctr |
| ctutils | 0.4.2 | Apache-2.0 OR MIT | https://crates.io/crates/ctutils |
| curve25519-dalek | 5.0.0-pre.6 | BSD-3-Clause | https://crates.io/crates/curve25519-dalek |
| curve25519-dalek-derive | 0.1.1 | MIT/Apache-2.0 | https://crates.io/crates/curve25519-dalek-derive |
| darling | 0.20.11 | MIT | https://crates.io/crates/darling |
| darling_core | 0.20.11 | MIT | https://crates.io/crates/darling_core |
| darling_macro | 0.20.11 | MIT | https://crates.io/crates/darling_macro |
| dashmap | 6.2.1 | MIT | https://crates.io/crates/dashmap |
| data-encoding | 2.11.0 | MIT | https://crates.io/crates/data-encoding |
| data-encoding-macro | 0.1.20 | MIT | https://crates.io/crates/data-encoding-macro |
| data-encoding-macro-internal | 0.1.18 | MIT | https://crates.io/crates/data-encoding-macro-internal |
| dbus | 0.9.11 | Apache-2.0/MIT | https://crates.io/crates/dbus |
| dbus-secret-service | 4.1.0 | MIT OR Apache-2.0 | https://crates.io/crates/dbus-secret-service |
| der | 0.7.10 | Apache-2.0 OR MIT | https://crates.io/crates/der |
| der | 0.8.0 | Apache-2.0 OR MIT | https://crates.io/crates/der |
| deranged | 0.5.8 | MIT OR Apache-2.0 | https://crates.io/crates/deranged |
| derive_builder | 0.20.2 | MIT OR Apache-2.0 | https://crates.io/crates/derive_builder |
| derive_builder_core | 0.20.2 | MIT OR Apache-2.0 | https://crates.io/crates/derive_builder_core |
| derive_builder_macro | 0.20.2 | MIT OR Apache-2.0 | https://crates.io/crates/derive_builder_macro |
| derive_more | 2.1.1 | MIT | https://crates.io/crates/derive_more |
| derive_more-impl | 2.1.1 | MIT | https://crates.io/crates/derive_more-impl |
| der-parser | 10.0.0 | MIT OR Apache-2.0 | https://crates.io/crates/der-parser |
| diatomic-waker | 0.2.3 | MIT OR Apache-2.0 | https://crates.io/crates/diatomic-waker |
| digest | 0.10.7 | MIT OR Apache-2.0 | https://crates.io/crates/digest |
| digest | 0.11.3 | MIT OR Apache-2.0 | https://crates.io/crates/digest |
| dispatch2 | 0.3.1 | Zlib OR Apache-2.0 OR MIT | https://crates.io/crates/dispatch2 |
| displaydoc | 0.2.6 | MIT OR Apache-2.0 | https://crates.io/crates/displaydoc |
| dlopen2 | 0.8.2 | MIT | https://crates.io/crates/dlopen2 |
| document-features | 0.2.12 | MIT OR Apache-2.0 | https://crates.io/crates/document-features |
| dotenvy | 0.15.7 | MIT | https://crates.io/crates/dotenvy |
| dunce | 1.0.5 | CC0-1.0 OR MIT-0 OR Apache-2.0 | https://crates.io/crates/dunce |
| dyn-clone | 1.0.20 | MIT OR Apache-2.0 | https://crates.io/crates/dyn-clone |
| ed25519 | 3.0.0 | Apache-2.0 OR MIT | https://crates.io/crates/ed25519 |
| ed25519-dalek | 3.0.0-pre.7 | BSD-3-Clause | https://crates.io/crates/ed25519-dalek |
| either | 1.16.0 | MIT OR Apache-2.0 | https://crates.io/crates/either |
| embedded-io | 0.4.0 | MIT OR Apache-2.0 | https://crates.io/crates/embedded-io |
| embedded-io | 0.6.1 | MIT OR Apache-2.0 | https://crates.io/crates/embedded-io |
| enum-assoc | 1.3.0 | MIT OR Apache-2.0 | https://crates.io/crates/enum-assoc |
| equivalent | 1.0.2 | Apache-2.0 OR MIT | https://crates.io/crates/equivalent |
| errno | 0.3.14 | MIT OR Apache-2.0 | https://crates.io/crates/errno |
| etcetera | 0.8.0 | MIT OR Apache-2.0 | https://crates.io/crates/etcetera |
| event-listener | 5.4.1 | Apache-2.0 OR MIT | https://crates.io/crates/event-listener |
| event-listener-strategy | 0.5.4 | Apache-2.0 OR MIT | https://crates.io/crates/event-listener-strategy |
| fastbloom | 0.17.0 | MIT OR Apache-2.0 | https://crates.io/crates/fastbloom |
| fastrand | 2.4.1 | Apache-2.0 OR MIT | https://crates.io/crates/fastrand |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 | https://crates.io/crates/fdeflate |
| fiat-crypto | 0.3.0 | MIT OR Apache-2.0 OR BSD-1-Clause | https://crates.io/crates/fiat-crypto |
| find-msvc-tools | 0.1.9 | MIT OR Apache-2.0 | https://crates.io/crates/find-msvc-tools |
| fixedbitset | 0.5.7 | MIT OR Apache-2.0 | https://crates.io/crates/fixedbitset |
| flate2 | 1.1.9 | MIT OR Apache-2.0 | https://crates.io/crates/flate2 |
| flume | 0.11.1 | Apache-2.0/MIT | https://crates.io/crates/flume |
| flume | 0.12.0 | Apache-2.0/MIT | https://crates.io/crates/flume |
| fnv | 1.0.7 | Apache-2.0 / MIT | https://crates.io/crates/fnv |
| foldhash | 0.1.5 | Zlib | https://crates.io/crates/foldhash |
| foldhash | 0.2.0 | Zlib | https://crates.io/crates/foldhash |
| form_urlencoded | 1.2.2 | MIT OR Apache-2.0 | https://crates.io/crates/form_urlencoded |
| fs_extra | 1.3.0 | MIT | https://crates.io/crates/fs_extra |
| futures | 0.3.32 | MIT OR Apache-2.0 | https://crates.io/crates/futures |
| futures-buffered | 0.2.13 | MIT | https://crates.io/crates/futures-buffered |
| futures-channel | 0.3.32 | MIT OR Apache-2.0 | https://crates.io/crates/futures-channel |
| futures-concurrency | 7.7.1 | MIT OR Apache-2.0 | https://crates.io/crates/futures-concurrency |
| futures-core | 0.3.32 | MIT OR Apache-2.0 | https://crates.io/crates/futures-core |
| futures-executor | 0.3.32 | MIT OR Apache-2.0 | https://crates.io/crates/futures-executor |
| futures-intrusive | 0.5.0 | MIT OR Apache-2.0 | https://crates.io/crates/futures-intrusive |
| futures-io | 0.3.32 | MIT OR Apache-2.0 | https://crates.io/crates/futures-io |
| futures-lite | 2.6.1 | Apache-2.0 OR MIT | https://crates.io/crates/futures-lite |
| futures-macro | 0.3.32 | MIT OR Apache-2.0 | https://crates.io/crates/futures-macro |
| futures-sink | 0.3.32 | MIT OR Apache-2.0 | https://crates.io/crates/futures-sink |
| futures-task | 0.3.32 | MIT OR Apache-2.0 | https://crates.io/crates/futures-task |
| futures-util | 0.3.32 | MIT OR Apache-2.0 | https://crates.io/crates/futures-util |
| genawaiter | 0.99.1 | MIT | https://crates.io/crates/genawaiter |
| genawaiter-macro | 0.99.1 | MIT/Apache-2.0 | https://crates.io/crates/genawaiter-macro |
| genawaiter-proc-macro | 0.99.1 | MIT/Apache-2.0 | https://crates.io/crates/genawaiter-proc-macro |
| generator | 0.8.8 | MIT/Apache-2.0 | https://crates.io/crates/generator |
| generic-array | 0.14.7 | MIT | https://crates.io/crates/generic-array |
| getrandom | 0.2.17 | MIT OR Apache-2.0 | https://crates.io/crates/getrandom |
| getrandom | 0.3.4 | MIT OR Apache-2.0 | https://crates.io/crates/getrandom |
| getrandom | 0.4.2 | MIT OR Apache-2.0 | https://crates.io/crates/getrandom |
| ghash | 0.5.1 | Apache-2.0 OR MIT | https://crates.io/crates/ghash |
| gif | 0.14.2 | MIT OR Apache-2.0 | https://crates.io/crates/gif |
| gloo-timers | 0.3.0 | MIT OR Apache-2.0 | https://crates.io/crates/gloo-timers |
| h2 | 0.4.14 | MIT | https://crates.io/crates/h2 |
| hash32 | 0.2.1 | MIT OR Apache-2.0 | https://crates.io/crates/hash32 |
| hashbrown | 0.14.5 | MIT OR Apache-2.0 | https://crates.io/crates/hashbrown |
| hashbrown | 0.15.5 | MIT OR Apache-2.0 | https://crates.io/crates/hashbrown |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 | https://crates.io/crates/hashbrown |
| hashlink | 0.10.0 | MIT OR Apache-2.0 | https://crates.io/crates/hashlink |
| heapless | 0.7.17 | MIT OR Apache-2.0 | https://crates.io/crates/heapless |
| heck | 0.5.0 | MIT OR Apache-2.0 | https://crates.io/crates/heck |
| hex | 0.4.3 | MIT OR Apache-2.0 | https://crates.io/crates/hex |
| hex-conservative | 0.2.2 | CC0-1.0 | https://crates.io/crates/hex-conservative |
| hickory-net | 0.26.1 | MIT OR Apache-2.0 | https://crates.io/crates/hickory-net |
| hickory-proto | 0.26.1 | MIT OR Apache-2.0 | https://crates.io/crates/hickory-proto |
| hickory-resolver | 0.26.1 | MIT OR Apache-2.0 | https://crates.io/crates/hickory-resolver |
| hkdf | 0.12.4 | MIT OR Apache-2.0 | https://crates.io/crates/hkdf |
| hmac | 0.12.1 | MIT OR Apache-2.0 | https://crates.io/crates/hmac |
| home | 0.5.12 | MIT OR Apache-2.0 | https://crates.io/crates/home |
| http | 1.4.1 | MIT OR Apache-2.0 | https://crates.io/crates/http |
| httparse | 1.10.1 | MIT OR Apache-2.0 | https://crates.io/crates/httparse |
| http-body | 1.0.1 | MIT | https://crates.io/crates/http-body |
| http-body-util | 0.1.3 | MIT | https://crates.io/crates/http-body-util |
| httpdate | 1.0.3 | MIT OR Apache-2.0 | https://crates.io/crates/httpdate |
| hybrid-array | 0.4.12 | MIT OR Apache-2.0 | https://crates.io/crates/hybrid-array |
| hyper | 1.10.0 | MIT | https://crates.io/crates/hyper |
| hyper-rustls | 0.27.9 | Apache-2.0 OR ISC OR MIT | https://crates.io/crates/hyper-rustls |
| hyper-util | 0.1.20 | MIT | https://crates.io/crates/hyper-util |
| iana-time-zone | 0.1.65 | MIT OR Apache-2.0 | https://crates.io/crates/iana-time-zone |
| iana-time-zone-haiku | 0.1.2 | MIT OR Apache-2.0 | https://crates.io/crates/iana-time-zone-haiku |
| icu_collections | 2.2.0 | Unicode-3.0 | https://crates.io/crates/icu_collections |
| icu_locale_core | 2.2.0 | Unicode-3.0 | https://crates.io/crates/icu_locale_core |
| icu_normalizer | 2.2.0 | Unicode-3.0 | https://crates.io/crates/icu_normalizer |
| icu_normalizer_data | 2.2.0 | Unicode-3.0 | https://crates.io/crates/icu_normalizer_data |
| icu_properties | 2.2.0 | Unicode-3.0 | https://crates.io/crates/icu_properties |
| icu_properties_data | 2.2.0 | Unicode-3.0 | https://crates.io/crates/icu_properties_data |
| icu_provider | 2.2.0 | Unicode-3.0 | https://crates.io/crates/icu_provider |
| id-arena | 2.3.0 | MIT/Apache-2.0 | https://crates.io/crates/id-arena |
| ident_case | 1.0.1 | MIT/Apache-2.0 | https://crates.io/crates/ident_case |
| identity-hash | 0.1.0 | Apache-2.0 OR MIT | https://crates.io/crates/identity-hash |
| idna | 1.1.0 | MIT OR Apache-2.0 | https://crates.io/crates/idna |
| idna_adapter | 1.2.2 | Apache-2.0 OR MIT | https://crates.io/crates/idna_adapter |
| igd-next | 0.17.0 | MIT | https://crates.io/crates/igd-next |
| image | 0.25.10 | MIT OR Apache-2.0 | https://crates.io/crates/image |
| image-webp | 0.2.4 | MIT OR Apache-2.0 | https://crates.io/crates/image-webp |
| indexmap | 2.14.0 | Apache-2.0 OR MIT | https://crates.io/crates/indexmap |
| inout | 0.1.4 | MIT OR Apache-2.0 | https://crates.io/crates/inout |
| inplace-vec-builder | 0.1.1 | MIT OR Apache-2.0 | https://crates.io/crates/inplace-vec-builder |
| ipconfig | 0.3.4 | MIT/Apache-2.0 | https://crates.io/crates/ipconfig |
| ipnet | 2.12.0 | MIT OR Apache-2.0 | https://crates.io/crates/ipnet |
| iroh | 1.0.0-rc.1 | MIT OR Apache-2.0 | https://crates.io/crates/iroh |
| iroh-base | 1.0.0-rc.1 | MIT OR Apache-2.0 | https://crates.io/crates/iroh-base |
| iroh-blobs | 0.102.0 | MIT OR Apache-2.0 | https://crates.io/crates/iroh-blobs |
| iroh-dns | 1.0.0-rc.1 | MIT OR Apache-2.0 | https://crates.io/crates/iroh-dns |
| iroh-docs | 0.100.0 | MIT/Apache-2.0 | https://crates.io/crates/iroh-docs |
| iroh-gossip | 0.100.0 | MIT/Apache-2.0 | https://crates.io/crates/iroh-gossip |
| iroh-io | 0.6.2 | Apache-2.0 OR MIT | https://crates.io/crates/iroh-io |
| iroh-mainline-address-lookup | 0.3.0 | MIT OR Apache-2.0 | https://crates.io/crates/iroh-mainline-address-lookup |
| iroh-metrics | 1.0.0-rc.0 | MIT OR Apache-2.0 | https://crates.io/crates/iroh-metrics |
| iroh-metrics-derive | 1.0.0-rc.0 | MIT OR Apache-2.0 | https://crates.io/crates/iroh-metrics-derive |
| iroh-relay | 1.0.0-rc.1 | MIT OR Apache-2.0 | https://crates.io/crates/iroh-relay |
| iroh-tickets | 1.0.0-rc.1 | MIT OR Apache-2.0 | https://crates.io/crates/iroh-tickets |
| iroh-util | 0.5.0 | MIT OR Apache-2.0 | https://crates.io/crates/iroh-util |
| irpc | 0.16.0 | Apache-2.0/MIT | https://crates.io/crates/irpc |
| irpc-derive | 0.16.0 | Apache-2.0/MIT | https://crates.io/crates/irpc-derive |
| is_terminal_polyfill | 1.70.2 | MIT OR Apache-2.0 | https://crates.io/crates/is_terminal_polyfill |
| itoa | 1.0.18 | MIT OR Apache-2.0 | https://crates.io/crates/itoa |
| jni | 0.22.4 | MIT OR Apache-2.0 | https://crates.io/crates/jni |
| jni-macros | 0.22.4 | MIT OR Apache-2.0 | https://crates.io/crates/jni-macros |
| jni-sys | 0.4.1 | MIT OR Apache-2.0 | https://crates.io/crates/jni-sys |
| jni-sys-macros | 0.4.1 | MIT OR Apache-2.0 | https://crates.io/crates/jni-sys-macros |
| jobserver | 0.1.34 | MIT OR Apache-2.0 | https://crates.io/crates/jobserver |
| jsonwebtoken | 10.4.0 | MIT | https://crates.io/crates/jsonwebtoken |
| js-sys | 0.3.99 | MIT OR Apache-2.0 | https://crates.io/crates/js-sys |
| keyring | 3.6.3 | MIT OR Apache-2.0 | https://crates.io/crates/keyring |
| lazy_static | 1.5.0 | MIT OR Apache-2.0 | https://crates.io/crates/lazy_static |
| leb128fmt | 0.1.0 | MIT OR Apache-2.0 | https://crates.io/crates/leb128fmt |
| libc | 0.2.186 | MIT OR Apache-2.0 | https://crates.io/crates/libc |
| libdbus-sys | 0.2.7 | Apache-2.0/MIT | https://crates.io/crates/libdbus-sys |
| libm | 0.2.16 | MIT | https://crates.io/crates/libm |
| libredox | 0.1.17 | MIT | https://crates.io/crates/libredox |
| libsqlite3-sys | 0.30.1 | MIT | https://crates.io/crates/libsqlite3-sys |
| linux-raw-sys | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://crates.io/crates/linux-raw-sys |
| litemap | 0.8.2 | Unicode-3.0 | https://crates.io/crates/litemap |
| litrs | 1.0.0 | MIT OR Apache-2.0 | https://crates.io/crates/litrs |
| lock_api | 0.4.14 | MIT OR Apache-2.0 | https://crates.io/crates/lock_api |
| log | 0.4.30 | MIT OR Apache-2.0 | https://crates.io/crates/log |
| loom | 0.7.2 | MIT | https://crates.io/crates/loom |
| lru | 0.16.4 | MIT | https://crates.io/crates/lru |
| lru | 0.18.0 | MIT | https://crates.io/crates/lru |
| lru-slab | 0.1.2 | MIT OR Apache-2.0 OR Zlib | https://crates.io/crates/lru-slab |
| mac-addr | 0.3.0 | MIT | https://crates.io/crates/mac-addr |
| mainline | 6.2.0 | MIT | https://crates.io/crates/mainline |
| matchers | 0.2.0 | MIT | https://crates.io/crates/matchers |
| matchit | 0.8.4 | MIT AND BSD-3-Clause | https://crates.io/crates/matchit |
| md-5 | 0.10.6 | MIT OR Apache-2.0 | https://crates.io/crates/md-5 |
| memchr | 2.8.1 | Unlicense OR MIT | https://crates.io/crates/memchr |
| mime | 0.3.17 | MIT OR Apache-2.0 | https://crates.io/crates/mime |
| minimal-lexical | 0.2.1 | MIT/Apache-2.0 | https://crates.io/crates/minimal-lexical |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 | https://crates.io/crates/miniz_oxide |
| mio | 1.2.0 | MIT | https://crates.io/crates/mio |
| moka | 0.12.15 | (MIT OR Apache-2.0) AND Apache-2.0 | https://crates.io/crates/moka |
| moxcms | 0.8.1 | BSD-3-Clause OR Apache-2.0 | https://crates.io/crates/moxcms |
| n0-error | 1.0.0-rc.0 | MIT OR Apache-2.0 | https://crates.io/crates/n0-error |
| n0-error-macros | 1.0.0-rc.0 | MIT OR Apache-2.0 | https://crates.io/crates/n0-error-macros |
| n0-future | 0.3.2 | MIT OR Apache-2.0 | https://crates.io/crates/n0-future |
| n0-mainline | 0.4.0 | MIT OR Apache-2.0 | https://crates.io/crates/n0-mainline |
| n0-watcher | 1.0.0-rc.0 | MIT OR Apache-2.0 | https://crates.io/crates/n0-watcher |
| ndk-context | 0.1.1 | MIT OR Apache-2.0 | https://crates.io/crates/ndk-context |
| nested_enum_utils | 0.2.3 | MIT OR Apache-2.0 | https://crates.io/crates/nested_enum_utils |
| netdev | 0.43.0 | MIT | https://crates.io/crates/netdev |
| netlink-packet-core | 0.8.1 | MIT | https://crates.io/crates/netlink-packet-core |
| netlink-packet-route | 0.29.0 | MIT | https://crates.io/crates/netlink-packet-route |
| netlink-packet-route | 0.30.0 | MIT | https://crates.io/crates/netlink-packet-route |
| netlink-proto | 0.12.0 | MIT | https://crates.io/crates/netlink-proto |
| netlink-sys | 0.8.8 | MIT | https://crates.io/crates/netlink-sys |
| netwatch | 0.18.0 | MIT OR Apache-2.0 | https://crates.io/crates/netwatch |
| nom | 7.1.3 | MIT | https://crates.io/crates/nom |
| noq | 1.0.0-rc.1 | MIT OR Apache-2.0 | https://crates.io/crates/noq |
| noq-proto | 1.0.0-rc.1 | MIT OR Apache-2.0 | https://crates.io/crates/noq-proto |
| noq-udp | 1.0.0-rc.1 | MIT OR Apache-2.0 | https://crates.io/crates/noq-udp |
| ntimestamp | 1.0.0 | MIT | https://crates.io/crates/ntimestamp |
| nu-ansi-term | 0.50.3 | MIT | https://crates.io/crates/nu-ansi-term |
| num_enum | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 | https://crates.io/crates/num_enum |
| num_enum_derive | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 | https://crates.io/crates/num_enum_derive |
| num_threads | 0.1.7 | MIT OR Apache-2.0 | https://crates.io/crates/num_threads |
| num-bigint | 0.4.6 | MIT OR Apache-2.0 | https://crates.io/crates/num-bigint |
| num-bigint-dig | 0.8.6 | MIT/Apache-2.0 | https://crates.io/crates/num-bigint-dig |
| num-conv | 0.2.2 | MIT OR Apache-2.0 | https://crates.io/crates/num-conv |
| num-integer | 0.1.46 | MIT OR Apache-2.0 | https://crates.io/crates/num-integer |
| num-iter | 0.1.45 | MIT OR Apache-2.0 | https://crates.io/crates/num-iter |
| num-traits | 0.2.19 | MIT OR Apache-2.0 | https://crates.io/crates/num-traits |
| objc2 | 0.6.4 | MIT | https://crates.io/crates/objc2 |
| objc2-core-foundation | 0.3.2 | Zlib OR Apache-2.0 OR MIT | https://crates.io/crates/objc2-core-foundation |
| objc2-core-wlan | 0.3.2 | Zlib OR Apache-2.0 OR MIT | https://crates.io/crates/objc2-core-wlan |
| objc2-encode | 4.1.0 | MIT | https://crates.io/crates/objc2-encode |
| objc2-foundation | 0.3.2 | MIT | https://crates.io/crates/objc2-foundation |
| objc2-security | 0.3.2 | Zlib OR Apache-2.0 OR MIT | https://crates.io/crates/objc2-security |
| objc2-security-foundation | 0.3.2 | Zlib OR Apache-2.0 OR MIT | https://crates.io/crates/objc2-security-foundation |
| objc2-system-configuration | 0.3.2 | Zlib OR Apache-2.0 OR MIT | https://crates.io/crates/objc2-system-configuration |
| oid-registry | 0.8.1 | MIT OR Apache-2.0 | https://crates.io/crates/oid-registry |
| once_cell | 1.21.4 | MIT OR Apache-2.0 | https://crates.io/crates/once_cell |
| once_cell_polyfill | 1.70.2 | MIT OR Apache-2.0 | https://crates.io/crates/once_cell_polyfill |
| opaque-debug | 0.3.1 | MIT OR Apache-2.0 | https://crates.io/crates/opaque-debug |
| openssl-probe | 0.2.1 | MIT OR Apache-2.0 | https://crates.io/crates/openssl-probe |
| papaya | 0.2.4 | MIT | https://crates.io/crates/papaya |
| parking | 2.2.1 | Apache-2.0 OR MIT | https://crates.io/crates/parking |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 | https://crates.io/crates/parking_lot |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 | https://crates.io/crates/parking_lot_core |
| paste | 1.0.15 | MIT OR Apache-2.0 | https://crates.io/crates/paste |
| pem | 3.0.6 | MIT | https://crates.io/crates/pem |
| pem-rfc7468 | 0.7.0 | Apache-2.0 OR MIT | https://crates.io/crates/pem-rfc7468 |
| pem-rfc7468 | 1.0.0 | Apache-2.0 OR MIT | https://crates.io/crates/pem-rfc7468 |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 | https://crates.io/crates/percent-encoding |
| pharos | 0.5.3 | Unlicense | https://crates.io/crates/pharos |
| pin-project | 1.1.13 | Apache-2.0 OR MIT | https://crates.io/crates/pin-project |
| pin-project-internal | 1.1.13 | Apache-2.0 OR MIT | https://crates.io/crates/pin-project-internal |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT | https://crates.io/crates/pin-project-lite |
| pkarr | 5.0.2 | MIT | https://crates.io/crates/pkarr |
| pkcs1 | 0.7.5 | Apache-2.0 OR MIT | https://crates.io/crates/pkcs1 |
| pkcs8 | 0.10.2 | Apache-2.0 OR MIT | https://crates.io/crates/pkcs8 |
| pkcs8 | 0.11.0 | Apache-2.0 OR MIT | https://crates.io/crates/pkcs8 |
| pkg-config | 0.3.33 | MIT OR Apache-2.0 | https://crates.io/crates/pkg-config |
| plain | 0.2.3 | MIT/Apache-2.0 | https://crates.io/crates/plain |
| plist | 1.9.0 | MIT | https://crates.io/crates/plist |
| png | 0.18.1 | MIT OR Apache-2.0 | https://crates.io/crates/png |
| poly1305 | 0.8.0 | Apache-2.0 OR MIT | https://crates.io/crates/poly1305 |
| polyval | 0.6.2 | Apache-2.0 OR MIT | https://crates.io/crates/polyval |
| portable-atomic | 1.13.1 | Apache-2.0 OR MIT | https://crates.io/crates/portable-atomic |
| portmapper | 0.18.0 | MIT OR Apache-2.0 | https://crates.io/crates/portmapper |
| positioned-io | 0.3.5 | MIT | https://crates.io/crates/positioned-io |
| postcard | 1.1.3 | MIT OR Apache-2.0 | https://crates.io/crates/postcard |
| postcard-derive | 0.2.2 | MIT OR Apache-2.0 | https://crates.io/crates/postcard-derive |
| potential_utf | 0.1.5 | Unicode-3.0 | https://crates.io/crates/potential_utf |
| powerfmt | 0.2.0 | MIT OR Apache-2.0 | https://crates.io/crates/powerfmt |
| ppv-lite86 | 0.2.21 | MIT OR Apache-2.0 | https://crates.io/crates/ppv-lite86 |
| prefix-trie | 0.8.4 | MIT OR Apache-2.0 | https://crates.io/crates/prefix-trie |
| prettyplease | 0.2.37 | MIT OR Apache-2.0 | https://crates.io/crates/prettyplease |
| proc-macro2 | 1.0.106 | MIT OR Apache-2.0 | https://crates.io/crates/proc-macro2 |
| proc-macro-crate | 3.5.0 | MIT OR Apache-2.0 | https://crates.io/crates/proc-macro-crate |
| proc-macro-error | 0.4.12 | MIT OR Apache-2.0 | https://crates.io/crates/proc-macro-error |
| proc-macro-error-attr | 0.4.12 | MIT OR Apache-2.0 | https://crates.io/crates/proc-macro-error-attr |
| proc-macro-hack | 0.5.20+deprecated | MIT OR Apache-2.0 | https://crates.io/crates/proc-macro-hack |
| pxfm | 0.1.29 | BSD-3-Clause OR Apache-2.0 | https://crates.io/crates/pxfm |
| quick-error | 2.0.1 | MIT/Apache-2.0 | https://crates.io/crates/quick-error |
| quick-xml | 0.39.4 | MIT | https://crates.io/crates/quick-xml |
| quinn | 0.11.9 | MIT OR Apache-2.0 | https://crates.io/crates/quinn |
| quinn-proto | 0.11.14 | MIT OR Apache-2.0 | https://crates.io/crates/quinn-proto |
| quinn-udp | 0.5.14 | MIT OR Apache-2.0 | https://crates.io/crates/quinn-udp |
| quote | 1.0.45 | MIT OR Apache-2.0 | https://crates.io/crates/quote |
| rand | 0.10.1 | MIT OR Apache-2.0 | https://crates.io/crates/rand |
| rand | 0.8.6 | MIT OR Apache-2.0 | https://crates.io/crates/rand |
| rand | 0.9.4 | MIT OR Apache-2.0 | https://crates.io/crates/rand |
| rand_chacha | 0.3.1 | MIT OR Apache-2.0 | https://crates.io/crates/rand_chacha |
| rand_chacha | 0.9.0 | MIT OR Apache-2.0 | https://crates.io/crates/rand_chacha |
| rand_core | 0.10.1 | MIT OR Apache-2.0 | https://crates.io/crates/rand_core |
| rand_core | 0.6.4 | MIT OR Apache-2.0 | https://crates.io/crates/rand_core |
| rand_core | 0.9.5 | MIT OR Apache-2.0 | https://crates.io/crates/rand_core |
| rand_pcg | 0.10.2 | MIT OR Apache-2.0 | https://crates.io/crates/rand_pcg |
| range-collections | 0.4.6 | MIT OR Apache-2.0 | https://crates.io/crates/range-collections |
| rcgen | 0.14.8 | MIT OR Apache-2.0 | https://crates.io/crates/rcgen |
| redb | 4.1.0 | MIT OR Apache-2.0 | https://crates.io/crates/redb |
| redis | 1.2.2 | BSD-3-Clause | https://crates.io/crates/redis |
| redox_syscall | 0.5.18 | MIT | https://crates.io/crates/redox_syscall |
| redox_syscall | 0.8.0 | MIT | https://crates.io/crates/redox_syscall |
| ref-cast | 1.0.25 | MIT OR Apache-2.0 | https://crates.io/crates/ref-cast |
| ref-cast-impl | 1.0.25 | MIT OR Apache-2.0 | https://crates.io/crates/ref-cast-impl |
| r-efi | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | https://crates.io/crates/r-efi |
| r-efi | 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | https://crates.io/crates/r-efi |
| reflink-copy | 0.1.29 | MIT/Apache-2.0 | https://crates.io/crates/reflink-copy |
| regex-automata | 0.4.14 | MIT OR Apache-2.0 | https://crates.io/crates/regex-automata |
| regex-syntax | 0.8.10 | MIT OR Apache-2.0 | https://crates.io/crates/regex-syntax |
| reloadable-core | 0.1.0 | MIT | https://crates.io/crates/reloadable-core |
| reloadable-state | 0.1.0 | MIT | https://crates.io/crates/reloadable-state |
| reqwest | 0.12.28 | MIT OR Apache-2.0 | https://crates.io/crates/reqwest |
| reqwest | 0.13.4 | MIT OR Apache-2.0 | https://crates.io/crates/reqwest |
| resolv-conf | 0.7.6 | MIT OR Apache-2.0 | https://crates.io/crates/resolv-conf |
| ring | 0.17.14 | Apache-2.0 AND ISC | https://crates.io/crates/ring |
| rsa | 0.9.10 | MIT OR Apache-2.0 | https://crates.io/crates/rsa |
| rustc_version | 0.4.1 | MIT OR Apache-2.0 | https://crates.io/crates/rustc_version |
| rustc-hash | 2.1.2 | Apache-2.0 OR MIT | https://crates.io/crates/rustc-hash |
| rusticata-macros | 4.1.0 | MIT/Apache-2.0 | https://crates.io/crates/rusticata-macros |
| rustix | 1.1.4 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://crates.io/crates/rustix |
| rustls | 0.23.40 | Apache-2.0 OR ISC OR MIT | https://crates.io/crates/rustls |
| rustls-cert-file-reader | 0.4.2 | MIT | https://crates.io/crates/rustls-cert-file-reader |
| rustls-cert-read | 0.3.0 | MIT | https://crates.io/crates/rustls-cert-read |
| rustls-cert-reloadable-resolver | 0.7.1 | MIT | https://crates.io/crates/rustls-cert-reloadable-resolver |
| rustls-native-certs | 0.8.3 | Apache-2.0 OR ISC OR MIT | https://crates.io/crates/rustls-native-certs |
| rustls-pki-types | 1.14.1 | MIT OR Apache-2.0 | https://crates.io/crates/rustls-pki-types |
| rustls-platform-verifier | 0.7.0 | MIT OR Apache-2.0 | https://crates.io/crates/rustls-platform-verifier |
| rustls-platform-verifier-android | 0.1.1 | MIT OR Apache-2.0 | https://crates.io/crates/rustls-platform-verifier-android |
| rustls-webpki | 0.103.13 | ISC | https://crates.io/crates/rustls-webpki |
| rustversion | 1.0.22 | MIT OR Apache-2.0 | https://crates.io/crates/rustversion |
| ryu | 1.0.23 | Apache-2.0 OR BSL-1.0 | https://crates.io/crates/ryu |
| same-file | 1.0.6 | Unlicense/MIT | https://crates.io/crates/same-file |
| schannel | 0.1.29 | MIT | https://crates.io/crates/schannel |
| scoped-tls | 1.0.1 | MIT/Apache-2.0 | https://crates.io/crates/scoped-tls |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 | https://crates.io/crates/scopeguard |
| secp256k1 | 0.31.1 | CC0-1.0 | https://crates.io/crates/secp256k1 |
| secp256k1-sys | 0.11.0 | CC0-1.0 | https://crates.io/crates/secp256k1-sys |
| security-framework | 3.7.0 | MIT OR Apache-2.0 | https://crates.io/crates/security-framework |
| security-framework-sys | 2.17.0 | MIT OR Apache-2.0 | https://crates.io/crates/security-framework-sys |
| seize | 0.5.1 | MIT | https://crates.io/crates/seize |
| self_cell | 1.2.2 | Apache-2.0 OR GPL-2.0-only | https://crates.io/crates/self_cell |
| semver | 1.0.28 | MIT OR Apache-2.0 | https://crates.io/crates/semver |
| send_wrapper | 0.6.0 | MIT/Apache-2.0 | https://crates.io/crates/send_wrapper |
| serde | 1.0.228 | MIT OR Apache-2.0 | https://crates.io/crates/serde |
| serde_bencode | 0.2.4 | MIT | https://crates.io/crates/serde_bencode |
| serde_bytes | 0.11.19 | MIT OR Apache-2.0 | https://crates.io/crates/serde_bytes |
| serde_core | 1.0.228 | MIT OR Apache-2.0 | https://crates.io/crates/serde_core |
| serde_derive | 1.0.228 | MIT OR Apache-2.0 | https://crates.io/crates/serde_derive |
| serde_json | 1.0.150 | MIT OR Apache-2.0 | https://crates.io/crates/serde_json |
| serde_path_to_error | 0.1.20 | MIT OR Apache-2.0 | https://crates.io/crates/serde_path_to_error |
| serde_spanned | 1.1.1 | MIT OR Apache-2.0 | https://crates.io/crates/serde_spanned |
| serde_urlencoded | 0.7.1 | MIT/Apache-2.0 | https://crates.io/crates/serde_urlencoded |
| serde_yaml | 0.9.34+deprecated | MIT OR Apache-2.0 | https://crates.io/crates/serde_yaml |
| serdect | 0.4.3 | Apache-2.0 OR MIT | https://crates.io/crates/serdect |
| serde-error | 0.1.3 | MIT | https://crates.io/crates/serde-error |
| sha1 | 0.10.6 | MIT OR Apache-2.0 | https://crates.io/crates/sha1 |
| sha1 | 0.11.0 | MIT OR Apache-2.0 | https://crates.io/crates/sha1 |
| sha1_smol | 1.0.1 | BSD-3-Clause | https://crates.io/crates/sha1_smol |
| sha2 | 0.10.9 | MIT OR Apache-2.0 | https://crates.io/crates/sha2 |
| sha2 | 0.11.0 | MIT OR Apache-2.0 | https://crates.io/crates/sha2 |
| sharded-slab | 0.1.7 | MIT | https://crates.io/crates/sharded-slab |
| shlex | 1.3.0 | MIT OR Apache-2.0 | https://crates.io/crates/shlex |
| signal-hook-registry | 1.4.8 | MIT OR Apache-2.0 | https://crates.io/crates/signal-hook-registry |
| signature | 2.2.0 | Apache-2.0 OR MIT | https://crates.io/crates/signature |
| signature | 3.0.0 | Apache-2.0 OR MIT | https://crates.io/crates/signature |
| simd_cesu8 | 1.1.1 | Apache-2.0 OR MIT | https://crates.io/crates/simd_cesu8 |
| simd-adler32 | 0.3.9 | MIT | https://crates.io/crates/simd-adler32 |
| simdutf8 | 0.1.5 | MIT OR Apache-2.0 | https://crates.io/crates/simdutf8 |
| simple_asn1 | 0.6.4 | ISC | https://crates.io/crates/simple_asn1 |
| simple-dns | 0.11.3 | MIT | https://crates.io/crates/simple-dns |
| simple-dns | 0.9.3 | MIT | https://crates.io/crates/simple-dns |
| siphasher | 1.0.3 | MIT/Apache-2.0 | https://crates.io/crates/siphasher |
| slab | 0.4.12 | MIT | https://crates.io/crates/slab |
| smallvec | 1.15.1 | MIT OR Apache-2.0 | https://crates.io/crates/smallvec |
| socket2 | 0.6.3 | MIT OR Apache-2.0 | https://crates.io/crates/socket2 |
| sorted-index-buffer | 0.2.1 | MIT OR Apache-2.0 | https://crates.io/crates/sorted-index-buffer |
| spez | 0.1.2 | BSD-2-Clause | https://crates.io/crates/spez |
| spin | 0.10.0 | MIT | https://crates.io/crates/spin |
| spin | 0.9.8 | MIT | https://crates.io/crates/spin |
| spki | 0.7.3 | Apache-2.0 OR MIT | https://crates.io/crates/spki |
| spki | 0.8.0 | Apache-2.0 OR MIT | https://crates.io/crates/spki |
| sqlx | 0.8.6 | MIT OR Apache-2.0 | https://crates.io/crates/sqlx |
| sqlx-core | 0.8.6 | MIT OR Apache-2.0 | https://crates.io/crates/sqlx-core |
| sqlx-macros | 0.8.6 | MIT OR Apache-2.0 | https://crates.io/crates/sqlx-macros |
| sqlx-macros-core | 0.8.6 | MIT OR Apache-2.0 | https://crates.io/crates/sqlx-macros-core |
| sqlx-mysql | 0.8.6 | MIT OR Apache-2.0 | https://crates.io/crates/sqlx-mysql |
| sqlx-postgres | 0.8.6 | MIT OR Apache-2.0 | https://crates.io/crates/sqlx-postgres |
| sqlx-sqlite | 0.8.6 | MIT OR Apache-2.0 | https://crates.io/crates/sqlx-sqlite |
| stable_deref_trait | 1.2.1 | MIT OR Apache-2.0 | https://crates.io/crates/stable_deref_trait |
| stringprep | 0.1.5 | MIT/Apache-2.0 | https://crates.io/crates/stringprep |
| strsim | 0.11.1 | MIT | https://crates.io/crates/strsim |
| strum | 0.28.0 | MIT | https://crates.io/crates/strum |
| strum_macros | 0.28.0 | MIT | https://crates.io/crates/strum_macros |
| subtle | 2.6.1 | BSD-3-Clause | https://crates.io/crates/subtle |
| syn | 1.0.109 | MIT OR Apache-2.0 | https://crates.io/crates/syn |
| syn | 2.0.117 | MIT OR Apache-2.0 | https://crates.io/crates/syn |
| sync_wrapper | 1.0.2 | Apache-2.0 | https://crates.io/crates/sync_wrapper |
| syn-mid | 0.5.4 | Apache-2.0 OR MIT | https://crates.io/crates/syn-mid |
| synstructure | 0.13.2 | MIT | https://crates.io/crates/synstructure |
| system-configuration | 0.7.0 | MIT OR Apache-2.0 | https://crates.io/crates/system-configuration |
| system-configuration-sys | 0.6.0 | MIT OR Apache-2.0 | https://crates.io/crates/system-configuration-sys |
| tagptr | 0.2.0 | MIT/Apache-2.0 | https://crates.io/crates/tagptr |
| tempfile | 3.27.0 | MIT OR Apache-2.0 | https://crates.io/crates/tempfile |
| thiserror | 2.0.18 | MIT OR Apache-2.0 | https://crates.io/crates/thiserror |
| thiserror-impl | 2.0.18 | MIT OR Apache-2.0 | https://crates.io/crates/thiserror-impl |
| thread_local | 1.1.9 | MIT OR Apache-2.0 | https://crates.io/crates/thread_local |
| time | 0.3.47 | MIT OR Apache-2.0 | https://crates.io/crates/time |
| time-core | 0.1.8 | MIT OR Apache-2.0 | https://crates.io/crates/time-core |
| time-macros | 0.2.27 | MIT OR Apache-2.0 | https://crates.io/crates/time-macros |
| tinystr | 0.8.3 | Unicode-3.0 | https://crates.io/crates/tinystr |
| tinyvec | 1.11.0 | Zlib OR Apache-2.0 OR MIT | https://crates.io/crates/tinyvec |
| tinyvec_macros | 0.1.1 | MIT OR Apache-2.0 OR Zlib | https://crates.io/crates/tinyvec_macros |
| tokio | 1.52.3 | MIT | https://crates.io/crates/tokio |
| tokio-macros | 2.7.0 | MIT | https://crates.io/crates/tokio-macros |
| tokio-rustls | 0.26.4 | MIT OR Apache-2.0 | https://crates.io/crates/tokio-rustls |
| tokio-rustls-acme | 0.9.1 | Apache-2.0 OR MIT | https://crates.io/crates/tokio-rustls-acme |
| tokio-stream | 0.1.18 | MIT | https://crates.io/crates/tokio-stream |
| tokio-tungstenite | 0.29.0 | MIT | https://crates.io/crates/tokio-tungstenite |
| tokio-util | 0.7.18 | MIT | https://crates.io/crates/tokio-util |
| tokio-websockets | 0.13.2 | MIT | https://crates.io/crates/tokio-websockets |
| toml | 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 | https://crates.io/crates/toml |
| toml_datetime | 1.1.1+spec-1.1.0 | MIT OR Apache-2.0 | https://crates.io/crates/toml_datetime |
| toml_edit | 0.25.12+spec-1.1.0 | MIT OR Apache-2.0 | https://crates.io/crates/toml_edit |
| toml_parser | 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 | https://crates.io/crates/toml_parser |
| toml_writer | 1.1.1+spec-1.1.0 | MIT OR Apache-2.0 | https://crates.io/crates/toml_writer |
| tower | 0.5.3 | MIT | https://crates.io/crates/tower |
| tower-http | 0.6.11 | MIT | https://crates.io/crates/tower-http |
| tower-layer | 0.3.3 | MIT | https://crates.io/crates/tower-layer |
| tower-service | 0.3.3 | MIT | https://crates.io/crates/tower-service |
| tracing | 0.1.44 | MIT | https://crates.io/crates/tracing |
| tracing-attributes | 0.1.31 | MIT | https://crates.io/crates/tracing-attributes |
| tracing-core | 0.1.36 | MIT | https://crates.io/crates/tracing-core |
| tracing-log | 0.2.0 | MIT | https://crates.io/crates/tracing-log |
| tracing-subscriber | 0.3.23 | MIT | https://crates.io/crates/tracing-subscriber |
| try-lock | 0.2.5 | MIT | https://crates.io/crates/try-lock |
| tungstenite | 0.29.0 | MIT OR Apache-2.0 | https://crates.io/crates/tungstenite |
| typenum | 1.20.0 | MIT OR Apache-2.0 | https://crates.io/crates/typenum |
| unicode-bidi | 0.3.18 | MIT OR Apache-2.0 | https://crates.io/crates/unicode-bidi |
| unicode-ident | 1.0.24 | (MIT OR Apache-2.0) AND Unicode-3.0 | https://crates.io/crates/unicode-ident |
| unicode-normalization | 0.1.25 | MIT OR Apache-2.0 | https://crates.io/crates/unicode-normalization |
| unicode-properties | 0.1.4 | MIT/Apache-2.0 | https://crates.io/crates/unicode-properties |
| unicode-segmentation | 1.13.2 | MIT OR Apache-2.0 | https://crates.io/crates/unicode-segmentation |
| unicode-xid | 0.2.6 | MIT OR Apache-2.0 | https://crates.io/crates/unicode-xid |
| universal-hash | 0.5.1 | MIT OR Apache-2.0 | https://crates.io/crates/universal-hash |
| unsafe-libyaml | 0.2.11 | MIT | https://crates.io/crates/unsafe-libyaml |
| untrusted | 0.7.1 | ISC | https://crates.io/crates/untrusted |
| untrusted | 0.9.0 | ISC | https://crates.io/crates/untrusted |
| url | 2.5.8 | MIT OR Apache-2.0 | https://crates.io/crates/url |
| utf8_iter | 1.0.4 | Apache-2.0 OR MIT | https://crates.io/crates/utf8_iter |
| utf8parse | 0.2.2 | Apache-2.0 OR MIT | https://crates.io/crates/utf8parse |
| uuid | 1.23.1 | Apache-2.0 OR MIT | https://crates.io/crates/uuid |
| valuable | 0.1.1 | MIT | https://crates.io/crates/valuable |
| vcpkg | 0.2.15 | MIT/Apache-2.0 | https://crates.io/crates/vcpkg |
| vergen | 9.1.0 | MIT OR Apache-2.0 | https://crates.io/crates/vergen |
| vergen-gitcl | 9.1.0 | MIT OR Apache-2.0 | https://crates.io/crates/vergen-gitcl |
| vergen-lib | 9.1.0 | MIT OR Apache-2.0 | https://crates.io/crates/vergen-lib |
| version_check | 0.9.5 | MIT/Apache-2.0 | https://crates.io/crates/version_check |
| walkdir | 2.5.0 | Unlicense/MIT | https://crates.io/crates/walkdir |
| want | 0.3.1 | MIT | https://crates.io/crates/want |
| wasi | 0.11.1+wasi-snapshot-preview1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://crates.io/crates/wasi |
| wasip2 | 1.0.3+wasi-0.2.9 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://crates.io/crates/wasip2 |
| wasip3 | 0.4.0+wasi-0.3.0-rc-2026-01-06 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://crates.io/crates/wasip3 |
| wasite | 0.1.0 | Apache-2.0 OR BSL-1.0 OR MIT | https://crates.io/crates/wasite |
| wasm-bindgen | 0.2.122 | MIT OR Apache-2.0 | https://crates.io/crates/wasm-bindgen |
| wasm-bindgen-futures | 0.4.72 | MIT OR Apache-2.0 | https://crates.io/crates/wasm-bindgen-futures |
| wasm-bindgen-macro | 0.2.122 | MIT OR Apache-2.0 | https://crates.io/crates/wasm-bindgen-macro |
| wasm-bindgen-macro-support | 0.2.122 | MIT OR Apache-2.0 | https://crates.io/crates/wasm-bindgen-macro-support |
| wasm-bindgen-shared | 0.2.122 | MIT OR Apache-2.0 | https://crates.io/crates/wasm-bindgen-shared |
| wasm-encoder | 0.244.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://crates.io/crates/wasm-encoder |
| wasm-metadata | 0.244.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://crates.io/crates/wasm-metadata |
| wasmparser | 0.244.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://crates.io/crates/wasmparser |
| wasm-streams | 0.5.0 | MIT OR Apache-2.0 | https://crates.io/crates/wasm-streams |
| webpki-root-certs | 1.0.7 | CDLA-Permissive-2.0 | https://crates.io/crates/webpki-root-certs |
| webpki-roots | 0.26.11 | CDLA-Permissive-2.0 | https://crates.io/crates/webpki-roots |
| webpki-roots | 1.0.7 | CDLA-Permissive-2.0 | https://crates.io/crates/webpki-roots |
| web-sys | 0.3.99 | MIT OR Apache-2.0 | https://crates.io/crates/web-sys |
| web-time | 1.1.0 | MIT OR Apache-2.0 | https://crates.io/crates/web-time |
| weezl | 0.1.12 | MIT OR Apache-2.0 | https://crates.io/crates/weezl |
| whoami | 1.6.1 | Apache-2.0 OR BSL-1.0 OR MIT | https://crates.io/crates/whoami |
| widestring | 1.2.1 | MIT OR Apache-2.0 | https://crates.io/crates/widestring |
| winapi | 0.3.9 | MIT/Apache-2.0 | https://crates.io/crates/winapi |
| winapi-i686-pc-windows-gnu | 0.4.0 | MIT/Apache-2.0 | https://crates.io/crates/winapi-i686-pc-windows-gnu |
| winapi-util | 0.1.11 | Unlicense OR MIT | https://crates.io/crates/winapi-util |
| winapi-x86_64-pc-windows-gnu | 0.4.0 | MIT/Apache-2.0 | https://crates.io/crates/winapi-x86_64-pc-windows-gnu |
| windows | 0.62.2 | MIT OR Apache-2.0 | https://crates.io/crates/windows |
| windows_aarch64_gnullvm | 0.48.5 | MIT OR Apache-2.0 | https://crates.io/crates/windows_aarch64_gnullvm |
| windows_aarch64_gnullvm | 0.52.6 | MIT OR Apache-2.0 | https://crates.io/crates/windows_aarch64_gnullvm |
| windows_aarch64_gnullvm | 0.53.1 | MIT OR Apache-2.0 | https://crates.io/crates/windows_aarch64_gnullvm |
| windows_aarch64_msvc | 0.48.5 | MIT OR Apache-2.0 | https://crates.io/crates/windows_aarch64_msvc |
| windows_aarch64_msvc | 0.52.6 | MIT OR Apache-2.0 | https://crates.io/crates/windows_aarch64_msvc |
| windows_aarch64_msvc | 0.53.1 | MIT OR Apache-2.0 | https://crates.io/crates/windows_aarch64_msvc |
| windows_i686_gnu | 0.48.5 | MIT OR Apache-2.0 | https://crates.io/crates/windows_i686_gnu |
| windows_i686_gnu | 0.52.6 | MIT OR Apache-2.0 | https://crates.io/crates/windows_i686_gnu |
| windows_i686_gnu | 0.53.1 | MIT OR Apache-2.0 | https://crates.io/crates/windows_i686_gnu |
| windows_i686_gnullvm | 0.52.6 | MIT OR Apache-2.0 | https://crates.io/crates/windows_i686_gnullvm |
| windows_i686_gnullvm | 0.53.1 | MIT OR Apache-2.0 | https://crates.io/crates/windows_i686_gnullvm |
| windows_i686_msvc | 0.48.5 | MIT OR Apache-2.0 | https://crates.io/crates/windows_i686_msvc |
| windows_i686_msvc | 0.52.6 | MIT OR Apache-2.0 | https://crates.io/crates/windows_i686_msvc |
| windows_i686_msvc | 0.53.1 | MIT OR Apache-2.0 | https://crates.io/crates/windows_i686_msvc |
| windows_x86_64_gnu | 0.48.5 | MIT OR Apache-2.0 | https://crates.io/crates/windows_x86_64_gnu |
| windows_x86_64_gnu | 0.52.6 | MIT OR Apache-2.0 | https://crates.io/crates/windows_x86_64_gnu |
| windows_x86_64_gnu | 0.53.1 | MIT OR Apache-2.0 | https://crates.io/crates/windows_x86_64_gnu |
| windows_x86_64_gnullvm | 0.48.5 | MIT OR Apache-2.0 | https://crates.io/crates/windows_x86_64_gnullvm |
| windows_x86_64_gnullvm | 0.52.6 | MIT OR Apache-2.0 | https://crates.io/crates/windows_x86_64_gnullvm |
| windows_x86_64_gnullvm | 0.53.1 | MIT OR Apache-2.0 | https://crates.io/crates/windows_x86_64_gnullvm |
| windows_x86_64_msvc | 0.48.5 | MIT OR Apache-2.0 | https://crates.io/crates/windows_x86_64_msvc |
| windows_x86_64_msvc | 0.52.6 | MIT OR Apache-2.0 | https://crates.io/crates/windows_x86_64_msvc |
| windows_x86_64_msvc | 0.53.1 | MIT OR Apache-2.0 | https://crates.io/crates/windows_x86_64_msvc |
| windows-collections | 0.3.2 | MIT OR Apache-2.0 | https://crates.io/crates/windows-collections |
| windows-core | 0.62.2 | MIT OR Apache-2.0 | https://crates.io/crates/windows-core |
| windows-future | 0.3.2 | MIT OR Apache-2.0 | https://crates.io/crates/windows-future |
| windows-implement | 0.60.2 | MIT OR Apache-2.0 | https://crates.io/crates/windows-implement |
| windows-interface | 0.59.3 | MIT OR Apache-2.0 | https://crates.io/crates/windows-interface |
| windows-link | 0.2.1 | MIT OR Apache-2.0 | https://crates.io/crates/windows-link |
| windows-numerics | 0.3.1 | MIT OR Apache-2.0 | https://crates.io/crates/windows-numerics |
| windows-registry | 0.6.1 | MIT OR Apache-2.0 | https://crates.io/crates/windows-registry |
| windows-result | 0.4.1 | MIT OR Apache-2.0 | https://crates.io/crates/windows-result |
| windows-strings | 0.5.1 | MIT OR Apache-2.0 | https://crates.io/crates/windows-strings |
| windows-sys | 0.48.0 | MIT OR Apache-2.0 | https://crates.io/crates/windows-sys |
| windows-sys | 0.52.0 | MIT OR Apache-2.0 | https://crates.io/crates/windows-sys |
| windows-sys | 0.60.2 | MIT OR Apache-2.0 | https://crates.io/crates/windows-sys |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | https://crates.io/crates/windows-sys |
| windows-targets | 0.48.5 | MIT OR Apache-2.0 | https://crates.io/crates/windows-targets |
| windows-targets | 0.52.6 | MIT OR Apache-2.0 | https://crates.io/crates/windows-targets |
| windows-targets | 0.53.5 | MIT OR Apache-2.0 | https://crates.io/crates/windows-targets |
| windows-threading | 0.2.1 | MIT OR Apache-2.0 | https://crates.io/crates/windows-threading |
| winnow | 1.0.3 | MIT | https://crates.io/crates/winnow |
| wit-bindgen | 0.51.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://crates.io/crates/wit-bindgen |
| wit-bindgen | 0.57.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://crates.io/crates/wit-bindgen |
| wit-bindgen-core | 0.51.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://crates.io/crates/wit-bindgen-core |
| wit-bindgen-rust | 0.51.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://crates.io/crates/wit-bindgen-rust |
| wit-bindgen-rust-macro | 0.51.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://crates.io/crates/wit-bindgen-rust-macro |
| wit-component | 0.244.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://crates.io/crates/wit-component |
| wit-parser | 0.244.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://crates.io/crates/wit-parser |
| wmi | 0.18.4 | MIT OR Apache-2.0 | https://crates.io/crates/wmi |
| writeable | 0.6.3 | Unicode-3.0 | https://crates.io/crates/writeable |
| ws_stream_wasm | 0.7.5 | Unlicense | https://crates.io/crates/ws_stream_wasm |
| x509-parser | 0.18.1 | MIT OR Apache-2.0 | https://crates.io/crates/x509-parser |
| xml-rs | 0.8.28 | MIT | https://crates.io/crates/xml-rs |
| xmltree | 0.10.3 | MIT | https://crates.io/crates/xmltree |
| xxhash-rust | 0.8.15 | BSL-1.0 | https://crates.io/crates/xxhash-rust |
| yasna | 0.6.0 | MIT OR Apache-2.0 | https://crates.io/crates/yasna |
| yoke | 0.8.2 | Unicode-3.0 | https://crates.io/crates/yoke |
| yoke-derive | 0.8.2 | Unicode-3.0 | https://crates.io/crates/yoke-derive |
| zerocopy | 0.8.49 | BSD-2-Clause OR Apache-2.0 OR MIT | https://crates.io/crates/zerocopy |
| zerocopy-derive | 0.8.49 | BSD-2-Clause OR Apache-2.0 OR MIT | https://crates.io/crates/zerocopy-derive |
| zerofrom | 0.1.8 | Unicode-3.0 | https://crates.io/crates/zerofrom |
| zerofrom-derive | 0.1.7 | Unicode-3.0 | https://crates.io/crates/zerofrom-derive |
| zeroize | 1.8.2 | Apache-2.0 OR MIT | https://crates.io/crates/zeroize |
| zeroize_derive | 1.4.3 | Apache-2.0 OR MIT | https://crates.io/crates/zeroize_derive |
| zerotrie | 0.2.4 | Unicode-3.0 | https://crates.io/crates/zerotrie |
| zerovec | 0.11.6 | Unicode-3.0 | https://crates.io/crates/zerovec |
| zerovec-derive | 0.11.3 | Unicode-3.0 | https://crates.io/crates/zerovec-derive |
| zmij | 1.0.21 | MIT | https://crates.io/crates/zmij |
| zune-core | 0.5.1 | MIT OR Apache-2.0 OR Zlib | https://crates.io/crates/zune-core |
| zune-jpeg | 0.5.15 | MIT OR Apache-2.0 OR Zlib | https://crates.io/crates/zune-jpeg |

## Desktop npm packages

Total packages: 139

| Package | Version | License | Source |
| --- | --- | --- | --- |
| @babel/runtime | 7.29.2 | MIT | https://babel.dev/docs/en/next/babel-runtime |
| @dimforge/rapier3d-compat | 0.12.0 | Apache-2.0 | https://rapier.rs |
| @floating-ui/core | 1.7.5 | MIT | https://floating-ui.com |
| @floating-ui/dom | 1.7.6 | MIT | https://floating-ui.com |
| @floating-ui/react-dom | 2.1.8 | MIT | https://floating-ui.com/docs/react-dom |
| @floating-ui/utils | 0.2.11 | MIT | https://floating-ui.com |
| @mediapipe/tasks-vision | 0.10.17 | Apache-2.0 | http://mediapipe.dev |
| @monogrid/gainmap-js | 3.4.0 | MIT | https://github.com/MONOGRID/gainmap-js#readme |
| @pixiv/three-vrm | 3.5.3 | MIT | https://github.com/pixiv/three-vrm#readme |
| @pixiv/three-vrm-animation | 3.5.3 | MIT | https://github.com/pixiv/three-vrm#readme |
| @pixiv/three-vrm-core | 3.5.3 | MIT | https://github.com/pixiv/three-vrm#readme |
| @pixiv/three-vrm-materials-hdr-emissive-multiplier | 3.5.3 | MIT | https://github.com/pixiv/three-vrm#readme |
| @pixiv/three-vrm-materials-mtoon | 3.5.3 | MIT | https://github.com/pixiv/three-vrm#readme |
| @pixiv/three-vrm-materials-v0compat | 3.5.3 | MIT | https://github.com/pixiv/three-vrm#readme |
| @pixiv/three-vrm-node-constraint | 3.5.3 | MIT | https://github.com/pixiv/three-vrm#readme |
| @pixiv/three-vrm-springbone | 3.5.3 | MIT | https://github.com/pixiv/three-vrm#readme |
| @pixiv/types-vrm-0.0 | 3.5.3 | MIT | https://github.com/pixiv/three-vrm#readme |
| @pixiv/types-vrmc-materials-hdr-emissive-multiplier-1.0 | 3.5.3 | MIT | https://github.com/pixiv/three-vrm#readme |
| @pixiv/types-vrmc-materials-mtoon-1.0 | 3.5.3 | MIT | https://github.com/pixiv/three-vrm#readme |
| @pixiv/types-vrmc-node-constraint-1.0 | 3.5.3 | MIT | https://github.com/pixiv/three-vrm#readme |
| @pixiv/types-vrmc-springbone-1.0 | 3.5.3 | MIT | https://github.com/pixiv/three-vrm#readme |
| @pixiv/types-vrmc-springbone-extended-collider-1.0 | 3.5.3 | MIT | https://github.com/pixiv/three-vrm#readme |
| @pixiv/types-vrmc-vrm-1.0 | 3.5.3 | MIT | https://github.com/pixiv/three-vrm#readme |
| @pixiv/types-vrmc-vrm-animation-1.0 | 3.5.3 | MIT | https://github.com/pixiv/three-vrm#readme |
| @radix-ui/primitive | 1.1.3 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-arrow | 1.1.7 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-compose-refs | 1.1.2 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-context | 1.1.2 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-dialog | 1.1.15 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-dismissable-layer | 1.1.11 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-focus-guards | 1.1.3 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-focus-scope | 1.1.7 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-id | 1.1.1 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-popover | 1.1.15 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-popper | 1.2.8 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-portal | 1.1.9 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-presence | 1.1.5 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-primitive | 2.1.3 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-slot | 1.2.3 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-slot | 1.2.4 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-tooltip | 1.2.8 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-use-callback-ref | 1.1.1 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-use-controllable-state | 1.2.2 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-use-effect-event | 0.0.2 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-use-escape-keydown | 1.1.1 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-use-layout-effect | 1.1.1 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-use-rect | 1.1.1 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-use-size | 1.1.1 | MIT | https://radix-ui.com/primitives |
| @radix-ui/react-visually-hidden | 1.2.3 | MIT | https://radix-ui.com/primitives |
| @radix-ui/rect | 1.1.1 | MIT | https://radix-ui.com/primitives |
| @react-three/drei | 10.7.7 | MIT | https://github.com/pmndrs/drei |
| @react-three/fiber | 9.6.1 | MIT | https://github.com/pmndrs/react-three-fiber#readme |
| @tauri-apps/api | 2.11.0 | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri#readme |
| @tauri-apps/plugin-deep-link | 2.4.9 | MIT OR Apache-2.0 | https://github.com/tauri-apps/plugins-workspace#readme |
| @tauri-apps/plugin-notification | 2.3.3 | MIT OR Apache-2.0 | https://github.com/tauri-apps/plugins-workspace#readme |
| @tauri-apps/plugin-updater | 2.10.1 | MIT OR Apache-2.0 | https://github.com/tauri-apps/plugins-workspace#readme |
| @tweenjs/tween.js | 23.1.3 | MIT | https://github.com/tweenjs/tween.js |
| @types/draco3d | 1.4.10 | MIT | https://github.com/DefinitelyTyped/DefinitelyTyped/tree/master/types/draco3d |
| @types/offscreencanvas | 2019.7.3 | MIT | https://github.com/DefinitelyTyped/DefinitelyTyped/tree/master/types/offscreencanvas |
| @types/react | 19.2.14 | MIT | https://github.com/DefinitelyTyped/DefinitelyTyped/tree/master/types/react |
| @types/react-dom | 19.2.3 | MIT | https://github.com/DefinitelyTyped/DefinitelyTyped/tree/master/types/react-dom |
| @types/react-reconciler | 0.28.9 | MIT | https://github.com/DefinitelyTyped/DefinitelyTyped/tree/master/types/react-reconciler |
| @types/stats.js | 0.17.4 | MIT | https://github.com/DefinitelyTyped/DefinitelyTyped/tree/master/types/stats.js |
| @types/three | 0.184.1 | MIT | https://github.com/DefinitelyTyped/DefinitelyTyped/tree/master/types/three |
| @types/webxr | 0.5.24 | MIT | https://github.com/DefinitelyTyped/DefinitelyTyped/tree/master/types/webxr |
| @use-gesture/core | 10.3.1 | MIT | https://use-gesture.netlify.app |
| @use-gesture/react | 10.3.1 | MIT | https://use-gesture.netlify.app |
| aria-hidden | 1.2.6 | MIT | https://github.com/theKashey/aria-hidden#readme |
| base64-js | 1.5.1 | MIT | https://github.com/beatgammit/base64-js |
| bidi-js | 1.0.3 | MIT | https://github.com/lojjic/bidi-js#readme |
| buffer | 6.0.3 | MIT | https://github.com/feross/buffer |
| camera-controls | 3.1.2 | MIT | https://github.com/yomotsu/camera-controls#readme |
| class-variance-authority | 0.7.1 | Apache-2.0 | https://github.com/joe-bell/cva#readme |
| clsx | 2.1.1 | MIT | https://github.com/lukeed/clsx#readme |
| cookie | 1.1.1 | MIT | https://github.com/jshttp/cookie#readme |
| cross-env | 7.0.3 | MIT | https://github.com/kentcdodds/cross-env#readme |
| cross-spawn | 7.0.6 | MIT | https://github.com/moxystudio/node-cross-spawn |
| csstype | 3.2.3 | MIT | https://github.com/frenic/csstype#readme |
| detect-gpu | 5.0.70 | MIT | https://github.com/pmndrs/detect-gpu#readme |
| detect-node-es | 1.1.0 | MIT | https://github.com/thekashey/detect-node |
| draco3d | 1.5.7 | Apache-2.0 | https://github.com/google/draco#readme |
| fflate | 0.6.10 | MIT | https://101arrowz.github.io/fflate |
| fflate | 0.8.3 | MIT | https://101arrowz.github.io/fflate |
| get-nonce | 1.0.1 | MIT | https://github.com/theKashey/get-nonce |
| glsl-noise | 0.0.0 | MIT | https://github.com/hughsk/glsl-noise#readme |
| hls.js | 1.6.16 | Apache-2.0 | https://github.com/video-dev/hls.js |
| html-parse-stringify | 3.0.1 | MIT | https://github.com/henrikjoreteg/html-parse-stringify |
| i18next | 26.0.6 | MIT | https://www.i18next.com |
| i18next-browser-languagedetector | 8.2.1 | MIT | https://github.com/i18next/i18next-browser-languageDetector |
| ieee754 | 1.2.1 | BSD-3-Clause | https://github.com/feross/ieee754#readme |
| immediate | 3.0.6 | MIT | https://github.com/calvinmetcalf/immediate#readme |
| isexe | 2.0.0 | ISC | https://github.com/isaacs/isexe#readme |
| is-promise | 2.2.2 | MIT | https://github.com/then/is-promise#readme |
| its-fine | 2.0.0 | MIT | https://github.com/pmndrs/its-fine |
| lie | 3.3.0 | MIT | https://github.com/calvinmetcalf/lie#readme |
| lucide-react | 1.8.0 | ISC | https://lucide.dev |
| maath | 0.10.8 | MIT | - |
| meshline | 3.3.1 | MIT | https://github.com/pmndrs/meshline#readme |
| meshoptimizer | 1.1.1 | MIT | https://github.com/zeux/meshoptimizer |
| path-key | 3.1.1 | MIT | https://github.com/sindresorhus/path-key#readme |
| potpack | 1.0.2 | ISC | https://mapbox.github.io/potpack/ |
| promise-worker-transferable | 1.0.4 | Apache-2.0 | https://github.com/terikon/promise-worker-transferable#readme |
| react | 19.2.5 | MIT | https://react.dev/ |
| react-dom | 19.2.5 | MIT | https://react.dev/ |
| react-i18next | 17.0.4 | MIT | https://github.com/i18next/react-i18next |
| react-remove-scroll | 2.7.2 | MIT | https://github.com/theKashey/react-remove-scroll#readme |
| react-remove-scroll-bar | 2.3.8 | MIT | https://github.com/theKashey/react-remove-scroll-bar#readme |
| react-router | 7.14.2 | MIT | https://github.com/remix-run/react-router#readme |
| react-router-dom | 7.14.2 | MIT | https://github.com/remix-run/react-router#readme |
| react-style-singleton | 2.2.3 | MIT | https://github.com/theKashey/react-style-singleton#readme |
| react-use-measure | 2.1.7 | MIT | https://github.com/pmndrs/react-use-measure |
| require-from-string | 2.0.2 | MIT | https://github.com/floatdrop/require-from-string#readme |
| scheduler | 0.27.0 | MIT | https://react.dev/ |
| set-cookie-parser | 2.7.2 | MIT | https://github.com/nfriedly/set-cookie-parser |
| shebang-command | 2.0.0 | MIT | https://github.com/kevva/shebang-command#readme |
| shebang-regex | 3.0.0 | MIT | https://github.com/sindresorhus/shebang-regex#readme |
| stats.js | 0.17.0 | MIT | https://github.com/mrdoob/stats.js |
| stats-gl | 2.4.2 | MIT | https://github.com/RenaudRohlinger/stats-gl |
| suspend-react | 0.1.3 | MIT | https://github.com/pmndrs/suspend-react#readme |
| tailwind-merge | 3.5.0 | MIT | https://github.com/dcastil/tailwind-merge |
| three | 0.184.0 | MIT | https://threejs.org/ |
| three-mesh-bvh | 0.8.3 | MIT | https://github.com/gkjohnson/three-mesh-bvh#readme |
| three-stdlib | 2.36.1 | MIT | https://github.com/pmndrs/three-stdlib |
| troika-three-text | 0.52.4 | MIT | https://github.com/protectwise/troika#readme |
| troika-three-utils | 0.52.4 | MIT | https://github.com/protectwise/troika#readme |
| troika-worker-utils | 0.52.0 | MIT | https://github.com/protectwise/troika#readme |
| tslib | 2.8.1 | 0BSD | https://www.typescriptlang.org/ |
| tunnel-rat | 0.1.2 | MIT | https://github.com/pmndrs/tunnel-rat#readme |
| typescript | 6.0.3 | Apache-2.0 | https://www.typescriptlang.org/ |
| use-callback-ref | 1.3.3 | MIT | https://github.com/theKashey/use-callback-ref#readme |
| use-sidecar | 1.1.3 | MIT | https://github.com/theKashey/use-sidecar |
| use-sync-external-store | 1.6.0 | MIT | https://github.com/facebook/react#readme |
| utility-types | 3.11.0 | MIT | https://github.com/piotrwitek/utility-types |
| void-elements | 3.1.0 | MIT | https://github.com/jadejs/void-elements |
| webgl-constants | 1.1.1 | MIT | - |
| webgl-sdf-generator | 1.1.1 | MIT | https://github.com/lojjic/webgl-sdf-generator#readme |
| which | 2.0.2 | ISC | https://github.com/isaacs/node-which#readme |
| zustand | 4.5.7 | MIT | https://github.com/pmndrs/zustand |
| zustand | 5.0.12 | MIT | https://github.com/pmndrs/zustand |
