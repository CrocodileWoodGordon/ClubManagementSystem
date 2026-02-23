# 项目开发规划

## 执行准则
- 每一步尽量聚焦单一文件，便于精确 Code Review 与回滚；如需改动其他文件，先在步骤说明里注明。
- 完成每一步后立刻 `git add -A && git commit -m '<step title>'`，并在提交信息中用与步骤一致的标题，方便追踪。
- 完成每一步后要将这次完成的内容写进 `AGENTS.md` 文件的实时进展部分。
- 任一步如果触碰接口/契约，及时同步 `PROJECT_SPEC.md` 或 `AGENTS.md`，但尽量安排独立步骤避免跨文件修改。

## 分步计划
1. **补充运行文档**
   - 目标：把本地依赖、调试命令、环境变量写进 `README.md`，确保新人能在不看其它文件的情况下跑通环境。
   - 预期修改文件：`README.md`
   - 衔接：为后续所有步骤提供统一的运行基线。

2. **建立初始数据库结构**
   - 目标：按照 `DATABASE.md` 中的范式设计创建 `backend/migrations/0001_init.sql`，覆盖 terms/students/clubs/classes/enrollments/attendance/billing/core 支撑表，暂不写入业务逻辑。
   - 预期修改文件：`backend/migrations/0001_init.sql`
   - 衔接：供 SQLx 模型与服务层共用的 schema 来源，必须与 `DATABASE.md` 保持一致。

3. **建模 Enrollment 领域类型**
   - 目标：在 `backend/src/domain/enrollment.rs` 定义报名相关 structs/enums（含导入状态、drop_date 等）。
   - 预期修改文件：`backend/src/domain/enrollment.rs`
   - 衔接：让后续服务层可以使用强类型而非裸 SQL 结果。

4. **实现 Excel 报名导入服务**
   - 目标：新增 `backend/src/services/enrollment_import.rs`，实现解析 Excel -> 写入 enrollments 的核心流程（依赖 domain 类型 + SQLx）。
   - 预期修改文件：`backend/src/services/enrollment_import.rs`
   - 衔接：作为 API 与任务层共用的业务核心。

5. **暴露报名导入 API**
   - 目标：在 `backend/src/api/enrollments.rs` 实现 `POST /api/import/enrollments` handler，并在该文件内注册路由（再由 `api/mod.rs` 引入）。
   - 预期修改文件：`backend/src/api/enrollments.rs`
   - 衔接：给前端上传 Excel 的入口，直接复用步骤 4 的服务。

6. **封装前端报名 Service**
   - 目标：完善 `frontend/services/enrollmentService.ts`，增加导入与“待分班名单”请求，保持统一错误处理。
   - 预期修改文件：`frontend/services/enrollmentService.ts`
   - 衔接：前端其余模块不直接依赖 API 地址，复用该封装。

7. **完成 Excel 上传组件**
   - 目标：扩展 `frontend/components/upload/ExcelDropzone.tsx`，调 `enrollmentService` 完成上传，并在 UI 中给出导入状态。
   - 预期修改文件：`frontend/components/upload/ExcelDropzone.tsx`
   - 衔接：管理员即可发起报名导入，触发步骤 5 的接口。

8. **建模班级与排课信息**
   - 目标：在 `backend/src/domain/class.rs` 描述 class_id、weekday、time_slot、room 等字段及校验逻辑。
   - 预期修改文件：`backend/src/domain/class.rs`
   - 衔接：支撑后续分班服务。

9. **实现分班批量服务**
   - 目标：新增 `backend/src/services/class_assignment.rs`，支持“社团+星期”筛选与批量更新 enrollment -> class_id。
   - 预期修改文件：`backend/src/services/class_assignment.rs`
   - 衔接：供 API 和任务层复用，直接操作步骤 8 的类型。

10. **提供班级分配 API**
    - 目标：在 `backend/src/api/classes.rs` 中实现 `POST /api/classes/assign`，校验 payload 后调用步骤 9 的服务。
    - 预期修改文件：`backend/src/api/classes.rs`
    - 衔接：对接前端批量分班表单。

