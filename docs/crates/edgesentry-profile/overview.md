# edgesentry-profile

Loads and validates a profile directory:

```
<profile-name>/
  rules.json      # rule definitions
  params.toml     # edge-deployable threshold values (no regulation text)
  kb/             # one file per rule ID — used by edgesentry-explain
```

`regulation` in `rules.json` appears verbatim in `AuditRecord`s. Use the exact clause text.

Built-in profiles: `fixtures/demo/`, `fixtures/sg-port-safety/`, `fixtures/sg-maritime-security/`, `fixtures/sg-port-compliance/`
