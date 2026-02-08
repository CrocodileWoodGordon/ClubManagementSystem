# Club Management System — Agent Notes

## 业务与流程速览
- Excel 学生名单导入：以“校区 / 班级 / 姓名”三列的 Excel 批量生成/更新 `homerooms` 与 `students`，班级唯一键包含校区 + 学年 + 班级显示名。
- Excel 报名导入：解析“年级班级姓名 + 周一~周五社团”生成 enrollment，初始班级均指向“待定班”。
- 分班：管理员在前端按照“校区 + 社团 + 星期”筛选学生，批量设置班级编号/时间/地点，生成 `classes` 并回写 enrollment。
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
7. 所有前端页面在加载耗时/大数据量内容时，必须提供明显的加载反馈（如路由级 `loading.tsx`、骨架屏、提示文案），确保切换路由或标签后立即给用户响应。

## 实时进展
- 初始化阶段：完成 Next.js/Axum 工程脚手架、Docker Postgres 已运行（容器 `club_db`，`admin/password123`）。尚未实现实际业务逻辑，API 仅返回占位数据。
- 2026-02-03：补充 `README.md` 运行手册，说明本地依赖、环境变量与调试命令，方便新人快速起步。
- 2026-02-04：完成数据库重构设计，新增 `DATABASE.md` 作为 schema 真源，后续 migrations 与服务实现需严格参照。
- 2026-02-04：依据 `DATABASE.md` 实现 `backend/migrations/0001_init.sql`，包含 terms/students/clubs/classes/enrollments/attendance/billing 等全部核心表。
- 2026-02-05：扩展 `backend/src/domain/enrollment.rs`，定义报名状态、Excel 草稿与导入反馈结构，供服务层统一复用。
- 2026-02-05：完成 Excel 报名导入服务（`utils/excel.rs`、`services/enrollment_import.rs`、`api/imports.rs` 等），支持解析问卷星 Excel -> `import_jobs`/`enrollments` 并记录错误，`cargo check` 通过验证。
- 2026-02-05：新增学生 Excel 导入链路（`services/student_import.rs` + `/api/import/students`），支持按校区/班级/姓名写入 `homerooms` 与 `students`，自动复用激活学期的学年信息。
- 2026-02-05：落地多校区能力，新增 `campuses` 表并为 `homerooms`/`club_terms`/`classes`/`enrollments` 添加 `campus_id` 外键；同步更新 `DATABASE.md`、`PROJECT_SPEC.md` 与报名导入服务以按校区匹配社团。
- 2026-02-05：新增学期/校区管理接口（`/api/admin/terms`、`/api/admin/campuses/*`）以及 `API.md` 文档，后续前端可直接创建学期、维护校区信息。
- 2026-02-06：修复 Axum 0.8 路由写法（`/api/admin/campuses/{id}`、`/api/attendance/template/{class_id}`），`cargo run` 可正常启动，已用 `curl --noproxy "*"` 验证 `/health`、`/api/enrollments/pending`、`/api/classes/pending`、`/api/reports/settlement` 等占位端点均返回 200。
- 2026-02-06：新增 `POST /api/admin/campuses`，支持传入 `code/name/...` 创建校区；同一接口返回 `CampusDto`，并保持原有 `PATCH /api/admin/campuses/{id}` 可继续更新名称、联系人等信息。
- 2026-02-06：学生 Excel 导入已改为按 `campuses.short_name`（兼容 code）匹配校区，并完善冲突写法避免数据库约束错误。
- 2026-02-06：Excel 上传兼容 `.xls/.xlsx`，失败时自动回退到临时文件解析，同时新增 `.gitignore` 忽略 `target/` 与 `data2import/`。
- 2026-02-06：问卷星报名导入支持列映射（默认 E/H~L），可通过 Multipart `config` JSON 自定义列；若 Excel 中出现新社团，会自动创建 `clubs`/`club_terms` 并按校区/星期存量去重。
- 2026-02-06：完善报名查询接口，`GET /api/enrollments/pending` 可按学期、校区、班级、社团、星期、学生姓名筛选，并新增 `/api/enrollments/summary` 返回“校区+社团+星期”统计结果供横向对比。
- 2026-02-06：新增导入占位文本配置（`GET/PUT /api/import/placeholders`），可查询或更新 Excel 中代表空报名的字符串；导入流程会实时读取该配置并跳过占位值。
- 2026-02-07：封装前端报名 Service，统一 `enrollmentService.ts` 错误处理与字段映射，并新增 Excel 导入调用以返回逐行导入结果，供后续组件直接复用。
- 2026-02-07：`/app/(admin)/enrollments` 页面改为实时请求后端的待分班与汇总接口，并新增 `EnrollmentImportPanel` 通过 Excel 上传组件直接调用导入接口，前端可查看逐行导入反馈。
- 2026-02-07：新增 `GET /api/enrollments/slots` 接口，可按“校区/社团/星期”返回报名学生列表，导入后可直接在前端核对名单，并在前端 `enrollmentService.ts` 中提供对应调用封装。
- 2026-02-07：报名汇总区新增下拉筛选（校区/社团/星期），`EnrollmentSlotExplorer` 使用 `fetchEnrollmentSlotDetails` 实时展示符合条件的报名名单，并提供“查询报名名单”按钮失败后可再次触发查询，方便导入后校验。
- 2026-02-07：后端启用 CORS（`FRONTEND_ORIGIN`，默认 `http://localhost:3000`），支持 Next.js 前端直接访问 `http://localhost:8080` 的 API。
- 2026-02-08：实现班级分配闭环：后端新增 `GET/POST /api/classes` 与 `POST /api/classes/assign`，支持按校区+社团+星期查询/创建具体班级并批量更新 `enrollments.class_id`；`/api/enrollments/slots` 响应补充 `class_id/class_code` 方便前端展示当前分班状态；前端新增“班级分配”页面，复用报名 slots 过滤，内置班级列表+新建表单、学生分班下拉与多选批量操作。
- 2026-02-08：补充 `PUT /api/classes/{id}`，允许更新既有班级的编号/时间/地点/容量；前端 `ClassAssignmentBoard` 支持“编辑班级”流程，可在列表内一键进入编辑模式并提交至新接口。
- 2026-02-08：报名管理页面改为二级导航结构，导入/汇总/筛选报名名单/待处理名单拆分为独立子页面，初次进入仅加载导入面板，避免一次性获取所有报名数据。
- 2026-02-08：待处理名单子页面新增骨架屏 `loading.tsx`，并确立“前端页面加载需即时反馈”规范。
- 2026-02-08：占位文本配置新增 `(跳过)` 默认值，并在数据库侧清理同名社团；前端 `/enrollments/placeholders` 提供查看/新增/删除占位文本的管理界面。

