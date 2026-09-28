provide "db" subcommand for gcloud sql (MySQL / PostgreSQL), config: `{env}/db/*.json` (env path via `--env`, default '.'), each json file is one sql instance

- config is plain json, `version` must match gm version
- `type` is `MySQL` or `PostgreSQL`
- user `auth` is `IAM` or `PASSWORD`, `PASSWORD` user must have `secret`
- user `db` is optional, default to all `dbs` in config
- MySQL user name must be no longer than 32

## `gm db sync` (idempotent)

for each config file

1. get instance public / private ip via sql admin api, connect via public ip
2. get or generate root password in `rootSecret`, reset root user (`root` for MySQL, `postgres` for PostgreSQL) password to it
3. create dbs if not exist
   - MySQL: utf8mb4 / utf8mb4_unicode_ci
   - PostgreSQL: enable `pg_stat_statements`, set `auto_explain` settings (log slow query > 3s)
4. for each user
   - `PASSWORD` user: get or generate password in `secret`, create user if not exist (MySQL also resets password to secret, PostgreSQL doesn't change existing user)
   - `IAM` user: user is expected to be created by gcloud, only grant privileges
   - grant privileges by `role` on target dbs, additive only (never revoke)
5. write kube headless Service + EndpointSlice + Endpoints to `{env}/{endpoint.path}`, pointing to instance private ip, so apps access db via `{endpoint.name}.{endpoint.ns}`

## roles

| role        | MySQL                                                              | PostgreSQL                                                           |
|-------------|--------------------------------------------------------------------|----------------------------------------------------------------------|
| APP         | read / write on db                                                 | connect, pg_read_all_data, pg_write_all_data                         |
| MIGRATION   | ddl + read / write, global scope                                   | connect / create on db, create / usage on public schema (owns tables) |
| VIEWER      | select on db                                                       | connect, pg_read_all_data                                            |
| REPLICATION | replication privileges, global scope (required by mysql)           | user created WITH REPLICATION, connect, usage on public, pg_read_all_data |
| SUPER       | not supported                                                      | APP + cloudsqlsuperuser (e.g. view pg_stat_activity)                 |
