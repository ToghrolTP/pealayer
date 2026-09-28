# Configuration, branding, and localization

Configuration is stored as JSON in the platform user configuration directory,
or under `config/settings.json` in portable mode.

| Environment value | Accepted values | Purpose |
|---|---|---|
| `APP_NAME` | non-empty text | Product/window identity |
| `APP_ICON` | PNG path | Runtime window icon |
| `APP_THEME` | `system`, `light`, `dark` | Application and Windows caption theme |
| `APP_LOCALE` | `system`, `en`, `fa` | Application-owned text |
| `APP_DIRECTION` | `auto`, `ltr`, `rtl` | Widget order and alignment |
| `APP_PUBLISHER` | optional text | About dialog and Win32 company identity |
| `APP_COPYRIGHT` | optional text | About dialog and Win32 legal identity |
| `APP_DESCRIPTION` | optional text | Win32 file description |
| `APP_ICON_ICO` | ICO path | Build-time Windows executable icon |
| `APP_EXECUTABLE_NAME` | safe file name | Packaged executable and Win32 identity |
| `APPLICATION_BRAND` | JSON path | Shared living application-brand document |
| `PEALAYER_CONFIG_FILE` | JSON path | Explicit isolated/portable settings file |

The appearance values intentionally match PCController WebUI. Persian selects
RTL automatically and uses the system UI font first with bundled Vazirmatn as a
release-safe fallback. Language and direction can also be changed at runtime
from the Help menu.

Branding never changes protocol identities or PCController-advertised hardware
names. The shared branding document uses additive fields and tolerant readers;
it is a living contract rather than a numbered schema. `applicationName`,
`companyName`, `fileDescription`, `legalCopyright`, `executableName`, and the
`APP` entry under `windowsIcons` are consumed by Pealayer. Focused `APP_*`
values intentionally override those fields for a product-specific build.

Build and validate a branded Windows package with:

```powershell
pwsh -File scripts/package-windows.ps1 -Branding path\to\brand.json
```

The package manifest records the effective PE identity and artifact hashes.
The same document is consumed by PCController's
[branding work](https://github.com/atomicdeploy/PCController/pull/380).
