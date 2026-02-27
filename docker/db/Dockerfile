# syntax=docker/dockerfile:1.6

FROM postgres:16-alpine

LABEL org.opencontainers.image.title="Club Management System DB" \
      org.opencontainers.image.description="Pre-initialized Postgres 16 with CMS schema but no business data." \
      org.opencontainers.image.version="260227"

# 将 SQL 迁移和初始化脚本注入 Postgres 默认的 init 目录
COPY backend/migrations /docker-entrypoint-initdb.d/migrations
COPY docker/db/init.sh /docker-entrypoint-initdb.d/001_apply_migrations.sh

RUN chmod +x /docker-entrypoint-initdb.d/001_apply_migrations.sh
