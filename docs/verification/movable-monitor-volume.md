# Movable Program Monitor volume and focused defaults

Owner: [issue #80](https://github.com/ToghrolTP/pealayer/issues/80).

## Correction

The volume slider was painted in a separate fixed footer slot, outside the
shared customizable action bar. Moving the speaker moved only Mute, not its
slider. That also forced volume to remain after Fullscreen regardless of saved
action ordering.

Program Monitor now renders speaker, slider and percentage as one group owned
by the existing saved Mute action. Drag the speaker to reorder or move the
group between leading/trailing controls; its context menu calls it Volume.
Dragging or wheeling the slider still changes volume, not the workspace order.
The same current action identities, indexed flags, commands and narrow CAS
preference saves remain in use: no new layout format or migration is needed.

The native responsive footer measures this group as part of its chosen side.
When narrow it places leading and trailing groups on separate rows, retains a
full-width seeking row, and exposes genuinely omitted controls through overflow.
An overflowed volume group has working volume controls in that menu.

Monitor defaults hide Video/Audio/Subtitle pickers because NLE already exposes
them through track controls. They remain available in Show / hide actions.
Fullscreen is the final configured action; the overflow button precedes the
trailing group, and the terminal Fullscreen target stays inline when it fits.
Simple's independent layout and volume presentation are unchanged.

Web uses the same grouped action/order/visibility model. Slider pointer-down
cannot start its enclosing action's HTML drag. Mute retains its correct command
and accessible name rather than becoming a fake Volume command.

## Verification and instance settings

- 26 focused native tests passed: real speaker drag across groups, slider
  gestures without action drag, unchanged Simple layout, default visibility,
  Fullscreen/menu/volume ordering, RTL, every density, bounded 240/400/700/1400
  layouts, existing popups, and seek-preview behavior.
- An additional opt-in native test hovered the real preview path, decoded a
  finite synthetic video into a 320×180 texture, and retained the paused
  position without creating a seek.
- TypeScript checking and Web control-bar, media-control and responsive
  guardrails passed.
- Existing instance preferences were updated through the narrow, checked peer
  configuration API: hide the three monitor pickers, move Fullscreen last, and
  enable both monitor and Simple seekbar hover thumbnails. No default thumbnail
  preference or unrelated user settings were changed.
- The live thumbnail endpoint returned HTTP 200 with a 320×180 image from the
  currently loaded media, preserving its paused state and position. Private
  media and thumbnails are not published.

Native captures, package receipt and deployment acceptance follow below once
the verified source has been packaged. Cafe remains deferred by the user.