11. **完善前端批量分班表单**
    - 目标：实现 `frontend/components/forms/BulkAssignmentForm.tsx` 的提交流程，使用步骤 6 的 service。
    - 预期修改文件：`frontend/components/forms/BulkAssignmentForm.tsx`
    - 衔接：完成“分班”业务闭环。

12. **落地考勤领域模型**
    - 目标：依据 `DATABASE.md` 在 `backend/src/domain/attendance.rs` 定义 `AttendanceStatus`、`AttendanceSessionKey`、`AttendanceRecord`、`AttendanceImportBatch` 等结构，并提供 SQLx row/Excel DTO 的 `TryFrom` 与校验 helper。
    - 预期修改文件：`backend/src/domain/attendance.rs`、`backend/src/domain/mod.rs`
    - 验证：在 `backend` 目录执行 `cargo test domain::attendance`，覆盖状态转换与模板校验单测。
    - 衔接：服务层与 API 直接复用统一模型，避免后续重复定义。

13. **实现考勤导入/导出服务**
    - 目标：新增 `backend/src/services/attendance.rs`，实现空模板生成（含班级/周次/日期列）、Excel 导入解析、历史批次保留与幂等写入，复用 `utils/excel` 并处理缺席/调课学生过滤。
    - 预期修改文件：`backend/src/services/attendance.rs`、`backend/src/services/mod.rs`
    - 验证：在 `backend` 目录运行 `cargo test services::attendance`，对模板行数与导入规则写集成单测。
    - 衔接：API 仅需薄封装即可使用该服务，也是结算逻辑依赖的数据入口。

14. **暴露考勤 API**
    - 目标：添加 `backend/src/api/attendance.rs`，实现 `GET /api/attendance/template/{class_id}`、`POST /api/attendance/import`（multipart）与 `GET /api/attendance?class_id=` 查询，并在 `api/mod.rs` 注册路由与 CORS。
    - 预期修改文件：`backend/src/api/attendance.rs`、`backend/src/api/mod.rs`
    - 验证：运行 `cargo run` 后，使用 `curl --noproxy "*"` 请求模板导出/历史查询/导入上传文件，确认 200 与错误提示。
    - 衔接：前端和 QA 可以直接通过 HTTP 验证考勤流程。

15. **实现前端考勤页面**
    - 目标：完善 `frontend/app/(admin)/attendance/page.tsx`，新建 `frontend/services/attendanceService.ts` 封装下载/上传/历史查询逻辑，并提供 `frontend/app/(admin)/attendance/loading.tsx` 骨架屏，页面内支持筛班级、下载模板、上传 Excel 后展示逐行反馈。
    - 预期修改文件：`frontend/services/attendanceService.ts`、`frontend/app/(admin)/attendance/page.tsx`、`frontend/app/(admin)/attendance/loading.tsx`
    - 验证：在 `frontend` 目录执行 `npm run lint` 与 `npm run dev`，配合步骤 14 API 实测模板下载和导入提示。
    - 衔接：管理员可在前端独立完成考勤导出/导入，为后续结算提供已验证数据。

16. **处理换课/退课领域**
    - 目标：在 `backend/src/domain/enrollment_status.rs` 定义状态机与规则（材料费共享、三节内退课免课时费等）。
    - 预期修改文件：`backend/src/domain/enrollment_status.rs`
    - 衔接：为后续服务计算提供判定依据。

17. **实现换课/退课服务**
    - 目标：新增 `backend/src/services/enrollment_status.rs`，封装换班、退课、drop_date 记录、费用共享逻辑。
    - 预期修改文件：`backend/src/services/enrollment_status.rs`
    - 衔接：供 API/前端直接调用。

18. **换课/退课 API**
    - 目标：在 `backend/src/api/enrollment_status.rs` 暴露更改状态的端点，并复用步骤 17 的规则。
    - 预期修改文件：`backend/src/api/enrollment_status.rs`
    - 衔接：方便前端学生详情或分班界面触发操作。

