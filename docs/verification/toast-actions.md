# Toast actions and error diagnostics

Publish with `pealayer.toast.show`. Each toast may bind up to four named actions
and set `default_action` to one action ID. Clicking its message executes that
binding; no action runs on publish, hover, or expiry. Named buttons remain
available. Bindings use existing validated Pealayer text commands and retain
normal command authorization and hardware gates.

```json
{
  "message": "Test finished. Inspect the configuration?",
  "actions": [{"id":"inspect","label":"Preferences","command":"preferences"}],
  "default_action":"inspect",
  "timeout_ms":10000
}
```

Errors expose Copy and Details on hover/focus (always on touch Web clients).
Without a custom default, clicking an error opens its diagnostic dialog. Pass
`details` to retain the original diagnostic, up to 8192 characters. The dialog
shows source and time and survives toast expiry; it does not claim to contain
unrelated application logs. Copy includes the same diagnostic. Clipboard failure
in Web opens selectable details with an actionable message.

`pealayer.toast.invoke` takes `id`, `created_at_ms`, and `action_id`. The host
rejects expired/replaced notifications and consumes an activation once to prevent
duplicate hardware dispatch. Notification actions cannot publish other toasts.

Verification: Web TypeScript and existing message lifecycle fixtures; native
validation/replacement/expiry/duplicate-activation tests run in GitHub CI. Native
production-host compilation is prohibited by AGENTS.md. Deployment and live UI
acceptance remain pending until the CI package is available and validated.
