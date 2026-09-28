# Development, testing, and screenshots

## Core gates

```bash
cargo check --all-targets
cargo test --all-targets --locked
```

Windows packaging additionally runs the libmpv smoke test and validates the
native resources/package manifest. Run PCController integration tests with an
actual reachable controller; do not replace absent hardware with production
fixtures.

## Visual acceptance

Use the canonical scripts and workflow in
[`docs/SCREENSHOTS.md`](https://github.com/ToghrolTP/pealayer/blob/main/docs/SCREENSHOTS.md).
The manifest records source head, executable hash, image hashes, dimensions,
theme, locale, and direction. English/Persian, light/dark, narrow layout,
keyboard/focus, loading, empty, pending, failed, recovered, E-STOP, drag/drop,
and real connected-hardware states are acceptance surfaces.

Test fixtures may be used only in explicit test builds and must never populate
production device, effect, relay, macro, or telemetry lists.

## Review bookkeeping

Every human or automated review finding needs a durable finding → commit → test
entry on the owning pull request before merge.