19. **建模与计算结算结果**
    - 目标：在 `backend/src/domain/billing.rs` 定义课时费、材料费、教师子女折扣等结构，顺带放入校验函数。
    - 预期修改文件：`backend/src/domain/billing.rs`
    - 衔接：让结算服务拥有统一数据模型。

20. **实现结算服务**
    - 目标：新增 `backend/src/services/billing.rs`，依据考勤与退课规则生成班级/个人报表，并与 SQLx 交互。
    - 预期修改文件：`backend/src/services/billing.rs`
    - 衔接：生成待导出的数据集。

21. **批处理与报表任务**
    - 目标：扩展 `backend/src/tasks/reporting.rs`，调度结算批处理、生成报表文件。
    - 预期修改文件：`backend/src/tasks/reporting.rs`
    - 衔接：为后端定时任务或手动触发准备。

22. **前端结算/报表界面**
    - 目标：完善 `frontend/(admin)/billing/page.tsx` 与 `frontend/(admin)/reports/page.tsx`（可分两次提交），展示费用汇总与导出入口。
    - 预期修改文件：先 `frontend/(admin)/billing/page.tsx`，再 `frontend/(admin)/reports/page.tsx`
    - 衔接：完成财务闭环，确保与步骤 20/21 的输出一致。

23. **重构考勤模板表头**
    - 目标：调整导出模板的前四行合并单元格布局，以班级名称、社团名称、校区、上课时间填充，并在考勤列默认填入“正常”，列头命名为“第X周”。
    - 预期修改文件：`backend/src/services/attendance.rs`、相关单元测试。
    - 验证：在 `backend` 目录运行 `cargo test services::attendance` 检查模板结构。

24. **支持考勤模板周次区间参数**
    - 目标：导出模板时允许传入起始/结束周（默认 1~18），生成不足 18 周时补齐空列。
    - 预期修改文件：`backend/src/services/attendance.rs`、`backend/src/api/attendance.rs`
    - 验证：`cargo test services::attendance` 覆盖边界，并用 `curl` 校验 API 参数。

25. **扩展前端考勤模板导出表单**
    - 目标：在考勤页面新增起始/终止周输入框，默认值 1 和 18，导出时将参数传递给后端。
    - 预期修改文件：`frontend/app/(admin)/attendance/page.tsx`、`frontend/services/attendanceService.ts`
    - 验证：在 `frontend` 目录执行 `npm run lint`，并手动确认导出请求参数。

26. **强化考勤页面按钮样式**
    - 目标：优化考勤页面内按钮的颜色、边框或悬停反馈，让可点击操作与正文文字区分明显。
    - 预期修改文件：`frontend/app/(admin)/attendance/page.tsx`、相关样式文件。
    - 验证：`npm run lint`，本地预览确认视觉效果。

27. **完善考勤导入重复记录合并规则**
    - 目标：当导入存在同一学生同一课次的多条记录时，仅在全部记录为“正常”时写入正常，否则按最严重状态处理。
    - 预期修改文件：`backend/src/services/attendance.rs`、导入相关单测。
    - 验证：在 `backend` 目录运行 `cargo test services::attendance`，覆盖重复记录场景。

28. **补充规范与接口文档**
    - 目标：当以上步骤稳定后，单独更新 `PROJECT_SPEC.md`，把与实现一致的接口/模型补充进去。
    - 预期修改文件：`PROJECT_SPEC.md`
    - 衔接：确保文档与代码同步，方便后续维护。

## 离线交付专项计划（2026-02-23）

29. **整理客户离线部署文档**（已完成 2026-02-23）
    - 目标：在 `README.md`（及必要时 `AGENTS.md`）补充“离线交付”章节，移除/标注所有需要外网拉取的操作（例如 `docker compose build`、`docker compose pull`、`npm install`），改为“加载离线镜像 tar 包 + docker compose up --no-build”的流程，并解释前端/后端镜像在离线场景下如何校验。
    - 预期修改文件：`README.md`、必要的话同步 `AGENTS.md` 中对交付方式的记录。
    - 验证：自查文档中不再出现让客户主动拉取镜像/依赖的指令，新增的离线流程涵盖镜像加载、环境变量、健康检查等步骤。

