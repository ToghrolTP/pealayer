# Responsive Web UI

## Root cause

The phone breakpoint stacked the Ant Design sidebar and content vertically.
Ant Design's `.ant-layout.ant-layout-has-sider > .ant-layout-content` still set
`width: 0`, however. In a column, flex-grow grows height, not width. The content
shrank to its padding: measured at 16 px on a 390 px viewport. Clipping the page
hid almost every control. The old bottom navigation also overflowed its height.

The mobile override explicitly outranks that library selector. Content retains
the available width, independent vertical scrolling, and safe-area padding.
All seven navigation targets fit across the bottom at widths down to 320 px.

## Related fixes

- Grid children and video previews shrink without intrinsic image/title widths
  stretching their columns. Phone previews retain their aspect ratio.
- Phone timeline order: preview, cues, then effects. The ruler and lanes share a
  minimum width and scroll **inside** the timeline, rather than squeezing labels
  together or overflowing the page.
- PWM sliders occupy their own full-width row on phones; hardware header actions
  wrap, with no controls removed.
- Channel-manager rows wrap live controls and ordering into a second row.
- Preference sections use a horizontal, named, scrollable rail without assuming
  there will always be five sections. Labels and controls stack on phones.
- About tables wrap values; long library breadcrumbs wrap, while the file table
  has its own horizontal scroller.
- Dialog bodies scroll and their footer actions remain accessible on phone and
  short landscape screens. Remote file information switches to one column.
- Existing palettes, shared runtime/config state, WebSocket commands, hardware
  capabilities, user effects and desktop navigation are retained.

## Verification

`npm run build` runs 15 inexpensive source-layout guardrails, the existing small
Web checks, TypeScript compilation, bundling and installable/offline PWA checks.
These guardrails are not a substitute for browser testing.

Browser verification uses the actual Rust backend, not sample data. Check all
seven surfaces at 320, 390, 560, 561, 600, 768, 820, 821, 1024 and 1280 px;
compare light/dark themes, check viewport and content widths, scroll long pages,
and open editors in both portrait and landscape. Internal table/timeline
scrolling is intentional. Page-level horizontal overflow is not.

For development, Vite binds to loopback and forwards `/api` and `/ws` to the
running local backend on port 8080. Final acceptance must also use the embedded
production bundle after the application has installed its own verified update.

Real Android/iOS touch-device testing is an additional acceptance gate; viewport
emulation does not prove browser-specific touch gestures or safe-area behavior.
