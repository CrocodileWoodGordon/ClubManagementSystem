# Club Management System

## 本地依赖
- Docker / Docker Compose（用于运行 Postgres 16，本地容器名 `club_db`）
- Rust 1.75+（建议通过 `rustup` 安装，确保可用 `cargo`）
- Node.js 20+ 与 npm 10+（Next.js 16 需要 ESM 支持）

## 环境变量
后端会通过 `dotenvy` 自动读取同级目录下的 `.env` 文件，最少需要如下配置：

```
# backend/.env
DATABASE_URL=postgres://admin:password123@localhost:5432/club_management
PORT=8080 # 可选，默认 8080
FRONTEND_ORIGIN=http://localhost:3000
```

> 提示：以下“启动步骤”与“常用调试命令”仅面向内部开发环境，涉及 `docker compose build`、`npm install` 等需要访问外网的流程。客户部署请直接查看文末“离线交付说明（客户环境）”，全程无需联网。

## 启动步骤
1. **数据库**：在仓库根目录执行 `docker compose up -d db`，首次会自动拉取 `postgres:16-alpine` 并创建数据库。
2. **后端**：
   - `cd backend`
   - `cargo run`
   - 服务启动后监听 `http://localhost:8080`，可用 `curl http://localhost:8080/health` 验证。
3. **前端**：
   - `cd frontend`
   - `npm install`（首次安装依赖）
   - `npm run dev` 并访问 `http://localhost:3000`

## 常用调试命令
- `docker compose logs -f db`：观察数据库容器输出。
- `cargo check` / `cargo test`（在 `backend/`）：静态检查或运行 Rust 单元测试。
- `npm run lint` / `npm run build`（在 `frontend/`）：前端 ESLint / 生产构建检查。

## Docker 打包交付流程
1. **构建镜像**（在仓库根目录）：
   ```bash
   docker compose build backend frontend
   ```
   该命令会调用 `backend/Dockerfile`（Rust 多阶段构建）与 `frontend/Dockerfile`（Next.js 生产构建），生成 `club-management-backend:latest` 与 `club-management-frontend:latest`。
2. **镜像验收**：本地启动一遍确保镜像可用：
   ```bash
   docker compose up -d
   docker compose logs -f backend
   docker compose logs -f frontend
   ```
   后端启动后会自动执行 `backend/migrations`，无需额外 `sqlx migrate run` 步骤。
3. **交付方式**：
   - **推送镜像仓库**：`docker tag club-management-backend:latest <registry>/club-backend:<tag>` 并 `docker push`。
   - **离线交付**：
     1. 依次执行 `docker save <image> | gzip` 导出 `club-management-backend:<tag>`、`club-management-frontend:<tag>` 以及预初始化数据库镜像 `club-management-db:<tag>`（包含最新 schema，无业务数据）。
     2. 打包 `docker-compose.yml`、`.env.production`（或客户定制示例）、离线部署说明，确保客户解压后即可 `docker load`。
     3. 为每个镜像附带 `SHA256` 校验值，客户可通过 `shasum -a 256 *.tar.gz` 验证包体完整。
     4. 前端镜像已经包含 `npm ci` 与 `npm run build` 结果，客户侧不会再触发依赖安装；后端镜像包含 Rust Release 可执行文件。

## 客户离线部署说明（Docker）
1. **准备环境**：安装 Docker Engine 24+ 与 Docker Compose v2，确认 3000/8080/5432 端口空闲；将我们交付的 `club-management-backend-<tag>.tar.gz`、`club-management-frontend-<tag>.tar.gz`、`club-management-db-<tag>.tar.gz`、`docker-compose.yml`、`.env.production` 放在同一目录。
2. **导入镜像**（全程离线）：
   ```bash
   docker load -i club-management-db-<tag>.tar.gz
   docker load -i club-management-backend-<tag>.tar.gz
   docker load -i club-management-frontend-<tag>.tar.gz
   ```
   - 数据库镜像已包含最新 schema，首次启动会在本地卷 `postgres_data` 内创建空库。
   - 前端/后端镜像均已完成构建，不再执行 `npm install` 或访问外网；可用 `docker images | grep club-management` 确认三者均存在。
3. **配置环境变量**：根据部署环境填写 `.env.production`（示例内容与 `backend/.env` 相同），重点确认 `DATABASE_URL=postgres://admin:password123@db:5432/club_management`、`FRONTEND_ORIGIN` 以及前端 `NEXT_PUBLIC_API_URL`/`API_PROXY_TARGET`。
4. **首次启动**：
   ```bash
   docker compose --env-file .env.production up -d --no-build
   ```
   - `--no-build` 会忽略 compose 中的 `build` 段，确保只使用刚刚导入的镜像，不触发任何拉取/构建步骤。
   - 若需要单独启动某个服务，可附加服务名（例如 `docker compose ... up -d --no-build backend frontend`）。
5. **健康检查**：
   ```bash
   docker compose ps
   curl http://localhost:8080/health
   curl -I http://localhost:3000
   ```
   浏览器访问 `http://localhost:3000` 可确认前端静态资源与 API 代理是否正常（不需要额外下载）。
6. **运维与升级**：
   - 日志：`docker compose logs -f backend`、`docker compose logs -f frontend`、`docker compose logs -f db`。
   - 重启：`docker compose restart backend`。
   - 停止：`docker compose down`（保留数据）或 `docker compose down -v`（连同数据库卷一起清空）。
   - 升级：仅需加载新版本 tar 包，然后运行 `docker compose --env-file .env.production up -d --no-build --force-recreate`，无需 `docker compose pull`。
   - 如看到 “image not found” 报错，请重新执行 `docker load` 并用 `docker compose images` 检查三张镜像均已就绪。

### 数据库重建（开发环境）
```bash
# 关闭旧库后重建空库
docker compose exec db psql -U admin -d postgres -c "DROP DATABASE IF EXISTS club_management WITH (FORCE)"
docker compose exec db psql -U admin -d postgres -c "CREATE DATABASE club_management;"

# 重新应用 migrations
cd backend
cargo sqlx migrate run

# 可选：检查表结构
docker compose exec db psql -U admin -d club_management -c "\d+ campuses"
```
