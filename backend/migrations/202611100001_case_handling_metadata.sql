ALTER TABLE cases ADD COLUMN handled_at TIMESTAMPTZ NULL,
    ADD COLUMN handled_by UUID NULL REFERENCES accounts(id),
    ADD CONSTRAINT cases_handling_metadata CHECK (
        (handled_at IS NULL) = (handled_by IS NULL)
        AND (state <> 'open' OR handled_at IS NULL)
    );
