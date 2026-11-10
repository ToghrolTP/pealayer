# Web filesystem locations

The media library and server-owned Preferences file picker now share a searchable
Locations control. The old browser exposed only the current directory and its
ancestors, so it could not switch drive roots. Its breadcrumb parser additionally
treated UNC shares as Unix paths and collapsed their leading backslashes.

`GET /api/fs/locations` returns bounded, deduplicated records with `name`, `path`
and semantic `kind`. The existing host-file-access permission protects this route;
consumer instances relay it to their selected authority, not their own filesystem.

On Windows the read-only catalogue includes:

- Drive roots visible to the running account, including mapped drives and
  removable/optical roots. No capacity/volume probe is performed.
- Remembered mappings in that account's `HKCU\Network` whose drive letter is not
  currently present, offered by their stored UNC target without claiming online
  availability. No credentials or other registry values are read.
- Windows-resolved Home, Desktop, Documents, Downloads, Videos, Music and Pictures,
  including redirected/custom known-folder paths.
- Stored folder shortcuts in Links/Desktop and Windows Network Shortcuts
  (`NetHood`, including nested `target.lnk` entries). Links are read through COM,
  never executed or interactively resolved. Remote shortcut source directories
  are skipped rather than probed while listing locations.
- The current folder and recent local media's parent folders. HTTP media URLs are
  not converted into invented filesystem locations.

The library also accepts an explicit folder/UNC path. Breadcrumbs preserve drive,
UNC share and extended Windows namespace roots; navigation never synthesizes an
invalid `/server/share` path. Failed/offline browsing retains the last good list
and reports the real error. Newer requests cancel/ignore stale responses. Location
refresh is explicit, not polling, recursive discovery, remapping or LAN scanning.

Mappings are scoped to the Pealayer account/logon. This cannot make a mapping
owned by a different account magically accessible, mount an offline share, or
browse virtual Shell objects lacking a filesystem path. Modern Explorer Quick
Access automatic-destination files are not parsed. Existing permissions and OS
access checks remain authoritative.

Implementation references: Microsoft's [logical drive API](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getlogicaldrivestringsw)
and [Shell link interfaces](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-ishelllinkw).

Windows offline compiler check and Web TypeScript compilation passed with existing
native warnings. Added pure breadcrumb/location-validation, namespace/dedup and
file-access regressions; tests/CI are skipped under the user's current instruction.
Package/deployment and real mapped-share access are separate verification gates.

## Cafe delivery, 13 October 2026

PR [#175](https://github.com/ToghrolTP/pealayer/pull/175) merged normally at
`8cc4b9e8a348dc29b6a3b01b95adb7f9c6072d18`. One shared-cache Windows release
package from clean source `9c9845c91908e83c027c6cddc5e1bb0a49d116f4` passed
Web/PWA integrity, native resources and retained-runtime smoke probes. Native
release compilation took 1m46s; downloader compilation took 6.60s. No destination
compiler, test suite or CI run was invoked.

Cafe's approved updater completed `update-ee4d3ff5-6a35-40e7-85c8-b81ccbce0efd`.
Fresh process-local inspection verified PID 43416/session 4 and player SHA-256
`4cbb7ff3e83edbb692ecd5cb82a9be8aa9796495476cb0e4fff2836bb74a2333`,
with Cafe's retained libmpv
`872827614ed0adfca11e68def5273bcfcaea6acf38bbf1950c35980b59f43a5f`.
The private program-root `verification/web-file-locations` backup remains intact.
Effect draft, workspace session/layout/profiles, active profile and branding
matched their pre-update values. Playback remained paused with hardware connected,
no sync error and matching desired/prepared/clock ACK 84.

Live read-only API checks returned one drive, one network and one removable root,
eight folders, two shortcuts and one recent folder. Drive-root browsing succeeded
with 20 entries and no parent. The returned network location also successfully
browsed with 15 entries and no parent. Private share names/contents are not
published. The embedded `FileLocationSelect.js` returned HTTP 200. These are live
host/catalog/network-listing results, not a claim of every share being accessible
or of completed phone/tablet gesture/visual acceptance. Local/Erfan latest native
activation restrictions were not bypassed for this pass.
