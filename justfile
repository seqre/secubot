database_url := env_var_or_default('SCBT__DATABASE__URL', 'db.sqlite')

default:
  @just --choose {{justfile()}}

migrations-list:
    @diesel --database-url '{{database_url}}' migration --migration-dir migrations/sqlite list

migrations-redo:
    @diesel --database-url '{{database_url}}' migration --migration-dir migrations/sqlite redo

migrations-run:
    @diesel --database-url '{{database_url}}' migration --migration-dir migrations/sqlite redo
