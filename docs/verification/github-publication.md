# Screenshot publication and merge completion

## Publish evidence, not placeholders

1. Capture the actual relevant native build through the maintained read-only
   capture utility or an application export. Inspect the image itself for blank,
   stale, clipped or unrelated content. Exclude private media, settings and shell
   transcripts. Keep matched before/after palette, scale and viewport where possible.
2. Commit approved PNGs under `docs/screenshots`, together with provenance and
   honest acceptance limits under `docs/verification`. Push and verify the exact
   remote ref. Local files and an unpushed commit are not published evidence.
3. Embed images directly in the owning PR description using full immutable
   `raw.githubusercontent.com/<owner>/<repo>/<commit>/...` URLs. Link the complete
   gallery too. A future branch deletion must not break the comparison.
4. Check that GitHub's rendered Markdown contains each expected image and that
   each public URL responds successfully as an image. Compare the repository blob
   identity with the inspected local asset. A Markdown link alone is insufficient.
5. Update outdated claims such as "screenshots pending" after publication. For
   historical requests, retain the actual chronological builds and distinguish
   layout changes from conditional states; never fabricate missing telemetry.

## Finish the authorized merge

1. Refresh PR head, base, changed files, comments, unresolved review threads and
   all checks on the current head. Superseded checks are not current evidence.
2. Resolve real failures and conflicts without discarding concurrent work or
   restoring obsolete behavior merely to satisfy an outdated assertion. Test the
   intended contract, including optional panes remaining hidden by default.
3. Wait for required/current checks and inspect their results. Do not use admin
   bypass, silently skip failures, or report queued auto-merge as merged.
4. Merge with an expected-head guard, preserving authorship/history as appropriate.
   Independently read back `MERGED`, the merge commit and its presence in the
   target branch. Push useful remaining source and receipt changes before handoff.
5. Keep explicit user-held PRs and genuinely unfinished drafts open, with a precise
   gate and owning issue. A green source build does not establish physical, remote
   deployment or human acceptance. Record an unavailable production host explicitly.

## Working network path

Use command-scoped proxy/authentication settings, never global account or proxy
changes. The approved office SSH route and its existing HTTP proxy provided a
working publication path when the local proxy's upstream TLS stalled. Connection
addresses and session ownership belong in private operational notes, not PR prose.
Verify that route afresh; close only the temporary forward owned by this pass.
Never print tokens, change remote-access services or bypass an execution denial.
