provide "clickhouse" subcommand, config: `{env}/clickhouse/clickhouse.jsonc` (env path via `--env`, default '.'), example in env/clickhouse/clickhouse.jsonc

- config is jsonc (comments / trailing commas allowed), `version` must match gm version
- `url` is clickhouse http interface, connect as `default` user with password from `rootSecret` (secret must exist)
- user `profile` / `role` are required, must reference names in `profiles` / `roles`

## `gm clickhouse sync` (idempotent)

1. create or update profiles, settings are replaced entirely by config
2. for each user
   - get or generate uuid password in `secret`
   - create or update user with the password
   - assign `profile`
   - grant `grants` of `role` directly to user, additive only (never revoke, to not interrupt current work; revoke manually if needed)

## `gm clickhouse send-password [--user {user_name}]`

- if user not specified, do it for all users having email
- get password from secret (must exist, run sync first), send email with url / user / password
- use sendgrid api, token / from address in `email` section of config
