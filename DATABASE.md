# 数据库设计（Club Management System）

## 1. 设计目标与原则
- **完整支撑业务**：覆盖报名导入、分班、考勤、换课/退课与结算的全流程数据。 
- **第三范式**：将“学期、学生、社团、班级、报名、考勤、结算”拆分为独立实体，并通过外键显式关联，避免字段冗余导致的更新异常。 
- **可审计**：所有批量导入、状态流转、结算批处理均有独立表记录，便于追溯。 
- **可扩展**：通过 term（学期）维度与枚举/配置表，保证未来学期、社团扩充时无需重构。 

> 说明：下文所有 `uuid` 类型推荐使用 `uuid_generate_v4()`，金额字段统一 `numeric(10,2)`，时间戳为 `timestamptz`。

## 2. 实体与表结构

### 2.1 全局与学期维度
| 表名 | 说明 |
| --- | --- |
| `campuses` | 校区主数据，描述不同校区的代码、名称、地址。 |
| `terms` | 学期/学段定义，所有业务数据均关联 term_id，便于按学期隔离。 |
| `homerooms` | 年级-班级定义。学生指向 homeroom，Excel 解析时以 `(homeroom.display_name, student.name)` 做匹配。 |

#### `campuses`
| 字段 | 类型 | 约束 | 备注 |
| --- | --- | --- | --- |
| `id` | uuid | PK | |
| `code` | text | UNIQUE NOT NULL | 例如 `main_campus`、`east` |
| `name` | text | NOT NULL | 校区中文名 |
| `short_name` | text | NULLABLE | |
| `address` | text | NULLABLE | |
| `contact_name` | text | NULLABLE | |
| `contact_phone` | text | NULLABLE | |
| `created_at` | timestamptz | default now() | |
| `updated_at` | timestamptz | default now() | trigger 自动更新时间 |

#### `terms`
| 字段 | 类型 | 约束 | 备注 |
| --- | --- | --- | --- |
| `id` | uuid | PK | |
| `code` | text | UNIQUE NOT NULL | 如 `2025_spring` |
| `name` | text | NOT NULL | 显示名称 |
| `start_date` | date | NOT NULL | |
| `end_date` | date | NOT NULL | |
| `enrollment_start` | date | NOT NULL | |
| `enrollment_end` | date | NOT NULL | |
| `is_active` | boolean | default true | 同时只允许一个 active term (可用部分索引约束) |
| `created_at` | timestamptz | default now() | |
| `updated_at` | timestamptz | default now() | trigger 自动更新时间 |

#### `homerooms`
| 字段 | 类型 | 约束 | 备注 |
| --- | --- | --- | --- |
| `id` | uuid | PK | |
| `term_id` | uuid | FK → terms.id ON DELETE CASCADE | 绑定具体学期，支持多学期并行维护 |
| `campus_id` | uuid | FK → campuses.id ON DELETE RESTRICT | 表示该年级班级所属校区 |
| `academic_year` | smallint | NOT NULL | 学年，如 2024（默认取 term.start_date 年份） |
| `grade_label` | text | NOT NULL | “三年级” |
| `class_label` | text | NOT NULL | “2 班” |
| `display_name` | text | NOT NULL | 例如 “三2班” |
| `head_teacher_name` | text | NULLABLE | 班主任 |
| `head_teacher_phone` | text | NULLABLE | 班主任电话 |
| `notes` | text | NULLABLE | 班级备注 |
| `created_at` / `updated_at` | timestamptz | default now() | updated_at 由 trigger 维护 |
| 组合唯一 | (`term_id`, `campus_id`, `display_name`) | 同一学期同校区内不可重复 |

### 2.2 基础主数据
| 表名 | 说明 |
| --- | --- |
| `students` | 学生基础信息。 |
| `clubs` | 社团定义与默认收费规则。 |
| `club_terms` | 社团在某学期的配置（可覆盖材料费、课时费、容量等）。 |

