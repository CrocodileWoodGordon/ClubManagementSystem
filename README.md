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

