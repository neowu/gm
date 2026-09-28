provide "clickhouse" subcommand, config: `{env}/clickhouse/clickhouse.jsonc` (env path via `--env`, default '.'), example in env/clickhouse/clickhouse.jsonc

- config is jsonc (comments / trailing commas allowed), `version` must match gm version
- `url` is clickhouse http interface, connect as `default` user with password from `rootSecret` (secret must exist)
- user `profile` / `role` are required, must reference names in `profiles` / `roles`

## `gm clickhouse sync` (idempotent)

1. create or update profiles, whole profile is replaced by config (settings replaced, `TO` cleared); use `ALTER` not `OR REPLACE`, as `OR REPLACE` creates new profile id and unlinks assigned users
2. for each user
   - get or generate uuid password in `secret`
   - create or update user with the password
   - assign `profile`
   - grant `grants` of `role` directly to user, additive only (never revoke, to not interrupt current work; revoke manually if needed)

## `gm clickhouse status`

show diff between clickhouse and config, read only

- for each profile: `up to date`, `not found`, or diff of whole `CREATE SETTINGS PROFILE` statement
- for each user: `up to date`, `not found`, or grants diff (grants of `role`)
- diff: `-` exists in clickhouse but not in config, `+` in config but not in clickhouse
- config side is normalized by clickhouse `formatQuerySingleLine`, to compare with `SHOW CREATE SETTINGS PROFILE` / `SHOW GRANTS` output; merged grants (`A, B ON db.*`) are split per privilege
- since sync never revokes, `-` grants remain after sync

## `gm clickhouse send-password [--user {user_name}]`

- if user not specified, do it for all users having email
- get password from secret (must exist, run sync first), send email with url / user / password
- use sendgrid api, token / from address in `email` section of config