#### `students`
| 字段 | 类型 | 约束 | 备注 |
| --- | --- | --- | --- |
| `id` | uuid | PK | |
| `student_code` | text | UNIQUE NULLABLE | 可用于 Excel 精确匹配 |
| `full_name` | text | NOT NULL | |
| `homeroom_id` | uuid | FK → homerooms.id ON DELETE RESTRICT | |
| `is_teacher_child` | boolean | default false | |
| `status` | text | CHECK IN (ACTIVE,INACTIVE) | 默认为 ACTIVE |
| `primary_guardian_name` | text | NULLABLE | |
| `primary_guardian_phone` | text | NULLABLE | |
| `created_at` / `updated_at` | timestamptz | default now() | |
| 组合唯一 | (`homeroom_id`, `full_name`) where status=ACTIVE | 避免同班同名重复 |

#### `clubs`
| 字段 | 类型 | 约束 | 备注 |
| --- | --- | --- | --- |
| `id` | uuid | PK | |
| `code` | text | UNIQUE NOT NULL | “robotics” |
| `name` | text | NOT NULL | 允许跨校区/星期重名，冲突由业务校验 |
| `description` | text | | |
| `material_fee` | numeric(10,2) | NOT NULL | 默认材料费 |
| `price_per_session` | numeric(10,2) | NOT NULL | 默认课时费 |
| `grace_sessions` | smallint | default 3 | 退课免计费课次（默认三节） |
| `created_at` | timestamptz | default now() | |

#### `club_terms`
| 字段 | 类型 | 约束 | 备注 |
| --- | --- | --- | --- |
| `id` | uuid | PK | |
| `term_id` | uuid | FK → terms.id ON DELETE CASCADE | |
| `campus_id` | uuid | FK → campuses.id ON DELETE RESTRICT | 表示该社团配置适用的校区 |
| `club_id` | uuid | FK → clubs.id ON DELETE CASCADE | |
| `material_fee` | numeric(10,2) | default clubs.material_fee | 学期覆盖 |
| `price_per_session` | numeric(10,2) | default clubs.price_per_session | |
| `capacity` | integer | NULLABLE | 整体容量上限 |
| `notes` | text | | |
| UNIQUE(term_id, campus_id, club_id) | | 同一学期同一校区的社团配置唯一 |

### 2.3 排课相关
| 表名 | 说明 |
| --- | --- |
| `classes` | 具体班级（term + club + weekday + 编号）。 |
| `class_meetings` | 某班级的每一次上课会话，用于考勤。 |

#### `classes`
| 字段 | 类型 | 约束 | 备注 |
| --- | --- | --- | --- |
| `id` | uuid | PK | |
| `term_id` | uuid | FK → terms.id ON DELETE CASCADE | |
| `campus_id` | uuid | FK → campuses.id ON DELETE RESTRICT | |
| `club_id` | uuid | FK → clubs.id | |
| `class_code` | text | NOT NULL | “1 班”、“A 组” |
| `weekday` | smallint | CHECK 1-7 | 1=周一 |
| `start_time` / `end_time` | time | NOT NULL | |
| `location` | text | NULLABLE | |
| `capacity` | integer | NULLABLE | |
| `status` | text | CHECK IN (PLANNED,ACTIVE,ARCHIVED) | |
| `notes` | text | | |
| UNIQUE(term_id, campus_id, club_id, class_code) | | |
| INDEX(term_id, campus_id, club_id, weekday) | | 筛选待分班数据 |

#### `class_meetings`
| 字段 | 类型 | 约束 | 备注 |
| --- | --- | --- | --- |
| `id` | uuid | PK | |
| `class_id` | uuid | FK → classes.id ON DELETE CASCADE | |
| `meeting_date` | date | NOT NULL | |
| `session_number` | smallint | NOT NULL | 从 1 开始 |
| `state` | text | CHECK IN (PLANNED,LOCKED) | 锁定后考勤不可更改 |
| `topic` | text | NULLABLE | |
| UNIQUE(class_id, meeting_date) | | |

### 2.4 报名导入与状态流转
| 表名 | 说明 |
| --- | --- |
| `import_jobs` | 记录 Excel 导入任务。 |
| `import_job_errors` | 导入失败行详情。 |
| `enrollments` | 选择社团的主记录。 |
| `enrollment_status_history` | 状态机轨迹（报名/分班/退课/换课）。 |

