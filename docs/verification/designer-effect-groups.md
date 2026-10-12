# Effects Designer folder navigation

Owner: issue #80.

The native left-hand list previously rendered audio, sequence and strip effects
as three flat loops. Category names existed only as repeated card subtitles;
there was no folder disclosure or count.

The list now groups actual saved effects by their stored folder/category,
independently of execution mode or effect kind. Controller-declared folder order
and icons are retained; non-declared used folders follow in catalog order.
Empty folders are omitted from this navigation view (folder management remains
in Effects Library). Blank identity renders as localized Ungrouped without
colliding with an operator's literal folder named Ungrouped. Case-sensitive names
remain separate. Counts cover all member kinds, not just the sequence loop.

Each divider has a start-aligned folder caption, a trailing count and full-row
mouse/keyboard disclosure. Stable IDs use the raw folder name, never count or
translated text. Body transitions use the application's egui animation policy.
Headers/cards share the existing solid-sidebar scrollbar lane; the scrollbar
does not overlay content. The redundant folder subtitle is removed from cards.

Changing selection or previewing a moved existing effect reveals its destination
once. Repainting/count updates do not reopen a manually collapsed folder.
Collapse does not select an effect, modify the draft or send a hardware command.
Discard restores the previous folder preview. New unsaved effects do not inflate
saved-item counts. Web library grouping also follows an existing draft's folder
preview without modifying its saved catalog.

## Verification and delivery boundary

Focused tests cover mixed-kind membership, authoritative order/icons, exact
counts, empty/blank/case-sensitive folders, selection reveal, mouse/Enter
disclosure and long/RTL names at narrow widths in both densities/themes and
1/1.25/1.5/2 display scales. The native suite passed 831 tests with one
pre-existing ignored test (832 total). Web build, 50 parity contracts and
installable/offline PWA checks passed.

The running Pealayer has not been replaced while unsaved authoring restart
readiness is unanswered. Cafe's GUI/smoke-launch denial remains a deployment
blocker, and the paired contextual Stop update still needs the matching primary
controller runtime. Source/render tests are not live visual acceptance. No
production window input, physical board actuation, new worktree or cache purge
was used for this change.
