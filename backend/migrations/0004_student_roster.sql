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
                   WHERE date_part(year, t.start_date) = h.academic_year
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
BEGIN
    IF EXISTS (SELECT 1 FROM homerooms WHERE term_id IS NULL) THEN
        RAISE EXCEPTION 无法为所有