#### `import_jobs`
| 字段 | 类型 | 约束 |
| --- | --- | --- |
| `id` | uuid | PK |
| `term_id` | uuid | FK → terms.id |
| `job_type` | text | CHECK IN (STUDENTS,ENROLLMENTS,ATTENDANCE) |
| `source_filename` | text | |
| `total_rows` / `success_rows` | integer | |
| `status` | text | CHECK IN (PENDING,PROCESSING,FAILED,COMPLETED) |
| `created_by` | text | |
| `created_at` | timestamptz | default now() |
| `finished_at` | timestamptz | |

#### `import_job_errors`
| 字段 | 类型 | 约束 |
| --- | --- | --- |
| `id` | uuid | PK |
| `job_id` | uuid | FK → import_jobs.id ON DELETE CASCADE |
| `row_number` | integer | |
| `column_name` | text | |
| `error_message` | text | |
| `raw_payload` | jsonb | 原始行数据 |

#### `import_placeholder_sets`
| 字段 | 类型 | 约束 | 备注 |
| --- | --- | --- | --- |
| `id` | uuid | PK | |
| `import_type` | text | UNIQUE NOT NULL | 目前支持 `ENROLLMENTS`、`STUDENTS` |
| `placeholders` | text[] | NOT NULL | 存储去重后的占位字符串 |
| `updated_by` | text | | |
| `updated_at` | timestamptz | default now() | |

> 默认占位集合会预置 `-`、`(空)`、`(跳过)` 等字符串，界面层可通过 API 自定义增删。

#### `enrollments`
| 字段 | 类型 | 约束 | 备注 |
| --- | --- | --- | --- |
| `id` | uuid | PK | |
| `term_id` | uuid | FK → terms.id | |
| `campus_id` | uuid | FK → campuses.id | Excel 导入时根据学生所在校区赋值 |
| `student_id` | uuid | FK → students.id | |
| `club_id` | uuid | FK → clubs.id | |
| `requested_weekday` | smallint | CHECK 1-7 | 与 Excel 列对应 |
| `class_id` | uuid | FK → classes.id NULLABLE | 分班后填写 |
| `import_job_id` | uuid | FK → import_jobs.id NULLABLE | |
| `status` | text | CHECK IN (PENDING,ACTIVE,DROPPED,TRANSFERRED_OUT,TRANSFERRED_IN) | |
| `status_reason` | text | |
| `drop_date` | date | NULLABLE | |
| `transferred_from_id` | uuid | FK → enrollments.id NULLABLE | 指向旧记录 |
| `material_fee_state` | text | CHECK IN (UNSET,CHARGED,REFUNDED) | 控制材料费是否重复收取 |
| `tuition_grace_applied` | boolean | default false | 三节课免课时费是否已用 |
| `created_at` / `updated_at` | timestamptz | default now() | |
| 索引 | (`term_id`, `campus_id`, `status`, `requested_weekday`) | 快速筛选待分班 |
| 部分唯一索引 | UNIQUE(term_id, campus_id, student_id, club_id, requested_weekday) WHERE status IN (PENDING,ACTIVE) | 保证同校区同一时段仅一条有效报名 |

#### `enrollment_status_history`
| 字段 | 类型 | 约束 |
| --- | --- | --- |
| `id` | uuid | PK |
| `enrollment_id` | uuid | FK → enrollments.id ON DELETE CASCADE |
| `from_status` | text | |
| `to_status` | text | |
| `changed_by` | text | |
| `changed_at` | timestamptz | default now() |
| `note` | text | |

### 2.5 考勤
| 表名 | 说明 |
| --- | --- |
| `attendance_records` | 每节课的学生出勤状态。 |

#### `attendance_records`
| 字段 | 类型 | 约束 |
| --- | --- | --- |
| `id` | uuid | PK |
| `class_meeting_id` | uuid | FK → class_meetings.id ON DELETE CASCADE |
| `enrollment_id` | uuid | FK → enrollments.id ON DELETE RESTRICT | 退课后依然关联 |
| `status` | text | CHECK IN (PRESENT,ABSENT,EXCUSED,LEAVE) |
| `minutes_attended` | integer | NULLABLE |
| `recorded_by` | text | |
| `recorded_at` | timestamptz | default now() |
| UNIQUE(class_meeting_id, enrollment_id) | | 防止重复写入 |

### 2.6 结算与报表
| 表名 | 说明 |
| --- | --- |
| `billing_runs` | 结算批处理，记录一次费用计算。 |
| `billing_items` | 按 enrollment 生成的费用项（课时费/材料费/调整）。 |

