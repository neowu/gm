# 0.7.1

- db: add `gm db sync --force`, revoke privileges of users not in config, drop `PASSWORD` ones (`IAM` ones are deleted by gcloud)
- db: `gm db status` shows users not in config

# 0.7.0

- db: add `gm db status`
- db: add `--config` to only apply one config file in env db dir
- clickhouse: add `gm clickhouse sync --force`, only revoke grants with it

# 0.6.8

- clickhouse: support clickhouse

# 0.6.4

- postgres: supports SUPER role

# 0.6.3

- add version check, to make sure use corresponding gm
