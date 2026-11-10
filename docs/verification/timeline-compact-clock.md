# Compact timeline clock

Build: `58d528f3b89a9c0a29f7bff67a8a3be1c062a583`.

Timeline rulers display compact clock times, omitting only leading zero units:
`0`, `5`, `1:05`, `1:00:00`, `1:02:03`. Interior zero units remain, so one hour
cannot be mistaken for one second. Millisecond precision remains available for
fine-grained effect-editor ticks, keyframe times and recorded steps. Storage and
playback timing are unchanged.

Verification:

- Targeted Rust formatter test passed.
- Web formatting checks and production build passed.
- Windows release package completed, including libmpv smoke and resource checks.
- Canonical local executable launched from the clean build above.
- Live Web timeline ruler displayed `0`, `35:38`, `1:11:16`, `1:46:55`, `2:22:33`.
- Native ruler compiled; native visual capture was not verified in this pass.
- Cafe-PC updater health endpoint timed out; no remote deployment is claimed.

![Live Web timeline ruler](timeline-compact-clock.jpg)

The image is cropped to the ruler to exclude private media details.
