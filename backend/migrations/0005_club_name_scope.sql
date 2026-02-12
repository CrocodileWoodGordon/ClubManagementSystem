-- Allow club names to repeat across campuses/weekday combinations.
ALTER TABLE clubs
    DROP CONSTRAINT IF EXISTS clubs_name_key;
