# Repository Guidelines

## Project Structure & Module Organization
- `backend/`: Rust + Axum API. Key folders: `src/api` (routes), `src/services` (business logic), `src/domain` (types), `src/tasks` (batch jobs). SQL migrations live in `backend/migrations`; keep them aligned with `DATABASE.md`.
- `frontend/`: Next.js App Router UI. `app/` hosts pages, `components/` holds reusable widgets, `services/` wraps API calls, and `lib/` stores shared utilities.
- `docker-compose.yml` starts the local Postgres container (`club_db`). `DATABASE.md` and `PROJECT_SPEC.md` are the authorities for schema and product scope.

## Build, Test, and Development Commands
- `docker compose up -d db` — boot Postgres 16 locally.
- `cd backend && sqlx migrate run && cargo run` — apply migrations then start the API on `:8080`.
- `cd backend && cargo check` for fast type validation; `cargo test` for Rust unit/integration tests.
- `cd frontend && npm install` once, then `npm run dev` for hot reload; `npm run lint` enforces Next.js lint rules.

## Coding Style & Naming Conventions
- Rust: use `rustfmt` defaults; favor explicit modules and `Result<T, AppError>`. File names snake_case; structs/enums in UpperCamelCase.
- TypeScript/React: 2-space indent, functional components, PascalCase for components, camelCase for hooks/utilities. Keep hooks in `hooks/`, server calls in `services/`.
- SQL migrations should be deterministic, idempotent, and reference UUID primary keys.

## Testing Guidelines
- Rust tests live beside code (`mod tests`). Use descriptive test names like `handles_pending_enrollment`. Prefer `sqlx::test` when DB access is required.
- Frontend: rely on Next.js lint + future React Testing Library suites; name files `*.test.tsx`. Ensure CI-safe commands (`npm run lint`, `cargo test`) stay green before pushing.

## Commit & Pull Request Guidelines
- Commits: single-purpose, present-tense summaries (e.g., `补充运行文档`, `建立初始数据库结构`). Always `git add -A && git commit -m "<summary>"` from repo root.
- PRs: describe scope, testing evidence (`sqlx migrate run`, `cargo test`, `npm run lint`), linked issues if any, and UI screenshots for frontend changes. Highlight schema or contract updates and confirm `DATABASE.md`/`PROJECT_SPEC.md` stays accurate.

## Security & Configuration Tips
- Never hardcode secrets; use `.env` (backend/.env template provided) and document required vars in README.
- When touching DB schema, update `DATABASE.md` first, then migrations, then application code. Always validate changes against a local Postgres instance before committing.
