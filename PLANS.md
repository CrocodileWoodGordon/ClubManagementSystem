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

12. **建模考勤记录**
    - 目标：在 `backend/src/domain/attendance.rs` 定义 daily_session、presence_status、import batch 等结构。
    - 预期修改文件：`backend/src/domain/attendance.rs`
    - 衔接：为考勤导入与结算准备数据结构。

13. **实现考勤导入/导出服务**
    - 目标：新增 `backend/src/services/attendance.rs`，提供考勤模板导出与导入、历史保留逻辑。
    - 预期修改文件：`backend/src/services/attendance.rs`
    - 衔接：后续 API 和结算将依赖此服务。

14. **暴露考勤 API**
    - 目标：在 `backend/src/api/attendance.rs` 定义导出、导入、查询端点。
    - 预期修改文件：`backend/src/api/attendance.rs`
    - 衔接：允许前端 attendance 页面读写考勤。

15. **实现前端考勤页面**
    - 目标：完善 `frontend/(admin)/attendance/page.tsx`，展示导出的模板及导入结果，调用步骤 14 的接口。
    - 预期修改文件：`frontend/(admin)/attendance/page.tsx`
    - 衔接：使考勤流程可视化。

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

23. **补充规范与接口文档**
    - 目标：当以上步骤稳定后，单独更新 `PROJECT_SPEC.md`，把与实现一致的接口/模型补充进去。
    - 预期修改文件：`PROJECT_SPEC.md`
    - 衔接：确保文档与代码同步，方便后续维护。
