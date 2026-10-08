# Preferences organization and application icons

Rust supplies one ordered list of groups and controls to the native editor and
the Web application. Non-contiguous controls in the same group produce one card,
not repeated cards. Identical help text is retained only once within a group.
Distinct, useful descriptions and all existing settings remain available.

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

Ask the user for full-window screenshots for visual acceptance; this task's user
has requested human interaction rather than computer-control automation. Record
installed build identity and actual screenshot results separately from tests.
