# Program Monitor viewport

The native monitor is a fitted video surface and a width-responsive transport
footer, not a scrolling document. Its video-height budget includes both egui's
automatic vertical item spacing and the explicit video/footer gap. Previously
only the explicit gap was reserved, producing constant overflow independent of
the panel height.

The dock does not enable document scrolling for this panel. egui pixel-snaps a
ScrollArea's content rectangle separately from its logical viewport, so even a
correctly fitted surface can otherwise acquire fractional scroll travel at
non-integer display scales. Other panels keep their existing scrolling policies.

The native regression renders the production tab and controls with and without
media at four sizes (including one- through four-row footers) and 100%, 125%,
150% and 200% scaling. It checks bounds against the original available rectangle,
allows only the framework's physical-pixel rounding, and requires zero dock
scroll offset. Existing footer tests retain track/volume order and bounded rows.

This is native layout verification, not physical-board, human playback or soak
acceptance. Cafe source alignment is separate from runtime delivery: the recorded
Cafe smoke-launch denial remains a deployment blocker and must not be bypassed.