#### `billing_runs`
| 字段 | 类型 | 约束 |
| --- | --- | --- |
| `id` | uuid | PK |
| `term_id` | uuid | FK → terms.id |
| `run_type` | text | CHECK IN (PREVIEW,FINAL) |
| `status` | text | CHECK IN (PENDING,RUNNING,FAILED,COMPLETED) |
| `triggered_by` | text | |
| `started_at` / `completed_at` | timestamptz | |
| `notes` | text | |

#### `billing_items`
| 字段 | 类型 | 约束 | 备注 |
| --- | --- | --- | --- |
| `id` | uuid | PK | |
| `billing_run_id` | uuid | FK → billing_runs.id ON DELETE CASCADE | |
| `enrollment_id` | uuid | FK → enrollments.id | |
| `item_type` | text | CHECK IN (TUITION,MATERIAL,ADJUSTMENT) | |
| `quantity` | numeric(10,2) | NOT NULL | 课时时数或次数 |
| `unit_amount` | numeric(10,2) | NOT NULL | |
| `total_amount` | numeric(10,2) | NOT NULL | quantity × unit |
| `source_attendance` | integer | NULLABLE | 课时数快照 |
| `policy_snapshot` | jsonb | 记录当时的优惠/规则 |
| `note` | text | |
| 索引 | (`billing_run_id`, `item_type`) | 报表查询 |

### 2.7 其他支撑表
- `files`: （可选）存储生成的报表文件 metadata。字段：`id`, `owner_type`, `owner_id`, `file_name`, `mime_type`, `path`, `created_at`。
- `task_logs`: 记录长任务执行日志（考勤表生成/导出）。若实现 `src/tasks` 需要，可包含 `task_type`, `payload`, `status`, `started_at`, `finished_at`, `result`。

## 3. 业务映射
1. **Excel 报名导入**：导入结果写入 `import_jobs`/`import_job_errors`，成功行创建 `enrollments`（status = `PENDING`，`class_id` 空，按列写入 `requested_weekday`），并根据匹配到的学生 `homeroom.campus_id` 自动填写 `campus_id`。
2. **分班**：前端筛选基于 `enrollments` 的 `(term_id, campus_id, club_id, requested_weekday, status=PENDING)`。批量操作创建/更新 `classes`（同样包含 `campus_id`）后，将选中 enrollment 的 `class_id`、`status` 设为 `ACTIVE`。
3. **考勤**：由 `classes` 生成 `class_meetings`，导入明细写入 `attendance_records`。即便退课，`attendance_records` 仍引用原 enrollment。
4. **换课/退课**：`enrollments` 的 `status` 切换并写入 `enrollment_status_history`，如换社团则创建新 enrollment，`transferred_from_id` 指向旧记录并沿用 `material_fee_state`。同社团换班只更新 `class_id`。
5. **结算**：触发 `billing_runs`，遍历 `enrollments` + `attendance_records` 生成 `billing_items`。教师子女 (`students.is_teacher_child`) 与三节课免课 (`clubs.grace_sessions`) 的逻辑写入 `policy_snapshot` 以备审计。

## 4. 索引与约束建议
- 为所有外键列建立 BTree 索引（如 `enrollments.student_id`, `attendance_records.class_meeting_id`）。
- 使用部分索引保证唯一性：`CREATE UNIQUE INDEX ux_active_enrollment ON enrollments(term_id, campus_id, student_id, club_id, requested_weekday) WHERE status IN (PENDING,ACTIVE);`
- 对 `attendance_records` 增加 `WHERE status=PRESENT` 的部分索引用于统计。
- `billing_items` 可创建 `GIN(policy_snapshot)` 以支撑 JSON 搜索（可选）。

## 5. 实施顺序建议
1. 先建 `campuses`、`terms`、`homerooms`、`students`、`clubs`、`classes` 等基础表。 
2. 随后创建 `enrollments`、`import_jobs` 等报名链路。 
3. 再补充 `class_meetings`、`attendance_records`。 
4. 最后实现 `billing_runs`、`billing_items` 等结算结构。

该设计可通过 SQLx migration（步骤 2）逐步落地；实现时按照 “先 reference，再 transactional” 的顺序，以确保外键引用均已存在。
