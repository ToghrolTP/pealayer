# 🎬 Pealayer vX.Y.Z — Milestone title

Describe what users can do in this version in one short paragraph. Choose a
milestone emoji and use it consistently in the release title, version index,
changelog and `history.json` entry's `emoji` field. Keep the original 🌱 and 🎬
identities for v0.1.0 and v0.2.0.

📦 [Downloads](https://github.com/ToghrolTP/pealayer/releases) ·
📚 [Version history](https://github.com/ToghrolTP/pealayer/blob/main/docs/releases/README.md) ·
📝 [Changelog](https://github.com/ToghrolTP/pealayer/blob/main/CHANGELOG.md)

**🗓️ Source checkpoint:** date · exact source link · comparison with the previous version.

## ✨ Added and implemented

- **[Added]** Describe an implemented capability and link its owning PR/issue.

## 🛠️ Fixes and improvements

- **[Fixed]** State the actual before/after behavior and owning PR.

## 🧪 Experimental features and later follow-ups

- **[Experimental]** Explain what is incomplete and link later work separately.

## 🐞 Bugs and problems in this build

- **[Known issue]** Describe the affected workflow and available workaround;
  distinguish problems in this version from later fixes and current acceptance.

## 📦 Packages and verification

Link the exact 🐧 Linux, 🪟 Windows and 🍎 macOS packages that exist. Report
unavailable platforms and actual CI results. Include verification instructions
appropriate to those artifacts; never claim an attestation or runtime acceptance
that was not established.

Synchronize the published body and attached `RELEASE-NOTES.md`. Recompute its
entry in `SHA256SUMS.txt` whenever the notes change. Preserve native package
bytes, source tags and historical publication dates during presentation edits.

## 📸 Screenshots

Embed inspected, genuine images at immutable URLs. Caption the source build,
surface and acceptance limits; link capture provenance. An intermediate build
must be identified, and later UI must not be attributed to an older version.

## 🔗 Pull requests and issues

List all merged PRs first included in this source range and related issues.
Keep existing machine-readable release/comment markers and tagged entries.
Include readable previous/next version navigation.

Before publication, replace all placeholders, update the version index and
changelog, run `node scripts/check-release-history.mjs --git`, and inspect the
GitHub-rendered notes. Preserve these decorated headings when generating or
editing future releases.
