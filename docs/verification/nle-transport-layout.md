# NLE transport: measured reflow and working overflow

The earlier two-row layout kept transport beside a compressed seekbar, then put
selectors and volume on a trailing second row. Smaller monitors fell through to
four rows, and wide custom action sets silently scrolled horizontally.

The revised layout measures the current action configuration and control density:

| Available panel space | Layout |
| --- | --- |
| Wide | One row: leading transport, elastic seekbar/time, trailing selectors and volume |
| Medium | Dedicated full-width seekbar/time row; transport left, selectors/volume right below |
| Narrow | Seekbar/time row, balanced action row with real overflow, trailing volume row |

Breakpoints come from measured group widths, not screen/device names. The video
reserves exactly the chosen footer height. RTL mirrors the clusters, and changes
in row count retain widget/drag identities and user action ordering/visibility.
Buttons that do not fit a narrow monitor group are available as real commands in
its trailing overflow menu, not clipped or stranded in an invisible scroller.

The first real native narrow capture also exposed an allocator shrinking the
action lane to its visible prefix, leaving the overflow button short of the pane
edge. The lane now reserves its exact measured bounds. A rendered trailing-edge
assertion guards it across densities and RTL, and medium rows reserve a distinct
inter-cluster gutter rather than packing transport against the track selectors.

## Three-dot failure and regression

The left-click `Popup::menu` and secondary-click `Response::context_menu` both used
the response's default popup ID. The secondary handler closed the primary popup
on the same click. A rendered-pointer regression reproduced this failure before
the fix (`left-click overflow menu disappeared on Monitor`). Primary and secondary
menus now have independent identities on Simple and NLE surfaces, matching the
existing media-track selector fix. Menu painting never invokes transport.

Focused tests cover real menu clicks after reflow, use of an omitted command,
custom action sets, exact-fit boundaries, bounded footer geometry and RTL. Live
native captures and installed-runtime receipts are recorded in the owning PR;
tests alone do not establish visual or playing-video acceptance.

The earlier isolated review was closed normally after the user closed its
Preferences window and a fresh authoring read reported clean. Regular settings
and hardware were not used as fixtures. Cafe deployment remains user-deferred
until its normal connection returns.
