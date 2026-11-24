DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM schools WHERE latitude IS NULL OR longitude IS NULL) THEN
        RAISE EXCEPTION 'School coordinates must be verified and completed before applying this migration';
    END IF;
END;
$$;

ALTER TABLE schools ALTER COLUMN latitude SET NOT NULL;
ALTER TABLE schools ALTER COLUMN longitude SET NOT NULL;
