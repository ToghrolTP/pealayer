# Header endpoint selector bounds

The controller endpoint selector occupies a 154-point slot to the left of the
connection action in the right-anchored native header. `ComboBox::width(154)` is
only a minimum in egui; the default extending text layout can grow the control
for a full `pccontroller://` endpoint. Commit `2bf892e4` introduced the centered
left-to-right child inside that slot without truncating the selected caption.
The growing child then painted its chevron into the Disconnect side of the row.

The header now uses one bounded endpoint-trigger helper with explicit caption
truncation. Text, chevron and click targets remain within the closed 154-point
slot. The shared open-chevron painter clips to its own icon rectangle. Do not
clip the entire child UI: that would constrain the popup too. The endpoint popup
has a separate 240-point minimum for its custom input and complete choice rows.
Full endpoint values and selection identity are unchanged; connection/disconnect
semantics and their safety gates are unchanged.

Pure egui regressions exercise active/disabled and normal/hover states across all
four control densities, 12/14/20/24-point fonts and 1/1.25/2 display scales. They
check exact trigger width, disjoint Disconnect/selector hit regions and chevron
paint bounds. A real open-popup fixture checks that wide popup controls retain
their independent clip region. No fixture starts a hardware engine, connection
or actuator action.

Native source/geometry checks do not replace live screenshot acceptance. Cafe
deployment must use its validated libmpv runtime and wait for draft-safe approval.
