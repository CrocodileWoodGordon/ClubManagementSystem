#!/bin/sh
set -eu

MIGRATIONS_DIR=/docker-entrypoint-initdb.d/migrations

if [ ! -d "$MIGRATIONS_DIR" ]; then
  echo "[init] 未检测到 migrations 目录，跳过初始化"
  exit 0
fi

MIGRATION_FILES=$(find "$MIGRATIONS_DIR" -maxdepth 1 -type f -name '*.sql' | sort || true)

if [ -z "$MIGRATION_FILES" ]; then
  echo "[init] migrations 目录为空，跳过"
  exit 0
fi

for file in $MIGRATION_FILES; do
  echo "[init] 执行迁移: $(basename "$file")"
  psql -v ON_ERROR_STOP=1 -U "${POSTGRES_USER:-postgres}" -d "${POSTGRES_DB:-postgres}" -f "$file"
done
