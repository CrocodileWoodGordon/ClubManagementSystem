-- 0001_init.sql
-- 核心 schema（对齐 DATABASE.md）

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 1. 学期、班级等基础维度 --------------------------------------------------
CREATE TABLE terms (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    code text NOT NULL UNIQUE,
    name text NOT NULL,
    start_date date NOT NULL,
    end_date date NOT NULL,
    enrollment_start date NOT NULL,
    enrollment_end date NOT NULL,
    is_active boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX ux_terms_active ON terms (is_active) WHERE is_active;

CREATE TRIGGER trg_terms_updated_at
BEFORE UPDATE ON terms
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE campuses (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    code text NOT NULL UNIQUE,
    name text NOT NULL,
    short_name text,
    address text,
    contact_name text,
    contact_phone text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TRIGGER trg_campuses_updated_at
BEFORE UPDATE ON campuses
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE homerooms (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    campus_id uuid NOT NULL REFERENCES campuses(id) ON DELETE RESTRICT,
    academic_year smallint NOT NULL,
    grade_label text NOT NULL,
    class_label text NOT NULL,
    display_name text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX ux_homerooms_campus_year_display
    ON homerooms (campus_id, academic_year, display_name);

-- 2. 学生、社团定义 ---------------------------------------------------------
CREATE TABLE students (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    student_code text UNIQUE,
    full_name text NOT NULL,
    homeroom_id uuid NOT NULL REFERENCES homerooms(id) ON DELETE RESTRICT,
    is_teacher_child boolean NOT NULL DEFAULT false,
    status text NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE','INACTIVE')),
    primary_guardian_name text,
    primary_guardian_phone text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX ux_students_active_name
    ON students (homeroom_id, full_name)
    WHERE status = 'ACTIVE';

CREATE TRIGGER trg_students_updated_at
BEFORE UPDATE ON students
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE clubs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    code text NOT NULL UNIQUE,
    name text NOT NULL UNIQUE,
    description text,
    material_fee numeric(10,2) NOT NULL DEFAULT 0,
    price_per_session numeric(10,2) NOT NULL DEFAULT 0,
    grace_sessions smallint NOT NULL DEFAULT 3,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE club_terms (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    term_id uuid NOT NULL REFERENCES terms(id) ON DELETE CASCADE,
    campus_id uuid NOT NULL REFERENCES campuses(id) ON DELETE RESTRICT,
    club_id uuid NOT NULL REFERENCES clubs(id) ON DELETE CASCADE,
    material_fee numeric(10,2) NOT NULL DEFAULT 0,
    price_per_session numeric(10,2) NOT NULL DEFAULT 0,
    capacity integer,
    notes text,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (term_id, campus_id, club_id)
);

-- 3. 班级与排课 -------------------------------------------------------------
CREATE TABLE classes (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    term_id uuid NOT NULL REFERENCES terms(id) ON DELETE CASCADE,
    campus_id uuid NOT NULL REFERENCES campuses(id) ON DELETE RESTRICT,
    club_id uuid NOT NULL REFERENCES clubs(id) ON DELETE RESTRICT,
    class_code text NOT NULL,
    weekday smallint NOT NULL CHECK (weekday BETWEEN 1 AND 7),
    start_time time NOT NULL,
    end_time time NOT NULL,
    location text,
    capacity integer,
    status text NOT NULL DEFAULT 'PLANNED' CHECK (status IN ('PLANNED','ACTIVE','ARCHIVED')),
    notes text,
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (start_time < end_time),
    UNIQUE (term_id, campus_id, club_id, class_code)
);

CREATE INDEX idx_classes_lookup ON classes (term_id, campus_id, club_id, weekday);

CREATE TABLE class_meetings (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    class_id uuid NOT NULL REFERENCES classes(id) ON DELETE CASCADE,
    meeting_date date NOT NULL,
    session_number smallint NOT NULL CHECK (session_number > 0),
    state text NOT NULL DEFAULT 'PLANNED' CHECK (state IN ('PLANNED','LOCKED')),
    topic text,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (class_id, meeting_date)
);

-- 4. 导入任务与报名 ---------------------------------------------------------
CREATE TABLE import_jobs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    term_id uuid REFERENCES terms(id) ON DELETE SET NULL,
    job_type text NOT NULL CHECK (job_type IN ('STUDENTS','ENROLLMENTS','ATTENDANCE')),
    source_filename text,
    total_rows integer NOT NULL DEFAULT 0 CHECK (total_rows >= 0),
    success_rows integer NOT NULL DEFAULT 0 CHECK (success_rows >= 0),
    status text NOT NULL DEFAULT 'PENDING' CHECK (status IN ('PENDING','PROCESSING','FAILED','COMPLETED')),
    created_by text,
    created_at timestamptz NOT NULL DEFAULT now(),
    finished_at timestamptz
);

CREATE INDEX idx_import_jobs_term ON import_jobs (term_id, job_type);

CREATE TABLE import_job_errors (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id uuid NOT NULL REFERENCES import_jobs(id) ON DELETE CASCADE,
    row_number integer,
    column_name text,
    error_message text NOT NULL,
    raw_payload jsonb,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_import_job_errors_job_id ON import_job_errors (job_id);

CREATE TABLE enrollments (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    term_id uuid NOT NULL REFERENCES terms(id) ON DELETE CASCADE,
    campus_id uuid NOT NULL REFERENCES campuses(id) ON DELETE RESTRICT,
    student_id uuid NOT NULL REFERENCES students(id) ON DELETE RESTRICT,
    club_id uuid NOT NULL REFERENCES clubs(id) ON DELETE RESTRICT,
    requested_weekday smallint NOT NULL CHECK (requested_weekday BETWEEN 1 AND 7),
    class_id uuid REFERENCES classes(id) ON DELETE SET NULL,
    import_job_id uuid REFERENCES import_jobs(id) ON DELETE SET NULL,
    status text NOT NULL DEFAULT 'PENDING' CHECK (status IN ('PENDING','ACTIVE','DROPPED','TRANSFERRED_OUT','TRANSFERRED_IN')),
    status_reason text,
    drop_date date,
    transferred_from_id uuid REFERENCES enrollments(id) ON DELETE SET NULL,
    material_fee_state text NOT NULL DEFAULT 'UNSET' CHECK (material_fee_state IN ('UNSET','CHARGED','REFUNDED')),
    tuition_grace_applied boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_enrollments_lookup ON enrollments (term_id, campus_id, status, requested_weekday);
CREATE INDEX idx_enrollments_class ON enrollments (class_id);
CREATE UNIQUE INDEX ux_enrollments_active
    ON enrollments (term_id, campus_id, student_id, club_id, requested_weekday)
    WHERE status IN ('PENDING','ACTIVE');

CREATE TRIGGER trg_enrollments_updated_at
BEFORE UPDATE ON enrollments
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE enrollment_status_history (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    enrollment_id uuid NOT NULL REFERENCES enrollments(id) ON DELETE CASCADE,
    from_status text,
    to_status text NOT NULL CHECK (to_status IN ('PENDING','ACTIVE','DROPPED','TRANSFERRED_OUT','TRANSFERRED_IN')),
    changed_by text,
    changed_at timestamptz NOT NULL DEFAULT now(),
    note text
);

CREATE INDEX idx_enrollment_status_history_enrollment ON enrollment_status_history (enrollment_id);

-- 5. 考勤 -------------------------------------------------------------------
CREATE TABLE attendance_records (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    class_meeting_id uuid NOT NULL REFERENCES class_meetings(id) ON DELETE CASCADE,
    enrollment_id uuid NOT NULL REFERENCES enrollments(id) ON DELETE RESTRICT,
    status text NOT NULL CHECK (status IN ('PRESENT','ABSENT','EXCUSED','LEAVE')),
    minutes_attended integer,
    recorded_by text,
    recorded_at timestamptz NOT NULL DEFAULT now()
);

ALTER TABLE attendance_records
    ADD CONSTRAINT uq_attendance_unique UNIQUE (class_meeting_id, enrollment_id);

CREATE INDEX idx_attendance_meeting ON attendance_records (class_meeting_id);
CREATE INDEX idx_attendance_enrollment ON attendance_records (enrollment_id);
CREATE INDEX idx_attendance_present
    ON attendance_records (enrollment_id)
    WHERE status = 'PRESENT';

-- 6. 结算 -------------------------------------------------------------------
CREATE TABLE billing_runs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    term_id uuid NOT NULL REFERENCES terms(id) ON DELETE CASCADE,
    run_type text NOT NULL CHECK (run_type IN ('PREVIEW','FINAL')),
    status text NOT NULL DEFAULT 'PENDING' CHECK (status IN ('PENDING','RUNNING','FAILED','COMPLETED')),
    triggered_by text,
    started_at timestamptz NOT NULL DEFAULT now(),
    completed_at timestamptz,
    notes text
);

CREATE TABLE billing_items (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    billing_run_id uuid NOT NULL REFERENCES billing_runs(id) ON DELETE CASCADE,
    enrollment_id uuid NOT NULL REFERENCES enrollments(id) ON DELETE RESTRICT,
    item_type text NOT NULL CHECK (item_type IN ('TUITION','MATERIAL','ADJUSTMENT')),
    quantity numeric(10,2) NOT NULL CHECK (quantity >= 0),
    unit_amount numeric(10,2) NOT NULL,
    total_amount numeric(10,2) NOT NULL,
    source_attendance integer,
    policy_snapshot jsonb,
    note text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_billing_items_run_type ON billing_items (billing_run_id, item_type);

-- 7. 支撑表 -----------------------------------------------------------------
CREATE TABLE files (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_type text NOT NULL,
    owner_id uuid NOT NULL,
    file_name text NOT NULL,
    mime_type text NOT NULL,
    storage_path text NOT NULL,
    size_bytes bigint,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_files_owner ON files (owner_type, owner_id);

CREATE TABLE task_logs (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    task_type text NOT NULL,
    payload jsonb,
    status text NOT NULL CHECK (status IN ('PENDING','RUNNING','FAILED','COMPLETED')),
    started_at timestamptz NOT NULL DEFAULT now(),
    finished_at timestamptz,
    result jsonb,
    message text
);

CREATE INDEX idx_task_logs_type_status ON task_logs (task_type, status);
