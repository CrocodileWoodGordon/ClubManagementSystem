# API 总览（开发阶段）

> 所有接口默认基于 `http://localhost:8080`，当前阶段未做鉴权，可在本地以 curl/HTTPie 等工具直接调试。

## 1. 健康检查
- `GET /health`：容器/部署探活，返回 `{ status, service }`。

## 2. 学期管理（新建/查看）
- `GET /api/admin/terms`  
  列出所有学期，按 `start_date` 倒序。字段包含 `code`、`name`、`start_date`、`end_date`、`enrollment_start/end`、`is_active`。
- `POST /api/admin/terms`  
  请求体：`{ code, name, start_date, end_date, enrollment_start, enrollment_end, is_active }`。  
  当 `is_active=true` 时会先将其他学期的 `is_active` 置为 `false`。

## 3. 校区管理（查看/修改）
- `GET /api/admin/campuses`  
  返回所有校区的 `code`、`name`、`short_name`、`address`、`contact_name`、`contact_phone` 等元数据。
- `PATCH /api/admin/campuses/{id}`  
  请求体允许任意组合的 `{ name?, short_name?, address?, contact_name?, contact_phone? }`，用于局部更新；若请求体为空会返回校验错误。

## 4. Excel 导入
- `POST /api/import/students`  
  上传“校区简称/班级/姓名” Excel（支持 `.xls` 与 `.xlsx`，表头占第一行）。服务优先使用 `campuses.short_name`（兼容 `code`）匹配校区，自动创建/更新 `homerooms` 并写入 `students`；响应包含导入汇总（total/success/skipped/errors）。
- `POST /api/import/enrollments`  
  上传问卷星报名 Excel（默认 E 列为“班级+姓名”，H~L 列为周一~周五社团，可通过 Multipart `config` 字段传入 JSON 自定义列映射）。返回逐行 `EnrollmentImportOutcome`，并在 `import_jobs` 中记录任务，遇到新社团会自动创建 `clubs`/`club_terms`。
- `GET /api/import/placeholders`  
  查询各导入类型的占位文本（默认包含 `ENROLLMENTS`）。可用 `?import_type=ENROLLMENTS` 单独获取某一类型。
- `PUT /api/import/placeholders/{import_type}`  
  传入 `{ "placeholders": ["-", "(空)", "(跳过)"] }` 全量替换指定类型的占位文本，更新后导入逻辑立即生效。

## 5. 报名 / 分班接口
- `GET /api/enrollments/pending`  
  支持按 `term_id`（默认激活学期）、`campus_id`、`homeroom`、`club`、`weekday`、`student_name` 筛选待分班记录，返回包含学生姓名、班级、校区、社团、星期的详细列表。
- `GET /api/enrollments/summary`  
  返回按“校区 + 社团 + 星期”聚合的报名数量，用于横向对比各社团热度；同样支持 `term_id`/`campus_id` 过滤。
- `GET /api/enrollments/slots`  
  必填 `campus_id`、`club_id`、`weekday`，可选 `term_id`（默认激活学期）。返回指定“校区/社团/星期”下所有报名学生的详情（含 `PENDING` 与 `ACTIVE` 状态），并附带 `class_id`/`class_code` 字段用于展示当前分班结果。
- `POST /api/enrollments/status`：批量更新报名状态（当前仍为占位实现，后续补充真实逻辑）。
- `GET /api/classes`  
  查询当前学期在指定 `campus_id` + `club_id` + `weekday` 下的班级列表，返回 `class_code`、起止时间、地点、容量及实时 `assigned_count`。
- `POST /api/classes`  
  传入 `{ campus_id, club_id, weekday, class_code, start_time, end_time, location?, capacity? }` 新建班级，`start/end_time` 需使用 `HH:MM` 文本。
- `PUT /api/classes/{id}`  
  与 POST 相同字段，用于更新既有班级（仅允许在当前筛选组合内编辑），响应返回最新 `assigned_count`。
- `POST /api/classes/assign`  
  请求体：`{ campus_id, club_id, weekday, class_id?, enrollment_ids[] }`。若 `class_id` 为 `null` 表示撤销分班，接口会同步更新报名状态为 `PENDING` 并返回 `updated` 数量。

## 6. 考勤与报表占位接口
- `POST /api/attendance/bulk`、`POST /api/attendance/template/{class_id}`：考勤上传/模板生成占位。
- `GET /api/reports/settlement`、`GET /api/reports/billing`：费用/账单报表占位。

## 7. 调试提示
1. 启动顺序：`docker compose up -d db` → `cd backend && cargo run`（必要时启动前端进行联调）。  
2. 通过 `docker compose exec db psql -U admin -d club_management -c "SELECT * FROM import_jobs ORDER BY created_at DESC LIMIT 5;"` 检查导入任务。  
3. 若需重置数据库，参考 `README.md` 的“数据库重建（开发环境）”章节。
