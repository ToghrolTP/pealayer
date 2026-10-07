# Preferences Save closes after success

- The regular footer Save previously requested persistence but did not set `close_after_save`; Save in the unsaved-changes prompt already did. Both now call one shared `request_save` path.
- Embedded Preferences closes only after validation, disk save and runtime commit succeed. The standalone native host closes only after disk save and owner-commit delivery succeed. Existing failure handling keeps the dialog open and preserves the error; an invalid config retains its dirty draft.
- Save stays disabled when there are no changes. Close/Discard/Cancel and live preference preview behavior are unchanged.
- All 15 focused Preferences tests passed, including close intent from both Save entry points and failed-save preservation. Native Windows release packaging passed with `-SkipTests -NoUpx`; no full Rust suite was run. Native mouse/visual verification is not claimed.
- Source commit `6b50ae987d316b38dc62a5023572b9f8029e3d70` pushed to PR #46; unmerged.
- Graceful `/api/ipc` Quit preceded replacement. Canonical `%LOCALAPPDATA%\Programs\Pealayer\bin\pealayer.exe` relaunched, PID 39148, health `ok`, manifest exact source commit and `git_dirty: false`.
- SHA256 `d7f553251132ae0599924a1af5312b3e64034927ba7d9a6b8ea53f1a2f471d29`, 36,405,760 bytes.
- Cafe-PC updater endpoint timed out after five seconds; no remote deployment or manual SSH substitution is claimed.
