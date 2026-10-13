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