30. **提供离线专用 Compose 配置**
    - 目标：新建 `docker-compose.offline.yml`（或调整现有 compose，通过 profile/override）仅引用预构建镜像标签（例如 `club-management-backend:<tag>`、`club-management-frontend:<tag>`、`club-management-db:<tag>`），完全去掉 `build` 块，确保客户即使直接执行 `docker compose -f docker-compose.offline.yml up -d` 也不会触发 `npm ci`/`apt-get`/远程镜像拉取。
    - 预期修改文件：`docker-compose.yml`（若需提取共用部分）、新增 `docker-compose.offline.yml`、相应文档引用。
    - 验证：在本地先 `docker load` 三个 tar 包后，使用该 compose 文件启动，确认不会触发额外构建，容器均可正常启动并连通。

31. **制作预初始化数据库镜像**
    - 目标：基于 `postgres:16-alpine` 新增 `docker/db/Dockerfile`，将 `backend/migrations/*.sql` 复制到 `/docker-entrypoint-initdb.d/`，构建 `club-management-db:<tag>` 镜像。首启时自动创建 schema（无业务数据），并输出 `docker save club-management-db:<tag> | gzip > club-management-db-<date>.tar.gz` 的离线包提供给客户。
    - 预期修改文件：`docker/db/Dockerfile`、可能的辅助脚本（例如 `docker/db/README.md`），以及主文档中关于数据库镜像加载的说明。
    - 验证：本地 `docker run --rm -e POSTGRES_USER=admin -e POSTGRES_PASSWORD=password123 -e POSTGRES_DB=club_management club-management-db:<tag>` 后，用 `psql` 检查应具备 schema 但无业务数据，再通过 `docker save` + `docker load` 验证镜像可在离线环境复现。

## 近期待接项
- [ ] 将 `backend/src/db/models/*_row` 结构在 SQLx 查询层落地复用，避免长期依赖 `#[cfg_attr]` 抑制未使用告警。
- [ ] 衔接结算批处理：把 `ReportingTask::run_settlement_batch` 接入任务调度/触发流程，实际产出 CSV 并写入 `billing_runs`/`billing_items`。
- [ ] 在公开 API 中统一使用 `domain::Enrollment` 与 `domain::Club`，确保领域模型成为数据源而非临时 DTO。
- [ ] 落实考勤写库闭环：实现 `AttendanceService::record_bulk` 的数据库持久化，并在后续 API 调用中启用 `AttendanceTemplate` 访问器。
- [ ] 复用 `utils/time.rs` 与 `utils/excel.rs` 预留工具（如批处理时间戳、Workbook 多 sheet 支持），替换当前手写逻辑。

## 新增需求记录（2026-02-13）
- 考勤模板首行需要合并 20 个单元格填入班级名称并居中，第二行分成 8/4/8 个单元格分别写入社团名称、校区、上课时间（周几几点到几点），第三行整行合并填入提示：“将考勤情况为缺席的同学对应单元格删除（设为空），考勤情况正常的同学无需更改。如果课时过多，直接将后面多余课时所有同学的对应单元格删除（设置为空）即可。机器读取，请不要擅自改动文件其余部分。如有成员增减，请重新获取最新的考勤表。”
- 第四行为表头，第五行开始为正式数据：第一列编号从 1 递增，第二列为学生 UID（班级+学生姓名），第 3~20 列对应“第X周”考勤，默认填入“正常”。
- 导出考勤模板时需支持自定义起始周与终止周（默认 1~18），仅生成指定区间的列，不足 18 周的列保留为空。
- 考勤页面新增两个输入框读取起始周和终止周参数，导出模板时传入后端；若未填写使用默认值。
- 调整考勤页面按钮样式，让可点击的按钮与普通文本有明显区分度。
- 考勤导入遇到重复记录时，只有当多条记录全部为“正常”时才记为正常，否则按非正常状态处理。
