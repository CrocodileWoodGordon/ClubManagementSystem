CREATE TABLE import_placeholder_sets (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    import_type text NOT NULL UNIQUE,
    placeholders text[] NOT NULL DEFAULT ARRAY[]::text[],
    updated_by text,
    updated_at timestamptz NOT NULL DEFAULT now()
);

INSERT INTO import_placeholder_sets (import_type, placeholders, updated_by)
VALUES
    ('ENROLLMENTS', ARRAY['-','—','——','无','N/A','n/a','NA','na','(空)','（空）'], 'system');
