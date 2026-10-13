provide "db" subcommand for gcloud sql (MySQL / PostgreSQL), config: `{env}/db/*.json` (env path via `--env`, default '.'), each json file is one sql instance

- `--config {file}` (e.g. `--config db.json`): only apply `{env}/db/{file}`, default all json files

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
   - grant privileges by `role` (see roles table) on target dbs, additive only (never revoke)
5. write kube headless Service + EndpointSlice + Endpoints to `{env}/{endpoint.path}`, pointing to instance private ip, so apps access db via `{endpoint.name}.{endpoint.ns}`

## `gm db status`

show diff between db and config, read only, connect as root with password from `rootSecret` (must exist, run sync first)

- for each config file, print `instance {instance}:` then
- for each db: `up to date` or `not found`
- for each user: `up to date`, `not found`, or grants diff
  - MySQL: `SHOW GRANTS` split per privilege (`USAGE`, `SHOW_ROUTINE` and cloud sql partial revokes on `mysql` / `sys` ignored; `SHOW_ROUTINE` is auto granted by mysql upgrade to users with global `SELECT` and cloud sql root can't revoke it), compare with privileges of `role` on target scope; extra grants implied by existing global grants are not shown (db scope privilege also granted on `*.*`, e.g. left by old gm granting MIGRATION per db; `FLUSH_PRIVILEGES` auto granted by mysql upgrade to users with global `RELOAD`)
  - PostgreSQL: only privileges managed by sync, on config dbs: database acl, `public` schema acl, role membership (except gcloud managed `cloudsqliamuser` / `cloudsqliamserviceaccount`), `REPLICATION` attribute
- diff: `-` exists in db but not in config (sync never revokes, so it stays until removed manually), `+` in config but not in db
- endpoint file: `up to date`, `outdated` or `not found`, compared with generated content

## roles

| role        | MySQL                                                              | PostgreSQL                                                           |
|-------------|--------------------------------------------------------------------|----------------------------------------------------------------------|
| APP         | read / write on db                                                 | connect, pg_read_all_data, pg_write_all_data                         |
| MIGRATION   | ddl + read / write, global scope                                   | connect / create on db, create / usage on public schema (owns tables) |
| VIEWER      | select on db                                                       | connect, pg_read_all_data                                            |
| REPLICATION | replication privileges, global scope (required by mysql)           | user created WITH REPLICATION, connect, usage on public, pg_read_all_data |
| SUPER       | not supported                                                      | APP + cloudsqlsuperuser (e.g. view pg_stat_activity)                 |
