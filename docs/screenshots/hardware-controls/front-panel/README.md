# Live front-panel verification

These captures show Pealayer's native Board Information window rendering the
exact four-byte segment frame reported by the physical PCController board on
CAFE-PC. The second capture was taken after invoking K2 through Pealayer's own
JSON-RPC surface.

| Step | PCController exact read | Pealayer exact read |
| --- | --- | --- |
| Before K2 | page `1`, frame `06 DB 5B 4F` | page `1`, frame `06 DB 5B 4F` |
| After K2 | page `2`, frame `00 BF 06 7D` | page `2`, frame `00 BF 06 7D` |

The key command was sent to `pealayer.hardware.front_panel.press` with
`{"key":"K2"}`. Pealayer routed it to the ordinary physical-board menu because
`host_captured` was false, then performed a fresh `controller.front_panel` read
before updating the native preview.