## 跨文件接口备忘
- `frontend/services/enrollmentService.ts` 通过 `GET /api/enrollments/pending` 读取待分班名单（当前返回空数组，需要后端实现）。
- `frontend/components/forms/BulkAssignmentForm.tsx` 预期调用 `/api/classes/assign` 批量设置班级编号（暂未接线）。
- `frontend/components/upload/ExcelDropzone.tsx` 将对接 `/api/import/enrollments` 上传 Excel。
- 新增校区维度：后端 `enrollments`、`classes`、`club_terms` 均要求 `campus_id`，Excel 导入会根据学生的 `homeroom` 自动写入，前端筛选/分班时需补充校区参数（`/api/enrollments/pending` 已支持多条件筛选）。
- `/api/import/placeholders` 返回当前导入场景使用的占位文本；PUT 接口可让管理员自定义占位字符串，前端如需展示可直接读取该接口。
- `/api/import/students` 接收 `file` 字段的 Excel（支持 `.xls/.xlsx`），A 列填校区简称（匹配 `campuses.short_name`，也兼容 `code`），B 列班级（显示名），C 列姓名；激活学期 `start_date` 的年份会写入 `homerooms.academic_year`。
- `/api/admin/terms` 列出/创建学期；`/api/admin/campuses` 新增校区；`/api/admin/campuses/{id}` 支持更新校区名称、地址及联系人信息，准备在前端提供直接维护入口。
- `frontend/services/enrollmentService.ts` 暴露 `fetchEnrollmentSlotDetails`，调用 `GET /api/enrollments/slots` 根据三元组拉取报名学生列表供导入后核对。
- `/app/(admin)/enrollments/page.tsx` 通过 `EnrollmentSlotExplorer` 组件下发汇总数据并触发 `fetchEnrollmentSlotDetails`，在前端直接渲染筛选结果名单。
- `/api/classes` 提供按学期/校区/社团/星期查询及新建班级能力，返回 `assigned_count`；`POST /api/classes/assign` 用于批量写入 `enrollments.class_id`，并在 `class_id = null` 时恢复 `PENDING` 状态。
- 前端 `ClassAssignmentBoard`（`/app/(admin)/class-assignment`）复用报名汇总的筛选条件，调用 `fetchEnrollmentSummary` + `fetchEnrollmentSlotDetails` + `classAssignmentService`，提供单选/多选分班，并支持创建/编辑班级；`BulkAssignmentForm` 组件接收 `classes` 列表进行批量分配。
- `/app/(admin)/enrollments/placeholders` 使用 `importPlaceholderService` 调用 `/api/import/placeholders`，支持管理员在前端增删改占位字符串。
