-- Ensure “(跳过)” 一律作为占位值，并移除同名社团。
WITH target_clubs AS (
    SELECT id FROM clubs WHERE name = '(跳过)'
)
DELETE FROM enrollments WHERE club_id IN (SELECT id FROM target_clubs);

WITH target_clubs AS (
    SELECT id FROM clubs WHERE name = '(跳过)'
)
DELETE FROM classes WHERE club_id IN (SELECT id FROM target_clubs);

WITH target_clubs AS (
    SELECT id FROM clubs WHERE name = '(跳过)'
)
DELETE FROM clubs WHERE id IN (SELECT id FROM target_clubs);

UPDATE import_placeholder_sets s
SET placeholders = (
    SELECT ARRAY(
        SELECT DISTINCT value
        FROM (
            SELECT unnest(s.placeholders) AS value
            UNION ALL
            SELECT '(跳过)'::text
        ) merged
        ORDER BY value
    )
)
WHERE s.import_type = 'ENROLLMENTS';

INSERT INTO import_placeholder_sets (import_type, placeholders, updated_by)
SELECT 'ENROLLMENTS', ARRAY['(跳过)'], 'system'
WHERE NOT EXISTS (
    SELECT 1 FROM import_placeholder_sets WHERE import_type = 'ENROLLMENTS'
);
