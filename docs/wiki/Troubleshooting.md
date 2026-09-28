# Troubleshooting

## Connect flashes green then disconnects

The button should remain in **Connecting…** until transport success or a real
failure. Inspect the error dialog's copyable technical details. A reachable
PCController with no board is a degraded state and should remain amber.

## Wrong serial devices appear

Pealayer lists only current operating-system enumeration. Missing devices are
not replaced with `/dev/tty*`, `COM` examples, or other sample endpoints. Confirm
the device is present in the OS and that another process does not own it.

## Windows build cannot find MPV

Check `rustc -vV`. MSVC needs `mpv.lib`; GNU needs `libmpv.dll.a` or
`libmpv.a`. Both require `libmpv-2.dll` or `mpv-2.dll` at runtime. Set
`LIBMPV_DIR` when the files are not under `%ProgramFiles%\MPV`.

## A full test run fails in CLI forwarding

The forwarding integration test owns Pealayer's local command endpoint. Stop the
installed instance during the controlled replacement/test window, or run the
test in an isolated job. Do not terminate an unknown or unverified production
process merely to free the port.

## Screenshot script cannot find a window

Run it inside the signed-in graphical session. SSH service sessions usually do
not have access to the interactive Windows desktop or Linux `DISPLAY`.
