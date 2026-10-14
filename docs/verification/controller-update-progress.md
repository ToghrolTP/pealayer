# PCController updates in Pealayer's status bar

The existing PCController `update.*` and `peer-update.*` event contracts feed
one bounded operation inventory. Pealayer previously recognized only the first
family, so outbound peer-host progress was omitted even though the controller
published it. The existing read-only `controller.update.status` seeds a current
local operation at connection/catalog discovery; progress notifications do not
add UART reads, HTTP polls, writes or update authority.

The status bar distinguishes host updates (blue), board firmware/EEPROM/readback
operations (amber), and artifact transfers (teal). Operation kind, never display
text or byte counts, determines the source category. Native and Web interfaces
share these categories through `status_operations`; controller operations take
priority over unrelated downloads and the newest active operation is presented.
The fixed 168px widget reserves a percentage slot and truncates only its caption.
Hover/context details expose the current stage and all other active operations;
clicking controller progress opens Hardware Monitor.

Only a valid `progress_known` percentage is determinate. Preparation/restart or
health-check stages without measured progress are indeterminate. UTC event order
prevents delayed progress from resurrecting an operation after completion;
unchanged events do not invalidate the hardware snapshot or repaint it. A real
completion removes the active progress widget. Transport loss alone is not
completion; reconnect obtains the coordinator's authoritative operation status.
The user's visibility preference remains respected, and idle operations do not
add an empty widget. No updater or firmware operation is started by this feature.

Focused coverage checks peer-host progress/terminal ordering, invalid percentage
rejection, event-driven hardware revision invalidation without catalog polling,
source categories, newest-operation selection, fixed native widget geometry and
Web category/navigation behavior. RF regressions also cover the integration of
current main's typed-error work with the pending RF activity improvements.

Deployment remains gated on fresh draft-safe approval for the running Cafe
authoring instance and its exact host-specific libmpv runtime. Source/tests and
local package construction are not a live update-cycle or firmware-flash proof.
