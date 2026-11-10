# Windows readiness source review

The [original source review](https://github.com/ToghrolTP/pealayer/commit/ebd6bd2fac91155cf153ec024a1cf0593ff84638) is reachable from `restored/windows-readiness-review`. Its audit note and source changes remain available under the `archive/windows-readiness-review` tag.

At [integration commit `3b9670c`](https://github.com/ToghrolTP/pealayer/commit/3b9670cd2dc0d358710a786fa09a75626287ddf7), `src/` and `build.rs` match the source review exactly. That integration also unified the branding and package format strings, added Web localization and moved the Web build from Yarn to npm.

Current `main` retains the isolated Windows screenshot profile and the tests for an empty production timeline and unknown playback state during startup. [Windows readiness #30](https://github.com/ToghrolTP/pealayer/issues/30) and [Win32 integration #34](https://github.com/ToghrolTP/pealayer/issues/34) track packaged runtime verification.
