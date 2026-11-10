# UI font assets

Vazirmatn-Regular is the existing Persian fallback. Ubuntu-Light and
Hack-Regular are the same regular faces bundled by epaint_default_fonts 0.36.2,
reused for browser/native preference parity, not separately downloaded fonts.
Ubuntu uses the adjacent Ubuntu-LICENSE.txt (Ubuntu Font Licence); Hack uses
Hack-LICENSE.txt (MIT and Bitstream Vera licence notices).

InterVariable.ttf is the unmodified upstream Inter asset at rsms/inter commit
`353b61b9f4430d5f420d56605a6e7993e0941470`, path `docs/font-files/InterVariable.ttf`.
SHA-256: `4989b125924991b90d05b2d16e0e388c48f7d5bb8b30539bbf9c755278d0ccaf`.
Its SIL OFL 1.1 notice is Inter-LICENSE.txt, included in About and Windows packages.
Inter is the Windows status-bar default, not a replacement global UI theme.
macOS status uses its CoreText system face, Linux installed Roboto then the DE's
fontconfig choice; explicit custom UI faces override these defaults. Persian
fallback and Phosphor coverage remain intact. The browser bundles the same Inter
asset and uses actual CSS tabular figures.

System UI otherwise retains the platform face. Genuine fixed-pitch text keeps
the configured monospace family: Windows prefers installed Segoe mono variants,
Cascadia Mono then Consolas; other platforms retain native/DE choices and bundled
fallback. See `docs/verification/status-tabular-typography.md` for the usage audit.
