# Rust dependency license review

This inventory records every direct and transitive third-party package in the
committed `Cargo.lock` used for Git Pin V1. The project itself remains licensed
under the MIT License in `LICENSE`.

The only direct dependency is the target-scoped `windows` crate. All remaining
entries are its transitive dependencies. No development-only dependency, Git
dependency, alternate registry, or wildcard version is present.

| Package | Version | Relationship | Upstream SPDX license |
| --- | --- | --- | --- |
| `windows` | 0.58.0 | direct, Windows target only | MIT OR Apache-2.0 |
| `windows-core` | 0.58.0 | transitive | MIT OR Apache-2.0 |
| `windows-implement` | 0.58.0 | transitive | MIT OR Apache-2.0 |
| `windows-interface` | 0.58.0 | transitive | MIT OR Apache-2.0 |
| `windows-result` | 0.2.0 | transitive | MIT OR Apache-2.0 |
| `windows-strings` | 0.1.0 | transitive | MIT OR Apache-2.0 |
| `windows-targets` | 0.52.6 | transitive | MIT OR Apache-2.0 |
| `windows_aarch64_gnullvm` | 0.52.6 | transitive target support | MIT OR Apache-2.0 |
| `windows_aarch64_msvc` | 0.52.6 | transitive target support | MIT OR Apache-2.0 |
| `windows_i686_gnu` | 0.52.6 | transitive target support | MIT OR Apache-2.0 |
| `windows_i686_gnullvm` | 0.52.6 | transitive target support | MIT OR Apache-2.0 |
| `windows_i686_msvc` | 0.52.6 | transitive target support | MIT OR Apache-2.0 |
| `windows_x86_64_gnu` | 0.52.6 | transitive target support | MIT OR Apache-2.0 |
| `windows_x86_64_gnullvm` | 0.52.6 | transitive target support | MIT OR Apache-2.0 |
| `windows_x86_64_msvc` | 0.52.6 | transitive target support | MIT OR Apache-2.0 |
| `proc-macro2` | 1.0.107 | transitive build/proc-macro support | MIT OR Apache-2.0 |
| `quote` | 1.0.47 | transitive build/proc-macro support | MIT OR Apache-2.0 |
| `syn` | 2.0.119 | transitive build/proc-macro support | MIT OR Apache-2.0 |
| `unicode-ident` | 1.0.24 | transitive build/proc-macro support | (MIT OR Apache-2.0) AND Unicode-3.0 |

The authoritative automated policy is `deny.toml`. CI installs exactly
`cargo-deny` 0.20.2 with its locked dependency graph and runs advisory,
license, ban, and source checks. The policy has no advisory or license
exceptions: a future exception requires an explicit, reviewed repository
change with its rationale recorded next to the exception.
