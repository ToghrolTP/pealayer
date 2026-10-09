# Preferences organization and application icons

Rust supplies one ordered list of groups and controls to the native editor and
the Web application. Non-contiguous controls in the same group produce one card,
not repeated cards. Identical help text is retained only once within a group.
Distinct, useful descriptions and all existing settings remain available.

Each control carries a semantic icon name in the shared contract. Theme,
palette, language, fullscreen background, window pinning and OSD placement no
longer inherit the same list icon merely because they use dropdowns. Language
follows Fullscreen background at the end of Interface.

Single-line native inputs share `dialog::singleline_text_edit`, centering both
text and placeholders vertically when allocated extra row height. Multiline
and wrapped editors retain top alignment.

Appearance ends with **Application icons**, a single optional disclosure. With
no configured overrides it initially stays collapsed. Configured overrides make
it initially open; the user can still collapse or expand it. It contains:

- Default icon, falling back to the bundled application icon.
- Playing, Paused and Stopped overrides, falling back to the default icon.

Each field has Browse and a contextual Reset action. PNG, JPEG, WebP and ICO are
supported by the existing image loader. Reset clears the override rather than
writing a bundled-image path into user configuration. Native Browse uses the OS
file chooser. Web Browse and connected-peer Browse use the server's filesystem,
not the browser/consumer's local files, and reject directory selections.

Advanced ends with **Config file**, including automatic reload and the existing
path, context menu, Import, Export and Reload operations. Native-only File
associations comes immediately before Config file, not after it.

Interface also exposes **Application icon**: Current or Classic (previous).
Both are bundled; the exact previous artwork is preserved under `assets/icons`.
Custom icons remain in their existing optional section and take precedence over
the bundled choice. Native/window and Web/PWA resolution share `src/branding.rs`;
the Web status revision refreshes imagery when settings change, without waiting
for a play/pause transition. See [bundled icon provenance](../assets/icons/README.md).

Release builds serve their embedded Web assets unless `PEALAYER_WEB_ROOT` is
explicitly configured. This prevents stale adjacent assets from obscuring new
controls after an executable-only peer update. Development asset overrides are
unchanged.

## Verification and visual acceptance

Focused Rust checks cover card uniqueness/order, help uniqueness, optional
disclosure rendering, file metadata and supported non-directory selection.
Web checks cover type checking, server-owned Browse, cancellation/stale-list
handling, file extensions, Windows/Unix parent paths, shrinkable layout and PWA
packaging. These checks do not substitute for screenshots.

After deploying the exact build, verify:

1. Appearance with no overrides: one collapsed Application icons section.
2. Expanded section: aligned fields, Browse and Reset; no repeated paragraphs.
3. Browse an image, preview/save it, reopen; Reset restores the existing fallback.
4. Advanced scrolled to its bottom: Config file is the final card.
5. Web Preferences at phone, tablet and desktop widths in light and dark themes.

The user authorized Win32/built-in screenshot capture on 2026-10-08. Reuse
`scripts/update-screenshots-windows.ps1` with an isolated profile, an explicit
Preferences tab and optional image override. Do not change production settings
just to stage a screenshot. Record build identity and inspected screenshot
results separately from tests; a capture alone is not an installed-build claim.
