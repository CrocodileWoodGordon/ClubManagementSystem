# Club Management System — Agent Notes

## 业务与流程速览
- Excel 报名导入：解析“年级班级姓名 + 周一~周五社团”生成 enrollment，初始班级均指向“待定班”。
- 分班：管理员在前端按照“社团 + 星期”筛选学生，批量设置班级编号/时间/地点，生成 `classes` 并回写 enrollment。
- 考勤：按班级导出空白考勤表 -> 期末回收后导入，历史考勤在退换课时依然保留。
- 换课/退课：旧 enrollment 置为 `DROPPED/TRANSFERRED`，记录 drop_date；同社团换班共用材料费，跨社团再次收材料费；三节课内退课免课时费。
- 结算：依据考勤 + 教师子女 + 退课规则计算课时费，再叠加材料费，生成班级和个人维度报表。

## 仓库结构
### 根目录
- `docker-compose.yml`：Postgres + Axum + Next.js 的本地部署定义。
- `PROJECT_SPEC.md`：完整业务规格说明，变更任何业务前务必复核。
- `DATABASE.md`：数据库范式设计（表、约束、索引），实现迁移/查询时必须对照此文件。
- `AGENTS.md`：当前文件，供后续 AI 参考。

### backend（Rust + Axum）
- `Cargo.toml`：依赖（Axum、SQLx、calamine 等）。若新增 crate，务必确认 docker 镜像同步更新。
- `src/main.rs`：应用入口，读取 `AppConfig`、初始化数据库连接并启动 Axum Router。
- `src/config/`：`AppConfig` 负责读取 `DATABASE_URL`、`PORT`、`FRONTEND_ORIGIN`。
- `src/db/`：`connect` 创建 SQLx 连接池；`models/` 下的 `*_row` 结构体对应核心数据表。
- `src/domain/`：业务领域模型（学生、社团、班级、enrollment、考勤、费用拆解）。服务层、API 都应优先使用这些类型。
- `src/services/`：按场景拆分（报名、分班、考勤、Excel 导入、计费、报表），屏蔽存储与任务细节。
- `src/api/`：每个业务域的路由 + handler，使用 Axum Router 组合并暴露 `/health`、`/api/*`。
- `src/tasks/`：长耗时任务（考勤表生成、结算批处理）。
- `src/utils/`：通用工具（Excel workbook 读取、时间工具）。
- `src/error.rs`：统一 `AppError`，实现 `IntoResponse`。

### frontend（Next.js App Router）
- `app/`：
  - `page.tsx`：登录页/入口指向 `/dashboard`。
  - `(admin)/layout.tsx`：后台共用框架 + 导航。
  - `(admin)/dashboard|enrollments|class-assignment|attendance|billing|reports|students|settings`：每个子页面放置业务描述与占位 UI。
- `components/`：
  - `common/SectionCard.tsx`：统一 section 外观。
  - `forms/BulkAssignmentForm.tsx`、`tables/StudentTable.tsx`、`upload/ExcelDropzone.tsx`、`widgets/MetricCard.tsx`：复用组件。
- `hooks/useSelection.ts`：前端批量选人逻辑占位。
- `lib/`：API client、领域类型、工具函数。
- `services/`：面向 API 的封装（例如 `enrollmentService`）。
- `constants/`：界面常量（周一~周五）。
- `providers/`：未来全局 Provider（当前只做占位）。

## 操作要求
1. **任何修改完成后必须在仓库根目录运行 `git add -A && git commit -m "<概述本次修改的标题>"`。**
2. 通过 `rg`/`rg --files` 查找内容；避免无意义的格式化更改。
3. 动到后端接口或数据库时，保持 `PROJECT_SPEC.md` 与实现一致，如有偏差先写明假设。
4. 向用户反馈时统一使用中文说明，保持语气专业、简洁。
5. 每次提交若涉及进度或跨模块契约，请同步更新下方“实时进展”与“跨文件接口备忘”。
6. 每次完成代码部分的更新时要通过编译和简单调试。调试方法要反馈给用户。

## 实时进展
- 初始化阶段：完成 Next.js/Axum 工程脚手架、Docker Postgres 已运行（容器 `club_db`，`admin/password123`）。尚未实现实际业务逻辑，API 仅返回占位数据。
- 2026-02-04：完成数据库重构设计，新增 `DATABASE.md` 作为 schema 真源，后续 migrations 与服务实现需严格参照。
- 2026-02-04：依据 `DATABASE.md` 实现 `backend/migrations/0001_init.sql`，包含 terms/students/clubs/classes/enrollments/attendance/billing 等全部核心表。

## 跨文件接口备忘
- `frontend/services/enrollmentService.ts` 通过 `GET /api/enrollments/pending` 读取待分班名单（当前返回空数组，需要后端实现）。
- `frontend/components/forms/BulkAssignmentForm.tsx` 预期调用 `/api/classes/assign` 批量设置班级编号（暂未接线）。
- `frontend/components/upload/ExcelDropzone.tsx` 将对接 `/api/import/enrollments` 上传 Excel。
