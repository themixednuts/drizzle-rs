# Performance Benchmark Results - mac

**Generated:** 2025-08-23 17:39:31 UTC
**Commit:** a6da482df5915e1d046f44de52db5ff4188bdf8a
**Platform:** macOS - ARM64

```
[1m[32m    Updating[0m crates.io index
[1m[32m Downloading[0m crates ...
[1m[32m  Downloaded[0m cfg-if v1.0.3
[1m[32m  Downloaded[0m typenum v1.18.0
[1m[32m  Downloaded[0m quote v1.0.40
[1m[32m  Downloaded[0m proc-macro2 v1.0.101
[1m[32m  Downloaded[0m regex-lite v0.1.6
[1m[32m  Downloaded[0m regex-syntax v0.8.5
[1m[32m  Downloaded[0m once_cell v1.21.3
[1m[32m  Downloaded[0m regex v1.11.1
[1m[32m  Downloaded[0m webpki-roots v0.26.11
[1m[32m  Downloaded[0m zeroize v1.8.1
[1m[32m  Downloaded[0m unicode-ident v1.0.18
[1m[32m  Downloaded[0m uuid v1.18.0
[1m[32m  Downloaded[0m serde v1.0.219
[1m[32m  Downloaded[0m tracing v0.1.41
[1m[32m  Downloaded[0m tower v0.4.13
[1m[32m  Downloaded[0m zerocopy v0.7.35
[1m[32m  Downloaded[0m rusqlite v0.34.0
[1m[32m  Downloaded[0m serde_json v1.0.143
[1m[32m  Downloaded[0m rustls-webpki v0.102.8
[1m[32m  Downloaded[0m h2 v0.3.27
[1m[32m  Downloaded[0m webpki-roots v1.0.2
[1m[32m  Downloaded[0m zerocopy v0.8.26
[1m[32m  Downloaded[0m syn v2.0.106
[1m[32m  Downloaded[0m rustix v0.38.44
[1m[32m  Downloaded[0m libc v0.2.175
[1m[32m  Downloaded[0m rustls v0.22.4
[1m[32m  Downloaded[0m vcpkg v0.2.15
[1m[32m  Downloaded[0m bindgen v0.66.1
[1m[32m  Downloaded[0m tokio-util v0.7.16
[1m[32m  Downloaded[0m security-framework v2.11.1
[1m[32m  Downloaded[0m rand v0.9.2
[1m[32m  Downloaded[0m rustix v1.0.8
[1m[32m  Downloaded[0m hashbrown v0.15.5
[1m[32m  Downloaded[0m tonic v0.11.0
[1m[32m  Downloaded[0m http v0.2.12
[1m[32m  Downloaded[0m hashbrown v0.14.5
[1m[32m  Downloaded[0m clap_builder v4.5.44
[1m[32m  Downloaded[0m libsql-sqlite3-parser v0.13.0
[1m[32m  Downloaded[0m libsql-rusqlite v0.9.20
[1m[32m  Downloaded[0m libsql v0.9.20
[1m[32m  Downloaded[0m hyper v0.14.32
[1m[32m  Downloaded[0m hashbrown v0.12.3
[1m[32m  Downloaded[0m futures-util v0.3.31
[1m[32m  Downloaded[0m base64 v0.21.7
[1m[32m  Downloaded[0m libsql_replication v0.9.20
[1m[32m  Downloaded[0m socket2 v0.6.0
[1m[32m  Downloaded[0m libsql-sys v0.9.20
[1m[32m  Downloaded[0m tonic-web v0.11.0
[1m[32m  Downloaded[0m socket2 v0.5.10
[1m[32m  Downloaded[0m rand_core v0.9.3
[1m[32m  Downloaded[0m pin-project-internal v1.1.10
[1m[32m  Downloaded[0m pin-project v1.1.10
[1m[32m  Downloaded[0m tokio v1.47.1
[1m[32m  Downloaded[0m memchr v2.7.5
[1m[32m  Downloaded[0m divan v0.1.21
[1m[32m  Downloaded[0m crc32fast v1.5.0
[1m[32m  Downloaded[0m compact_str v0.9.0
[1m[32m  Downloaded[0m clap v4.5.45
[1m[32m  Downloaded[0m clang-sys v1.8.1
[1m[32m  Downloaded[0m tracing-core v0.1.34
[1m[32m  Downloaded[0m tower-http v0.4.4
[1m[32m  Downloaded[0m serde_derive v1.0.219
[1m[32m  Downloaded[0m rustversion v1.0.22
[1m[32m  Downloaded[0m rustls-native-certs v0.7.3
[1m[32m  Downloaded[0m rand_core v0.6.4
[1m[32m  Downloaded[0m rand_chacha v0.3.1
[1m[32m  Downloaded[0m prost v0.12.6
[1m[32m  Downloaded[0m pkg-config v0.3.32
[1m[32m  Downloaded[0m pin-utils v0.1.0
[1m[32m  Downloaded[0m phf_generator v0.11.3
[1m[32m  Downloaded[0m phf_codegen v0.11.3
[1m[32m  Downloaded[0m percent-encoding v2.3.1
[1m[32m  Downloaded[0m matchit v0.7.3
[1m[32m  Downloaded[0m libloading v0.8.8
[1m[32m  Downloaded[0m indexmap v2.10.0
[1m[32m  Downloaded[0m indexmap v1.9.3
[1m[32m  Downloaded[0m iana-time-zone v0.1.63
[1m[32m  Downloaded[0m getrandom v0.3.3
[1m[32m  Downloaded[0m generic-array v0.14.7
[1m[32m  Downloaded[0m futures v0.3.31
[1m[32m  Downloaded[0m fallible-iterator v0.2.0
[1m[32m  Downloaded[0m divan-macros v0.1.21
[1m[32m  Downloaded[0m anstyle v1.0.11
[1m[32m  Downloaded[0m zerocopy-derive v0.7.35
[1m[32m  Downloaded[0m which v4.4.2
[1m[32m  Downloaded[0m version_check v0.9.5
[1m[32m  Downloaded[0m tracing-attributes v0.1.30
[1m[32m  Downloaded[0m tower-service v0.3.3
[1m[32m  Downloaded[0m tokio-rustls v0.25.0
[1m[32m  Downloaded[0m tokio-macros v2.5.0
[1m[32m  Downloaded[0m tokio-io-timeout v1.2.1
[1m[32m  Downloaded[0m thiserror-impl v1.0.69
[1m[32m  Downloaded[0m terminal_size v0.4.3
[1m[32m  Downloaded[0m static_assertions v1.1.0
[1m[32m  Downloaded[0m shlex v1.3.0
[1m[32m  Downloaded[0m rand_chacha v0.9.0
[1m[32m  Downloaded[0m prost-derive v0.12.6
[1m[32m  Downloaded[0m ppv-lite86 v0.2.21
[1m[32m  Downloaded[0m phf_shared v0.11.3
[1m[32m  Downloaded[0m peeking_take_while v0.1.2
[1m[32m  Downloaded[0m paste v1.0.15
[1m[32m  Downloaded[0m log v0.4.27
[1m[32m  Downloaded[0m lock_api v0.4.13
[1m[32m  Downloaded[0m lazycell v1.3.0
[1m[32m  Downloaded[0m itoa v1.0.15
[1m[32m  Downloaded[0m inout v0.1.4
[1m[32m  Downloaded[0m hyper-timeout v0.4.1
[1m[32m  Downloaded[0m httpdate v1.0.3
[1m[32m  Downloaded[0m http-body v0.4.6
[1m[32m  Downloaded[0m home v0.5.11
[1m[32m  Downloaded[0m glob v0.3.3
[1m[32m  Downloaded[0m futures-task v0.3.31
[1m[32m  Downloaded[0m futures-io v0.3.31
[1m[32m  Downloaded[0m fallible-streaming-iterator v0.1.9
[1m[32m  Downloaded[0m fallible-iterator v0.3.0
[1m[32m  Downloaded[0m either v1.15.0
[1m[32m  Downloaded[0m crypto-common v0.1.6
[1m[32m  Downloaded[0m bitflags v1.3.2
[1m[32m  Downloaded[0m axum-core v0.3.4
[1m[32m  Downloaded[0m async-trait v0.1.89
[1m[32m  Downloaded[0m async-stream-impl v0.3.6
[1m[32m  Downloaded[0m async-stream v0.3.6
[1m[32m  Downloaded[0m want v0.3.1
[1m[32m  Downloaded[0m untrusted v0.9.0
[1m[32m  Downloaded[0m uncased v0.9.10
[1m[32m  Downloaded[0m try-lock v0.2.5
[1m[32m  Downloaded[0m thiserror v1.0.69
[1m[32m  Downloaded[0m subtle v2.6.1
[1m[32m  Downloaded[0m slab v0.4.11
[1m[32m  Downloaded[0m siphasher v1.0.1
[1m[32m  Downloaded[0m signal-hook-registry v1.4.6
[1m[32m  Downloaded[0m security-framework-sys v2.14.0
[1m[32m  Downloaded[0m ryu v1.0.20
[1m[32m  Downloaded[0m rustls-pki-types v1.12.0
[1m[32m  Downloaded[0m rustls-pemfile v2.2.0
[1m[32m  Downloaded[0m rustc-hash v1.1.0
[1m[32m  Downloaded[0m ring v0.17.14
[1m[32m  Downloaded[0m regex-automata v0.4.9
[1m[32m  Downloaded[0m nom v7.1.3
[1m[32m  Downloaded[0m libsqlite3-sys v0.32.0
[1m[32m  Downloaded[0m lazy_static v1.5.0
[1m[32m  Downloaded[0m hyper-rustls v0.25.0
[1m[32m  Downloaded[0m heck v0.5.0
[1m[32m  Downloaded[0m futures-sink v0.3.31
[1m[32m  Downloaded[0m futures-macro v0.3.31
[1m[32m  Downloaded[0m futures-channel v0.3.31
[1m[32m  Downloaded[0m foldhash v0.1.5
[1m[32m  Downloaded[0m fnv v1.0.7
[1m[32m  Downloaded[0m errno v0.3.13
[1m[32m  Downloaded[0m cpufeatures v0.2.17
[1m[32m  Downloaded[0m core-foundation-sys v0.8.7
[1m[32m  Downloaded[0m core-foundation v0.9.4
[1m[32m  Downloaded[0m cmake v0.1.54
[1m[32m  Downloaded[0m clap_lex v0.7.5
[1m[32m  Downloaded[0m cipher v0.4.4
[1m[32m  Downloaded[0m chrono v0.4.41
[1m[32m  Downloaded[0m parking_lot_core v0.9.11
[1m[32m  Downloaded[0m parking_lot v0.12.4
[1m[32m  Downloaded[0m minimal-lexical v0.2.1
[1m[32m  Downloaded[0m libsql-hrana v0.9.20
[1m[32m  Downloaded[0m itertools v0.12.1
[1m[32m  Downloaded[0m httparse v1.10.1
[1m[32m  Downloaded[0m http-range-header v0.3.1
[1m[32m  Downloaded[0m hashlink v0.10.0
[1m[32m  Downloaded[0m hashlink v0.8.4
[1m[32m  Downloaded[0m getrandom v0.2.16
[1m[32m  Downloaded[0m futures-executor v0.3.31
[1m[32m  Downloaded[0m futures-core v0.3.31
[1m[32m  Downloaded[0m equivalent v1.0.2
[1m[32m  Downloaded[0m condtype v1.3.0
[1m[32m  Downloaded[0m cc v1.2.33
[1m[32m  Downloaded[0m cbc v0.1.2
[1m[32m  Downloaded[0m axum v0.6.20
[1m[32m  Downloaded[0m aes v0.8.4
[1m[32m  Downloaded[0m mio v1.0.4
[1m[32m  Downloaded[0m bitflags v2.9.2
[1m[32m  Downloaded[0m rand v0.8.5
[1m[32m  Downloaded[0m pin-project-lite v0.2.16
[1m[32m  Downloaded[0m num-traits v0.2.19
[1m[32m  Downloaded[0m prettyplease v0.2.37
[1m[32m  Downloaded[0m aho-corasick v1.1.3
[1m[32m  Downloaded[0m ahash v0.8.12
[1m[32m  Downloaded[0m cexpr v0.6.0
[1m[32m  Downloaded[0m block-padding v0.3.3
[1m[32m  Downloaded[0m allocator-api2 v0.2.21
[1m[32m  Downloaded[0m mime v0.3.17
[1m[32m  Downloaded[0m castaway v0.2.4
[1m[32m  Downloaded[0m bytes v1.10.1
[1m[32m  Downloaded[0m byteorder v1.5.0
[1m[32m  Downloaded[0m tower-layer v0.3.3
[1m[32m  Downloaded[0m tokio-stream v0.1.17
[1m[32m  Downloaded[0m sync_wrapper v0.1.2
[1m[32m  Downloaded[0m smallvec v1.15.1
[1m[32m  Downloaded[0m scopeguard v1.2.0
[1m[32m  Downloaded[0m phf v0.11.3
[1m[32m  Downloaded[0m bincode v1.3.3
[1m[32m  Downloaded[0m autocfg v1.5.0
[1m[32m  Downloaded[0m anyhow v1.0.99
[1m[32m  Downloaded[0m libsql-ffi v0.9.20
[1m[32m   Compiling[0m proc-macro2 v1.0.101
[1m[32m   Compiling[0m unicode-ident v1.0.18
[1m[32m   Compiling[0m libc v0.2.175
[1m[32m   Compiling[0m cfg-if v1.0.3
[1m[32m   Compiling[0m version_check v0.9.5
[1m[32m   Compiling[0m zerocopy v0.8.26
[1m[32m   Compiling[0m serde v1.0.219
[1m[32m   Compiling[0m autocfg v1.5.0
[1m[32m   Compiling[0m shlex v1.3.0
[1m[32m   Compiling[0m quote v1.0.40
[1m[32m   Compiling[0m pin-project-lite v0.2.16
[1m[32m   Compiling[0m cc v1.2.33
[1m[32m   Compiling[0m syn v2.0.106
[1m[32m   Compiling[0m smallvec v1.15.1
[1m[32m   Compiling[0m lock_api v0.4.13
[1m[32m   Compiling[0m futures-core v0.3.31
[1m[32m   Compiling[0m parking_lot_core v0.9.11
[1m[32m   Compiling[0m once_cell v1.21.3
[1m[32m   Compiling[0m scopeguard v1.2.0
[1m[32m   Compiling[0m memchr v2.7.5
[1m[32m   Compiling[0m futures-sink v0.3.31
[1m[32m   Compiling[0m parking_lot v0.12.4
[1m[32m   Compiling[0m signal-hook-registry v1.4.6
[1m[32m   Compiling[0m socket2 v0.6.0
[1m[32m   Compiling[0m serde_derive v1.0.219
[1m[32m   Compiling[0m tokio-macros v2.5.0
[1m[32m   Compiling[0m mio v1.0.4
[1m[32m   Compiling[0m bitflags v2.9.2
[1m[32m   Compiling[0m itoa v1.0.15
[1m[32m   Compiling[0m log v0.4.27
[1m[32m   Compiling[0m futures-macro v0.3.31
[1m[32m   Compiling[0m futures-channel v0.3.31
[1m[32m   Compiling[0m getrandom v0.2.16
[1m[32m   Compiling[0m pin-utils v0.1.0
[1m[32m   Compiling[0m slab v0.4.11
[1m[32m   Compiling[0m either v1.15.0
[1m[32m   Compiling[0m futures-io v0.3.31
[1m[32m   Compiling[0m futures-task v0.3.31
[1m[32m   Compiling[0m futures-util v0.3.31
[1m[32m   Compiling[0m foldhash v0.1.5
[1m[32m   Compiling[0m rustversion v1.0.22
[1m[32m   Compiling[0m hashbrown v0.15.5
[1m[32m   Compiling[0m tracing-attributes v0.1.30
[1m[32m   Compiling[0m tracing-core v0.1.34
[1m[32m   Compiling[0m uncased v0.9.10
[1m[32m   Compiling[0m bytes v1.10.1
[1m[32m   Compiling[0m fnv v1.0.7
[1m[32m   Compiling[0m tracing v0.1.41
[1m[32m   Compiling[0m tokio v1.47.1
[1m[32m   Compiling[0m http v0.2.12
[1m[32m   Compiling[0m glob v0.3.3
[1m[32m   Compiling[0m clang-sys v1.8.1
[1m[32m   Compiling[0m ppv-lite86 v0.2.21
[1m[32m   Compiling[0m equivalent v1.0.2
[1m[32m   Compiling[0m prettyplease v0.2.37
[1m[32m   Compiling[0m rustix v0.38.44
[1m[32m   Compiling[0m anyhow v1.0.99
[1m[32m   Compiling[0m tower-service v0.3.3
[1m[32m   Compiling[0m typenum v1.18.0
[1m[32m   Compiling[0m indexmap v2.10.0
[1m[32m   Compiling[0m http-body v0.4.6
[1m[32m   Compiling[0m errno v0.3.13
[1m[32m   Compiling[0m generic-array v0.14.7
[1m[32m   Compiling[0m minimal-lexical v0.2.1
[1m[32m   Compiling[0m httparse v1.10.1
[1m[32m   Compiling[0m tokio-util v0.7.16
[1m[32m   Compiling[0m zeroize v1.8.1
[1m[32m   Compiling[0m regex-syntax v0.8.5
[1m[32m   Compiling[0m nom v7.1.3
[1m[32m   Compiling[0m regex-automata v0.4.9
[1m[32m   Compiling[0m rustls-pki-types v1.12.0
[1m[32m   Compiling[0m rand_core v0.6.4
[1m[32m   Compiling[0m libloading v0.8.8
[1m[32m   Compiling[0m indexmap v1.9.3
[1m[32m   Compiling[0m getrandom v0.3.3
[1m[32m   Compiling[0m home v0.5.11
[1m[32m   Compiling[0m bindgen v0.66.1
[1m[32m   Compiling[0m try-lock v0.2.5
[1m[32m   Compiling[0m want v0.3.1
[1m[32m   Compiling[0m which v4.4.2
[1m[32m   Compiling[0m rand_chacha v0.3.1
[1m[32m   Compiling[0m regex v1.11.1
[1m[32m   Compiling[0m cexpr v0.6.0
[1m[32m   Compiling[0m h2 v0.3.27
[1m[32m   Compiling[0m pin-project-internal v1.1.10
[1m[32m   Compiling[0m ring v0.17.14
[1m[32m   Compiling[0m socket2 v0.5.10
[1m[32m   Compiling[0m ahash v0.8.12
[1m[32m   Compiling[0m tower-layer v0.3.3
[1m[32m   Compiling[0m peeking_take_while v0.1.2
[1m[32m   Compiling[0m hashbrown v0.12.3
[1m[32m   Compiling[0m httpdate v1.0.3
[1m[32m   Compiling[0m rustc-hash v1.1.0
[1m[32m   Compiling[0m lazycell v1.3.0
[1m[32m   Compiling[0m core-foundation-sys v0.8.7
[1m[32m   Compiling[0m lazy_static v1.5.0
[1m[32m   Compiling[0m hyper v0.14.32
[1m[32m   Compiling[0m pin-project v1.1.10
[1m[32m   Compiling[0m rand v0.8.5
[1m[32m   Compiling[0m axum-core v0.3.4
[1m[32m   Compiling[0m itertools v0.12.1
[1m[32m   Compiling[0m cmake v0.1.54
[1m[32m   Compiling[0m siphasher v1.0.1
[1m[32m   Compiling[0m phf_shared v0.11.3
[1m[32m   Compiling[0m libsql-ffi v0.9.20
[1m[32m   Compiling[0m tower v0.4.13
[1m[32m   Compiling[0m prost-derive v0.12.6
[1m[32m   Compiling[0m block-padding v0.3.3
[1m[32m   Compiling[0m axum v0.6.20
[1m[32m   Compiling[0m async-trait v0.1.89
[1m[32m   Compiling[0m allocator-api2 v0.2.21
[1m[32m   Compiling[0m mime v0.3.17
[1m[32m   Compiling[0m untrusted v0.9.0
[1m[32m   Compiling[0m prost v0.12.6
[1m[32m   Compiling[0m hashbrown v0.14.5
[1m[32m   Compiling[0m inout v0.1.4
[1m[32m   Compiling[0m phf_generator v0.11.3
[1m[32m   Compiling[0m crypto-common v0.1.6
[1m[32m   Compiling[0m tokio-io-timeout v1.2.1
[1m[32m   Compiling[0m async-stream-impl v0.3.6
[1m[32m   Compiling[0m bitflags v1.3.2
[1m[32m   Compiling[0m percent-encoding v2.3.1
[1m[32m   Compiling[0m sync_wrapper v0.1.2
[1m[32m   Compiling[0m matchit v0.7.3
[1m[32m   Compiling[0m fallible-streaming-iterator v0.1.9
[1m[32m   Compiling[0m rustls v0.22.4
[1m[32m   Compiling[0m base64 v0.21.7
[1m[32m   Compiling[0m async-stream v0.3.6
[1m[32m   Compiling[0m hyper-timeout v0.4.1
[1m[32m   Compiling[0m cipher v0.4.4
[1m[32m   Compiling[0m phf_codegen v0.11.3
[1m[32m   Compiling[0m rustls-webpki v0.102.8
[1m[32m   Compiling[0m hashlink v0.8.4
[1m[32m   Compiling[0m core-foundation v0.9.4
[1m[32m   Compiling[0m security-framework-sys v2.14.0
[1m[32m   Compiling[0m tokio-stream v0.1.17
[1m[32m   Compiling[0m zerocopy-derive v0.7.35
[1m[32m   Compiling[0m num-traits v0.2.19
[1m[32m   Compiling[0m ryu v1.0.20
[1m[32m   Compiling[0m pkg-config v0.3.32
[1m[32m   Compiling[0m vcpkg v0.2.15
[1m[32m   Compiling[0m fallible-iterator v0.2.0
[1m[32m   Compiling[0m serde_json v1.0.143
[1m[32m   Compiling[0m byteorder v1.5.0
[1m[32m   Compiling[0m thiserror v1.0.69
[1m[32m   Compiling[0m subtle v2.6.1
[1m[32m   Compiling[0m zerocopy v0.7.35
[1m[32m   Compiling[0m libsqlite3-sys v0.32.0
[1m[32m   Compiling[0m tonic v0.11.0
[1m[32m   Compiling[0m security-framework v2.11.1
[1m[32m   Compiling[0m libsql-sqlite3-parser v0.13.0
[1m[32m   Compiling[0m uuid v1.18.0
[1m[32m   Compiling[0m rustls-pemfile v2.2.0
[1m[32m   Compiling[0m webpki-roots v1.0.2
[1m[32m   Compiling[0m thiserror-impl v1.0.69
[1m[32m   Compiling[0m libsql-rusqlite v0.9.20
[1m[32m   Compiling[0m cpufeatures v0.2.17
[1m[32m   Compiling[0m http-range-header v0.3.1
[1m[32m   Compiling[0m fallible-iterator v0.3.0
[1m[32m   Compiling[0m crc32fast v1.5.0
[1m[32m   Compiling[0m paste v1.0.15
[1m[32m   Compiling[0m libsql-sys v0.9.20
[1m[32m   Compiling[0m tower-http v0.4.4
[1m[32m   Compiling[0m aes v0.8.4
[1m[32m   Compiling[0m webpki-roots v0.26.11
[1m[32m   Compiling[0m rustls-native-certs v0.7.3
[1m[32m   Compiling[0m tokio-rustls v0.25.0
[1m[32m   Compiling[0m phf v0.11.3
[1m[32m   Compiling[0m cbc v0.1.2
[1m[32m   Compiling[0m iana-time-zone v0.1.63
[1m[32m   Compiling[0m futures-executor v0.3.31
[1m[32m   Compiling[0m futures v0.3.31
[1m[32m   Compiling[0m chrono v0.4.41
[1m[32m   Compiling[0m libsql_replication v0.9.20
[1m[32m   Compiling[0m hyper-rustls v0.25.0
[1m[32m   Compiling[0m tonic-web v0.11.0
[1m[32m   Compiling[0m libsql-hrana v0.9.20
[1m[32m   Compiling[0m castaway v0.2.4
[1m[32m   Compiling[0m bincode v1.3.3
[1m[32m   Compiling[0m hashlink v0.10.0
[1m[32m   Compiling[0m rustix v1.0.8
[1m[32m   Compiling[0m static_assertions v1.1.0
[1m[32m   Compiling[0m compact_str v0.9.0
[1m[32m   Compiling[0m libsql v0.9.20
[1m[32m   Compiling[0m terminal_size v0.4.3
[1m[32m   Compiling[0m anstyle v1.0.11
[1m[32m   Compiling[0m clap_lex v0.7.5
[1m[32m   Compiling[0m clap_builder v4.5.44
[1m[32m   Compiling[0m rand_core v0.9.3
[1m[32m   Compiling[0m heck v0.5.0
[1m[32m   Compiling[0m drizzle-macros v0.1.2 (/Users/runner/work/drizzle-rs/drizzle-rs/procmacros)
[1m[32m   Compiling[0m rusqlite v0.34.0
[1m[32m   Compiling[0m drizzle-core v0.1.2 (/Users/runner/work/drizzle-rs/drizzle-rs/core)
[1m[32m   Compiling[0m drizzle-mysql v0.1.2 (/Users/runner/work/drizzle-rs/drizzle-rs/mysql)
[1m[32m   Compiling[0m drizzle-postgres v0.1.2 (/Users/runner/work/drizzle-rs/drizzle-rs/postgres)
[1m[32m   Compiling[0m drizzle-sqlite v0.1.2 (/Users/runner/work/drizzle-rs/drizzle-rs/sqlite)
[1m[32m   Compiling[0m rand_chacha v0.9.0
[1m[32m   Compiling[0m clap v4.5.45
[1m[32m   Compiling[0m divan-macros v0.1.21
[1m[32m   Compiling[0m condtype v1.3.0
[1m[32m   Compiling[0m regex-lite v0.1.6
[1m[32m   Compiling[0m rand v0.9.2
[1m[32m   Compiling[0m divan v0.1.21
[1m[32m   Compiling[0m drizzle-rs v0.1.2 (/Users/runner/work/drizzle-rs/drizzle-rs)
[0m[1m[33mwarning[0m[0m[1m: unused import: `SQLParam`[0m
[0m [0m[0m[1m[38;5;12m--> [0m[0msrc/drizzle/sqlite/libsql/prepared.rs:9:31[0m
[0m  [0m[0m[1m[38;5;12m|[0m
[0m[1m[38;5;12m9[0m[0m [0m[0m[1m[38;5;12m|[0m[0m [0m[0muse drizzle_core::{ParamBind, SQLParam, ToSQL};[0m
[0m  [0m[0m[1m[38;5;12m|[0m[0m                               [0m[0m[1m[33m^^^^^^^^[0m
[0m  [0m[0m[1m[38;5;12m|[0m
[0m  [0m[0m[1m[38;5;12m= [0m[0m[1mnote[0m[0m: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default[0m

[1m[33mwarning[0m[1m:[0m `drizzle-rs` (lib) generated 1 warning (run `cargo fix --lib -p drizzle-rs` to apply 1 suggestion)
[1m[32m    Finished[0m `bench` profile [optimized + debuginfo] target(s) in 1m 59s
[1m[32m     Running[0m benches/performance_comparison.rs (target/release/deps/performance_comparison-32a8e7e9fd724157)
Timer precision: 41 ns
performance_comparison     fastest       │ slowest       │ median        │ mean          │ samples │ iters
├─ libsql                                │               │               │               │         │
│  ├─ bulk_insert                        │               │               │               │         │
│  │  ├─ drizzle_prepared  751.4 µs      │ 1.123 ms      │ 782.1 µs      │ 795.2 µs      │ 100     │ 100
│  │  │                    max alloc:    │               │               │               │         │
│  │  │                      2006        │ 2006          │ 2006          │ 2006          │         │
│  │  │                      162.3 KB    │ 162.3 KB      │ 162.3 KB      │ 162.3 KB      │         │
│  │  │                    alloc:        │               │               │               │         │
│  │  │                      4008        │ 4008          │ 4008          │ 4008          │         │
│  │  │                      115.5 KB    │ 115.5 KB      │ 115.5 KB      │ 115.5 KB      │         │
│  │  │                    dealloc:      │               │               │               │         │
│  │  │                      6014        │ 6014          │ 6014          │ 6014          │         │
│  │  │                      401.1 KB    │ 401.1 KB      │ 401.1 KB      │ 401.1 KB      │         │
│  │  │                    grow:         │               │               │               │         │
│  │  │                      16          │ 16            │ 16            │ 16            │         │
│  │  │                      114.1 KB    │ 114.1 KB      │ 114.1 KB      │ 114.1 KB      │         │
│  │  ╰─ raw               621.1 µs      │ 807.1 µs      │ 667.7 µs      │ 668.3 µs      │ 100     │ 100
│  │                       max alloc:    │               │               │               │         │
│  │                         4           │ 4             │ 4             │ 4             │         │
│  │                         65.53 KB    │ 65.53 KB      │ 65.53 KB      │ 65.53 KB      │         │
│  │                       alloc:        │               │               │               │         │
│  │                         5           │ 5             │ 5             │ 5             │         │
│  │                         16.3 KB     │ 16.3 KB       │ 16.3 KB       │ 16.3 KB       │         │
│  │                       dealloc:      │               │               │               │         │
│  │                         2010        │ 2010          │ 2010          │ 2010          │         │
│  │                         182.3 KB    │ 182.3 KB      │ 182.3 KB      │ 182.3 KB      │         │
│  │                       grow:         │               │               │               │         │
│  │                         9           │ 9             │ 9             │ 9             │         │
│  │                         65.4 KB     │ 65.4 KB       │ 65.4 KB       │ 65.4 KB       │         │
│  ├─ insert                             │               │               │               │         │
│  │  ├─ drizzle           6.463 µs      │ 35.79 µs      │ 6.672 µs      │ 7.129 µs      │ 100     │ 100
│  │  │                    max alloc:    │               │               │               │         │
│  │  │                      13          │ 13            │ 13            │ 13            │         │
│  │  │                      1.744 KB    │ 1.744 KB      │ 1.744 KB      │ 1.744 KB      │         │
│  │  │                    alloc:        │               │               │               │         │
│  │  │                      34          │ 34            │ 34            │ 34            │         │
│  │  │                      2.887 KB    │ 2.887 KB      │ 2.887 KB      │ 2.887 KB      │         │
│  │  │                    dealloc:      │               │               │               │         │
│  │  │                      37          │ 37            │ 37            │ 37            │         │
│  │  │                      3.727 KB    │ 3.727 KB      │ 3.727 KB      │ 3.727 KB      │         │
│  │  │                    grow:         │               │               │               │         │
│  │  │                      1           │ 1             │ 1             │ 1             │         │
│  │  │                      224 B       │ 224 B         │ 224 B         │ 224 B         │         │
│  │  │                    shrink:       │               │               │               │         │
│  │  │                      1           │ 1             │ 1             │ 1             │         │
│  │  │                      32 B        │ 32 B          │ 32 B          │ 32 B          │         │
│  │  ├─ drizzle_prepared  5.44 µs       │ 14.98 µs      │ 5.899 µs      │ 6.191 µs      │ 100     │ 100
│  │  │                    max alloc:    │               │               │               │         │
│  │  │                      8           │ 8             │ 8             │ 8             │         │
│  │  │                      407 B       │ 407 B         │ 407 B         │ 407 B         │         │
│  │  │                    alloc:        │               │               │               │         │
│  │  │                      11          │ 11            │ 11            │ 11            │         │
│  │  │                      505 B       │ 505 B         │ 505 B         │ 505 B         │         │
│  │  │                    dealloc:      │               │               │               │         │
│  │  │                      19          │ 19            │ 19            │ 19            │         │
│  │  │                      1.383 KB    │ 1.383 KB      │ 1.383 KB      │ 1.383 KB      │         │
│  │  ╰─ raw               5.561 µs      │ 20.31 µs      │ 5.852 µs      │ 6.017 µs      │ 100     │ 100
│  │                       max alloc:    │               │               │               │         │
│  │                         7           │ 7             │ 7             │ 7             │         │
│  │                         346 B       │ 346 B         │ 346 B         │ 346 B         │         │
│  │                       alloc:        │               │               │               │         │
│  │                         8           │ 8             │ 8             │ 8             │         │
│  │                         378 B       │ 378 B         │ 378 B         │ 378 B         │         │
│  │                       dealloc:      │               │               │               │         │
│  │                         11          │ 11            │ 11            │ 11            │         │
│  │                         1.026 KB    │ 1.026 KB      │ 1.026 KB      │ 1.026 KB      │         │
│  ╰─ select                             │               │               │               │         │
│     ├─ drizzle           43.22 µs      │ 126.8 µs      │ 45.09 µs      │ 46.81 µs      │ 100     │ 100
│     │                    max alloc:    │               │               │               │         │
│     │                      207         │ 207           │ 207           │ 207           │         │
│     │                      11.17 KB    │ 11.17 KB      │ 11.17 KB      │ 11.17 KB      │         │
│     │                    alloc:        │               │               │               │         │
│     │                      615         │ 615           │ 615           │ 615           │         │
│     │                      75.45 KB    │ 75.45 KB      │ 75.45 KB      │ 75.45 KB      │         │
│     │                    dealloc:      │               │               │               │         │
│     │                      618         │ 618           │ 618           │ 618           │         │
│     │                      83.11 KB    │ 83.11 KB      │ 83.11 KB      │ 83.11 KB      │         │
│     │                    grow:         │               │               │               │         │
│     │                      8           │ 8             │ 8             │ 8             │         │
│     │                      7.007 KB    │ 7.007 KB      │ 7.007 KB      │ 7.007 KB      │         │
│     ├─ drizzle_prepared  43.73 µs      │ 87.44 µs      │ 45.42 µs      │ 46.77 µs      │ 100     │ 100
│     │                    max alloc:    │               │               │               │         │
│     │                      207         │ 207           │ 207           │ 207           │         │
│     │                      11.13 KB    │ 11.13 KB      │ 11.13 KB      │ 11.13 KB      │         │
│     │                    alloc:        │               │               │               │         │
│     │                      613         │ 613           │ 613           │ 613           │         │
│     │                      75.41 KB    │ 75.41 KB      │ 75.41 KB      │ 75.41 KB      │         │
│     │                    dealloc:      │               │               │               │         │
│     │                      618         │ 618           │ 618           │ 618           │         │
│     │                      83.1 KB     │ 83.1 KB       │ 83.1 KB       │ 83.1 KB       │         │
│     │                    grow:         │               │               │               │         │
│     │                      5           │ 5             │ 5             │ 5             │         │
│     │                      6.944 KB    │ 6.944 KB      │ 6.944 KB      │ 6.944 KB      │         │
│     ╰─ raw               41.71 µs      │ 79.2 µs       │ 44.68 µs      │ 47.14 µs      │ 100     │ 100
│                          max alloc:    │               │               │               │         │
│                            208         │ 208           │ 208           │ 208           │         │
│                            11.74 KB    │ 11.74 KB      │ 11.74 KB      │ 11.74 KB      │         │
│                          alloc:        │               │               │               │         │
│                            612         │ 612           │ 612           │ 612           │         │
│                            75.35 KB    │ 75.35 KB      │ 75.35 KB      │ 75.35 KB      │         │
│                          dealloc:      │               │               │               │         │
│                            615         │ 615           │ 615           │ 615           │         │
│                            82.94 KB    │ 82.94 KB      │ 82.94 KB      │ 82.94 KB      │         │
│                          grow:         │               │               │               │         │
│                            5           │ 5             │ 5             │ 5             │         │
│                            6.944 KB    │ 6.944 KB      │ 6.944 KB      │ 6.944 KB      │         │
╰─ rusqlite                              │               │               │               │         │
   ├─ bulk_insert                        │               │               │               │         │
   │  ├─ drizzle_prepared  655 µs        │ 904.8 µs      │ 737 µs        │ 740.4 µs      │ 100     │ 100
   │  │                    max alloc:    │               │               │               │         │
   │  │                      2002        │ 2002          │ 2002          │ 2002          │         │
   │  │                      97.97 KB    │ 97.97 KB      │ 97.97 KB      │ 97.97 KB      │         │
   │  │                    alloc:        │               │               │               │         │
   │  │                      4003        │ 4003          │ 4003          │ 4003          │         │
   │  │                      51.26 KB    │ 51.26 KB      │ 51.26 KB      │ 51.26 KB      │         │
   │  │                    dealloc:      │               │               │               │         │
   │  │                      6007        │ 6007          │ 6007          │ 6007          │         │
   │  │                      270.9 KB    │ 270.9 KB      │ 270.9 KB      │ 270.9 KB      │         │
   │  │                    grow:         │               │               │               │         │
   │  │                      7           │ 7             │ 7             │ 7             │         │
   │  │                      48.76 KB    │ 48.76 KB      │ 48.76 KB      │ 48.76 KB      │         │
   │  ╰─ raw               594 µs        │ 708.7 µs      │ 625.9 µs      │ 625.8 µs      │ 100     │ 100
   │                       alloc:        │               │               │               │         │
   │                         1           │ 1             │ 1             │ 1             │         │
   │                         64 B        │ 64 B          │ 64 B          │ 64 B          │         │
   │                       dealloc:      │               │               │               │         │
   │                         2004        │ 2004          │ 2004          │ 2004          │         │
   │                         100 KB      │ 100 KB        │ 100 KB        │ 100 KB        │         │
   ├─ insert                             │               │               │               │         │
   │  ├─ drizzle           6.08 µs       │ 15.45 µs      │ 6.246 µs      │ 6.398 µs      │ 100     │ 100
   │  │                    max alloc:    │               │               │               │         │
   │  │                      13          │ 13            │ 13            │ 13            │         │
   │  │                      1.744 KB    │ 1.744 KB      │ 1.744 KB      │ 1.744 KB      │         │
   │  │                    alloc:        │               │               │               │         │
   │  │                      30          │ 30            │ 30            │ 30            │         │
   │  │                      2.689 KB    │ 2.689 KB      │ 2.689 KB      │ 2.689 KB      │         │
   │  │                    dealloc:      │               │               │               │         │
   │  │                      31          │ 31            │ 31            │ 31            │         │
   │  │                      2.921 KB    │ 2.921 KB      │ 2.921 KB      │ 2.921 KB      │         │
   │  │                    grow:         │               │               │               │         │
   │  │                      1           │ 1             │ 1             │ 1             │         │
   │  │                      224 B       │ 224 B         │ 224 B         │ 224 B         │         │
   │  │                    shrink:       │               │               │               │         │
   │  │                      1           │ 1             │ 1             │ 1             │         │
   │  │                      32 B        │ 32 B          │ 32 B          │ 32 B          │         │
   │  ├─ drizzle_prepared  4.98 µs       │ 13.14 µs      │ 5.106 µs      │ 5.338 µs      │ 100     │ 100
   │  │                    max alloc:    │               │               │               │         │
   │  │                      3           │ 3             │ 3             │ 3             │         │
   │  │                      81 B        │ 81 B          │ 81 B          │ 81 B          │         │
   │  │                    alloc:        │               │               │               │         │
   │  │                      6           │ 6             │ 6             │ 6             │         │
   │  │                      147 B       │ 147 B         │ 147 B         │ 147 B         │         │
   │  │                    dealloc:      │               │               │               │         │
   │  │                      12          │ 12            │ 12            │ 12            │         │
   │  │                      417 B       │ 417 B         │ 417 B         │ 417 B         │         │
   │  ╰─ raw               4.739 µs      │ 6.239 µs      │ 4.823 µs      │ 4.893 µs      │ 100     │ 100
   │                       max alloc:    │               │               │               │         │
   │                         1           │ 1             │ 1             │ 1             │         │
   │                         64 B        │ 64 B          │ 64 B          │ 64 B          │         │
   │                       alloc:        │               │               │               │         │
   │                         1           │ 1             │ 1             │ 1             │         │
   │                         64 B        │ 64 B          │ 64 B          │ 64 B          │         │
   │                       dealloc:      │               │               │               │         │
   │                         2           │ 2             │ 2             │ 2             │         │
   │                         104 B       │ 104 B         │ 104 B         │ 104 B         │         │
   ╰─ select                             │               │               │               │         │
      ├─ drizzle           22.18 µs      │ 31.18 µs      │ 22.43 µs      │ 22.69 µs      │ 100     │ 100
      │                    max alloc:    │               │               │               │         │
      │                      202         │ 202           │ 202           │ 202           │         │
      │                      9.744 KB    │ 9.744 KB      │ 9.744 KB      │ 9.744 KB      │         │
      │                    alloc:        │               │               │               │         │
      │                      205         │ 205           │ 205           │ 205           │         │
      │                      2.873 KB    │ 2.873 KB      │ 2.873 KB      │ 2.873 KB      │         │
      │                    dealloc:      │               │               │               │         │
      │                      206         │ 206           │ 206           │ 206           │         │
      │                      9.92 KB     │ 9.92 KB       │ 9.92 KB       │ 9.92 KB       │         │
      │                    grow:         │               │               │               │         │
      │                      8           │ 8             │ 8             │ 8             │         │
      │                      7.007 KB    │ 7.007 KB      │ 7.007 KB      │ 7.007 KB      │         │
      ├─ drizzle_prepared  21.44 µs      │ 152.9 µs      │ 25.26 µs      │ 26.85 µs      │ 100     │ 100
      │                    max alloc:    │               │               │               │         │
      │                      202         │ 202           │ 202           │ 202           │         │
      │                      9.713 KB    │ 9.713 KB      │ 9.713 KB      │ 9.713 KB      │         │
      │                    alloc:        │               │               │               │         │
      │                      203         │ 203           │ 203           │ 203           │         │
      │                      2.833 KB    │ 2.833 KB      │ 2.833 KB      │ 2.833 KB      │         │
      │                    dealloc:      │               │               │               │         │
      │                      206         │ 206           │ 206           │ 206           │         │
      │                      9.906 KB    │ 9.906 KB      │ 9.906 KB      │ 9.906 KB      │         │
      │                    grow:         │               │               │               │         │
      │                      5           │ 5             │ 5             │ 5             │         │
      │                      6.944 KB    │ 6.944 KB      │ 6.944 KB      │ 6.944 KB      │         │
      ╰─ raw               17.04 µs      │ 100.6 µs      │ 18.43 µs      │ 21.7 µs       │ 100     │ 100
                           max alloc:    │               │               │               │         │
                             201         │ 201           │ 201           │ 201           │         │
                             10.67 KB    │ 10.67 KB      │ 10.67 KB      │ 10.67 KB      │         │
                           alloc:        │               │               │               │         │
                             202         │ 202           │ 202           │ 202           │         │
                             2.8 KB      │ 2.8 KB        │ 2.8 KB        │ 2.8 KB        │         │
                           dealloc:      │               │               │               │         │
                             203         │ 203           │ 203           │ 203           │         │
                             10.77 KB    │ 10.77 KB      │ 10.77 KB      │ 10.77 KB      │         │
                           grow:         │               │               │               │         │
                             5           │ 5             │ 5             │ 5             │         │
                             7.936 KB    │ 7.936 KB      │ 7.936 KB      │ 7.936 KB      │         │

```
