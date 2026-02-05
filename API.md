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
  上传“校区/班级/姓名” Excel（表头占第一行）。服务会自动匹配 `campuses.code/name`、创建/更新 `homerooms`，并写入 `students`；响应包含导入汇总（total/success/skipped/errors）。
- `POST /api/import/enrollments`  
  上传问卷星报名 Excel（列1为“年级班级姓名”，列2~6 为周一~周五社团）。返回逐行 `EnrollmentImportOutcome`，并在 `import_jobs` 中记录任务。

## 5. 报名 / 分班占位接口
- `GET /api/enrollments/pending`：待分班列表（当前返回空数组，占位）。
- `POST /api/enrollments/status`：批量更新报名状态（当前直接返回 202，占位）。
- `GET /api/classes/pending` / `POST /api/classes/assign`：班级分配占位接口，等待真实实现。

## 6. 考勤与报表占位接口
- `POST /api/attendance/bulk`、`POST /api/attendance/template/{class_id}`：考勤上传/模板生成占位。
- `GET /api/reports/settlement`、`GET /api/reports/billing`：费用/账单报表占位。

## 7. 调试提示
1. 启动顺序：`docker compose up -d db` → `cd backend && cargo run`（必要时启动前端进行联调）。  
2. 通过 `docker compose exec db psql -U admin -d club_management -c "SELECT * FROM import_jobs ORDER BY created_at DESC LIMIT 5;"` 检查导入任务。  
3. 若需重置数据库，参考 `README.md` 的“数据库重建（开发环境）”章节。
