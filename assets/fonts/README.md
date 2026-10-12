# UI font assets

Vazirmatn-Regular is the existing Persian fallback. Ubuntu-Light and
Hack-Regular are the same regular faces bundled by epaint_default_fonts 0.36.2,
reused for browser/native preference parity, not separately downloaded fonts.
Ubuntu uses the adjacent Ubuntu-LICENSE.txt (Ubuntu Font Licence); Hack uses
Hack-LICENSE.txt (MIT and Bitstream Vera licence notices).

System UI uses the platform face; System monospace prefers Consolas on Windows,
Menlo on macOS and DejaVu Sans Mono on Linux, retaining bundled fallback when
the OS font is unavailable. The browser likewise retains offline bundled fallbacks.
