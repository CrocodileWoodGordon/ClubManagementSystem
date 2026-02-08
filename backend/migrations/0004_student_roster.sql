-- Adds per-term homeroom metadata and teacher contact fields for the student roster UI.
BEGIN;

ALTER TABLE homerooms
    ADD COLUMN term_id uuid REFERENCES terms(id) ON DELETE CASCADE,
    ADD COLUMN head_teacher_name text,
    ADD COLUMN head_teacher_phone text,
    ADD COLUMN notes text,
    ADD COLUMN updated_at timestamptz NOT NULL DEFAULT now();

WITH active_term AS (
    SELECT id
    FROM terms
    WHERE is_active = true
    ORDER BY enrollment_start DESC
    LIMIT 1
),
resolved AS (
    SELECT h.id,
           COALESCE(
               (
                   SELECT t.id
                   FROM terms t
                   WHERE date_part('year', t.start_date) = h.academic_year
                   ORDER BY t.start_date DESC
                   LIMIT 1
               ),
               (SELECT id FROM active_term)
           ) AS term_id
    FROM homerooms h
)
UPDATE homerooms h
SET term_id = r.term_id
FROM resolved r
WHERE h.id = r.id;

DO $$
DECLARE
    missing_count integer;
BEGIN
    SELECT COUNT(*) INTO missing_count
    FROM homerooms
    WHERE term_id IS NULL;

    IF missing_count > 0 THEN
        RAISE EXCEPTION '无法为所有既有班级推断 term_id，请先创建覆盖这些班级学年的学期（待补齐数量：%）', missing_count;
    END IF;
END;
$$;

ALTER TABLE homerooms
    ALTER COLUMN term_id SET NOT NULL;

DROP INDEX IF EXISTS ux_homerooms_campus_year_display;

CREATE UNIQUE INDEX IF NOT EXISTS ux_homerooms_term_campus_display
    ON homerooms (term_id, campus_id, display_name);

CREATE TRIGGER trg_homerooms_updated_at
BEFORE UPDATE ON homerooms
FOR EACH ROW EXECUTE FUNCTION set_updated_at();

COMMIT;
