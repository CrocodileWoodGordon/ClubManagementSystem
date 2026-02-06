
---

# 社团管理系统 (Club Management System) 项目设计文档

## 1. 项目概况 (Project Overview)

本项目旨在构建一个社团选课、分班、考勤及费用结算管理系统。系统需处理从报名（Excel导入）到分班（Web界面手动操作）、考勤（纸质录入）、变更（换课/退课）及最终费用结算（根据考勤和规则计算）的全流程。

* **技术栈**:
* **Frontend**: Next.js (React)
* **Backend**: Rust (Axum)
* **Database**: PostgreSQL
* **Deployment**: Docker (Local deployment on Windows/Linux)


* **核心用户**: 管理员（用户父亲），负责数据导入、分班及导出报表。
* **运营模式**: 同一学期包含多个校区（例如主校区、东校区），每个校区独立排班/结算，因此核心业务实体（年级、班级、报名、社团开课配置）都必须带 `campus_id` 维度。

---

## 2. 业务流程 (Business Workflow)

### 2.1 阶段一：报名数据初始化 (Enrollment Initialization)

1. **基础数据准备**: 通过“学生名单 Excel 导入”接口上传“校区/班级/姓名”三列，系统会为缺失的 `homerooms` 建档并写入 `students`，学年默认使用当前激活学期的 `start_date` 年份。
2. **数据导入**: 管理员上传问卷星导出的 Excel。
* **输入格式**: 默认读取 Excel E 列为“班级+姓名”组合（如 `302张三`），H~L 列对应周一到周五的社团填报；后台可通过 `config` JSON 指定任意列映射，表头占第一行。
* **社团建档**: 导入过程中若发现未知社团，会自动在 `clubs`、`club_terms` 中建档，并以“社团名 + 校区 + 星期”组合统计报名量，免去手工维护。
* **处理逻辑**:
* 解析“年级班级姓名”匹配系统内学生 ID。
* 根据社团名称匹配系统内社团 ID。
* 生成初始 **Enrollment (报名记录)**。
* 所有报名记录的初始班级状态为 **"待定班" (Pending)**。
* 支持按班级/社团/校区/星期筛选报名数据，并提供“社团 x 校区 x 星期”汇总，用于横向对比报名热度。





### 2.2 阶段二：智能/手动分班 (Class Assignment)

* **操作界面**:
* 按“校区 + 社团 + 星期”筛选（例如：显示“主校区 · 周一 机器人班”的所有 150 名学生）。
* **列表显示**: 学生姓名、原班级、当前社团班级编号（默认为空/待定）。
* **批量操作**: 支持勾选多名学生 -> 输入班级编号 (如 1, 2, 3) -> 提交。


* **后端逻辑**:
* 创建具体的 **Class (班级实例)** 对象（关联社团、星期、班级编号）。
* 存储该班级的元数据：**上课时间**、**上课地点**（支持同一社团不同班级在不同时间，但地点通常一致）。
* 更新学生 Enrollment 中的 `class_id`。



### 2.3 阶段三：日常运营 (Operations)

* **考勤表生成**:
* 针对每个已分好的班级，生成 Excel/PDF 考勤表（包含学生名单、日期列）。
* 表单留空，供老师线下填写。


* **变更管理 (核心难点)**:
* **退课**: 标记 Enrollment 状态为 `DROPPED`，记录退课时间/课次。
* **换课**:
1. 将原课程 Enrollment 标记为 `DROPPED` (或 `TRANSFERRED_OUT`)。
2. 创建新课程 Enrollment，状态为 `ACTIVE`。


* **数据一致性**: 确保换课后，旧课的已产生考勤保留（用于费用判断），新课名单即时更新。



### 2.4 阶段四：期末结算 (Settlement)

1. **考勤录入**: 期末回收考勤表，管理员统一录入/导入数据库。
2. **费用计算**:
* 触发计费脚本，遍历所有学生。
* 应用“教师子女”、“退课三课时规则”等逻辑。


3. **报表导出**:
* 班级维度：学生名单、出勤率、收费明细。
* 个人维度：详细账单（上了什么课、几次、材料费、学费）。



---

## 3. 数据库设计 (Database Schema Design)

### 3.1 核心实体表

#### `campuses` (校区表)

* `id`: UUID
* `code`: String（如 `main_campus`）
* `name`: String（中文名）
* `address` / `contact`: 可选字段，方便报表展示
* **说明**: 所有与教学地点相关的实体（年级班级、社团开课配置、具体班级、报名记录）都需要指向某个 `campus_id`，以支撑“每学期每个校区都开社团”的管理模式。

#### `students` (学生表)

* `id`: UUID
* `original_class`: String (e.g., "3年2班")
* `name`: String
* `is_teacher_child`: Boolean (是否教师子女)
* `校区归属`: 通过其所属 `homeroom.campus_id` 推断（Excel 导入时按照该校区筛选可报社团）
* *Index*: `(original_class, name)` 用于 Excel 解析匹配。

#### `clubs` (社团定义表)

* `id`: UUID
* `name`: String (e.g., "机器人")
* `material_fee`: Decimal (材料费，不退不免)
* `price_per_lesson`: Decimal (单节课时费)

#### `classes` (具体班级实例表)

