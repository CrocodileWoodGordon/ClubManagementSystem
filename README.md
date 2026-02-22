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
   - **离线交付**：使用 `docker save club-management-backend:latest | gzip > backend.tar.gz`（前端同理），连同 `docker-compose.yml`、`.env` 配置说明一起传给客户。

## 客户部署指南（Docker）
1. **准备环境**：安装 Docker Engine（24+）与 Docker Compose v2，确保 3000/8080/5432 端口空闲。
2. **获取交付物**：解压代码或拉取 Git 仓库，并下载后端/前端镜像（若是离线包，执行 `docker load -i backend.tar.gz`、`docker load -i frontend.tar.gz`）。
3. **首次启动**：
   ```bash
   # 构建镜像：若已提供预构建镜像可跳过
   docker compose build

   # 启动全部服务
   docker compose up -d
   ```
   - Postgres 服务：`postgres:16-alpine`，数据持久化到本地 Docker Volume `postgres_data`。
   - Backend 服务：监听 `http://localhost:8080`，容器名 `club-management-backend`。
   - Frontend 服务：监听 `http://localhost:3000`，容器名 `club-management-frontend`。
4. **健康检查**：
   ```bash
   curl http://localhost:8080/health
   curl -I http://localhost:3000           # 前端可用即返回 200
   ```
   前端页面可在浏览器打开 `http://localhost:3000`，使用后台默认入口登录。
5. **环境变量调整**：
   - `backend`：通过 `docker-compose.yml` 中的 `DATABASE_URL` / `PORT` / `FRONTEND_ORIGIN` 覆盖，若部署到远程域名，请把 `FRONTEND_ORIGIN` 改为实际访问地址。
   - `frontend`：`NEXT_PUBLIC_API_URL` 默认为 `/api`，`API_PROXY_TARGET` 默认为 `http://backend:8080`。若反向代理层暴露不同域名或端口，修改 compose 中对应环境变量并重新 `docker compose build frontend`。
6. **运维常用命令**：
   ```bash
   docker compose logs -f backend
   docker compose logs -f frontend
   docker compose restart backend
   docker compose down            # 停止但保留数据卷
   docker compose down -v         # 停止并清理数据库数据
   ```
   升级版本时只需拉取最新代码/镜像，执行 `docker compose pull`（或 `build`）并 `docker compose up -d --force-recreate`。

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
