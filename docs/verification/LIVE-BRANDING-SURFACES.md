# Live application branding

Custom names, Current/Classic presets, and custom playback-state icons resolve
through the existing Rust branding/configuration contract. Changing a caption
does not rename Pealayer's protocol methods, legal attribution, or stable IDs.

## Corrections

| Surface | Live behavior |
| --- | --- |
| Main window, taskbar and tray | Native viewport icons update on state/artwork changes. The tray copies the HWND icon on `WM_SETICON`; its tooltip now refreshes on title-only config changes too. Explorer recreation retains the existing re-registration path. |
| About | Uses the configured artwork rather than an embedded, permanently cached default logo. Its header and overview use the configured name; upstream license attribution stays intact. |
| Preferences | The separately hosted window uses the configured icon and updates its title/icon while previewing settings, including Discard restoration. Its gear remains a semantic navigation/header icon. |
| Native media session | Configured display name and album replace the fixed product caption. A name change drops the old registration before creating its replacement, including MPRIS Identity. |
| Browser | Runtime metadata is read from live Rust config. Existing status revisions invalidate the runtime fetch; unchanged results retain their React identity. Branding changes do not reconnect the control WebSocket. |
| PWA/mobile | Manifest names and icon URLs reflect live config; favicon, touch icon, Apple app title and media-session branding refresh together. Browser-managed installed names/icons may still require the browser's manifest refresh or reinstall. |
| Configuration folder | Explorer's InfoTip and a single multi-resolution native ICO reflect saved branding. Unchanged bytes are not rewritten; a Shell change notification invalidates Explorer's view. Remote consumers still do not write authority settings locally. |

Window artwork is not decoded every frame, or on unrelated volume/config
changes. The source signature includes the resolved file's size/modification
time; replacing artwork at the same path is detected on the next configuration
application/reload. This is not an artwork-file watcher.

Executable resources, Explorer file icons, pinned shortcuts and installer names
belong to packaging/OS caches, not the live viewport. Existing branded packaging
must patch native multi-resolution ICO resources **before signing**. This fix
does not mutate or re-sign the running executable, or replace semantic action
icons in Jump Lists with an application logo.

## Verification checkpoint

- Web TypeScript and production/PWA build passed on David-PC.
- Focused Rust regression checks cover source-signature invalidation, native
  ICO shape, configured Preferences icon and live runtime metadata.
- Test suites remain deferred at the user's request; adding checks is not a
  claim that they were executed.
- Locked incremental Windows release build passed in 1 minute 30 seconds on
  David-PC. Seven semantic quick-action resources and UPX integrity passed.
- David and Cafe staging independently passed runtime smoke with their own
  retained DLLs. Cafe was updated first through the verified graceful peer
  updater; only after its loaded, paused media and connected board were healthy
  was David updated. Both canonical executables match the installed package.
- David remains Cafe's connected, cache-only consumer without a second local
  hardware scheduler. Its paused decoded/server positions agree and both
  processes remain responsive in the interactive desktop session.
- Erfan received an atomic offline installation with a verified rollback and
  unchanged destination DLL. KMPlayer remains running; Pealayer was not launched.
- A temporary API-only preview changed the title and selected Classic artwork.
  Rust runtime metadata and manifests on both endpoints reported the preview
  name; David's actual Win32 title and borrowed HWND icon bitmap changed too.
  Cancel restored the original settings, native title and exact original icon
  bitmap after asynchronous authority-to-consumer propagation. No preview was
  saved, hardware commands were not issued, and private artwork was not uploaded.
- Explorer configuration metadata and its native ICO exist on Cafe. The actual
  tray hover, About/Preferences interactions, and browser-installed app cache
  behavior still need human visual acceptance; a question was sent to the user.

Delivered runtime: [branding implementation](https://github.com/ToghrolTP/pealayer/commit/5059b05eff7b327ef5eb6641727c498e4f2adaea).
Review/checkpoint: [PR #99](https://github.com/ToghrolTP/pealayer/pull/99).