* `id`: UUID
* `campus_id`: FK -> campuses.id（同一社团在不同校区可重名）
* `club_id`: FK -> clubs.id
* `day_of_week`: Integer (1-5)
* `batch_number`: String (班级编号，e.g., "1班", "2班")
* `time_slot`: String (e.g., "16:00-17:30")
* `location`: String (e.g., "科技楼301")
* *Note*: 这是实际分班后的实体，同一 `class_code` 只在同一校区+学期内唯一。

#### `enrollments` (选课/报名表)

* `id`: UUID
* `campus_id`: FK -> campuses.id（Excel 导入时根据学生校区填入）
* `student_id`: FK -> students.id
* `class_id`: FK -> classes.id (分班前可能指向一个虚拟的"待定"班级或允许 NULL)
* `status`: Enum (`PENDING`, `ACTIVE`, `DROPPED`, `TRANSFERRED`)
* `drop_date`: Date (如果有退换课)
* `created_at`: Timestamp

#### `attendance_records` (考勤记录表)

* `id`: UUID
* `student_id`: FK -> students.id
* `class_id`: FK -> classes.id
* `date`: Date (上课日期)
* `status`: Enum (`PRESENT`, `ABSENT`, `EXCUSED`)
* *Note*: 即使退课，之前的考勤记录必须保留。

---

## 4. 核心业务逻辑详解 (Core Business Logic)

### 4.1 费用计算规则 (Fee Calculation Algorithm)

对于每个学生  的每个报名记录 ：

1. **基础参数获取**:
* : 来自 `clubs` 表。
* : 来自 `clubs` 表。
* : `attendance_records` 中该学生在该班级的 `PRESENT` 次数。
* : `students.is_teacher_child`。


2. **课时费 () 计算**:
* **情况 A: 教师子女**
* 


* **情况 B: 普通学生**
* **子逻辑: 退课判定**
* 如果是正常结课（未退课），。
* 如果是中途退课/换课 ()：
* 统计**退课前**产生的实际考勤次数 。
* **规则**:
* 若 :  (三课时内退课免课时费)。
* 若 :  (按实际出勤计费)。










3. **总费用 ()**:
* 
* *注意*: 材料费 () 始终收取，不退不免（除非完全未产生报名，但业务逻辑指一旦报名即产生材料费，需确认是否只要报名就收，还是出席1次才收。目前按“不退不免”理解为只要报名记录存在且未被管理员物理删除即收取）。



### 4.2 换课数据一致性 (Change Course Consistency)

当学生从 A 班换到 B 班：

1. **A 班处理**:
* 更新 A 班 Enrollment 状态为 `DROPPED`。
* 系统需保留 A 班已有的考勤记录（用于判断是否 >3 节课）。


2. **B 班处理**:
* 创建新的 Enrollment 指向 B 班。
* 继承学生基础信息。
* 材料费处理：*需确认*。通常换课如果不涉及社团变更（同社团换班），材料费不重复收；如果换社团，A社团材料费不退，B社团材料费需新交。
* *假定逻辑*: 换社团视为“A退课 + B新报”。



---

## 5. 接口与功能规划 (API & Functionality)

### 5.1 数据导入 API

* `POST /api/import/students`: 导入全校学生库。
* `POST /api/import/enrollments`: 解析问卷星 Excel，批量创建 Enrollments。

### 5.2 分班管理 API

* `GET /api/classes/pending`: 获取某社团、某日的待分班学生列表。
* `POST /api/classes/assign`: 批量分班。
* Body: `{ student_ids: [1, 2...], target_class_batch: "1", time: "16:00", location: "Room A" }`



### 5.3 考勤与报表 API

* `POST /api/attendance/bulk`: 批量上传考勤数据。
* `GET /api/reports/settlement`: 获取最终结算单。
* Output: JSON 结构，前端渲染为表格并支持导出 Excel。



---

## 6. 特殊情况与边界处理 (Edge Cases)

1. **同名同姓**:
* 依赖 `(年级, 班级, 姓名)` 三元组作为唯一键。如果同年级同班有同名，需提示管理员手动修正 Excel 源数据（如标记 张三A, 张三B）。


2. **多次换课**:
* 学生可能 A -> B -> C。系统应生成 3 条 Enrollment 记录，分别计算费用后求和。


3. **Excel 格式脏数据**:
* 解析器需具备容错能力（如去除空格、全角转半角）。无法匹配的学生放入“异常数据池”供管理员人工修正。


4. **中途加入**:
* 报名时间晚于开学。费用计算逻辑依然基于 `AttendanceCount`，天然支持中途加入的情况（只算来了的次数）。



---

## 7. 部署架构 (Deployment)

* **Docker Compose**:
* Service 1: `postgres` (数据持久化挂载到本地磁盘)。
* Service 2: `backend` (Rust binary)。
* Service 3: `frontend` (Next.js standalone output)。


* **本地化**: 无需云服务器，直接在管理员电脑安装 Docker Desktop 启动即可。数据文件映射到宿主机文件夹，方便备份。

---

**End of Document**

### 下一步建议

这份文档已经非常清晰地梳理了业务逻辑。为了让你能够顺利开始开发，我建议的下一步是：

**你想让我先帮你写出 PostgreSQL 的详细建表 SQL 语句（包含索引和约束），还是先帮你写那个“解析问卷星 Excel”的 Rust 核心逻辑代码？**
