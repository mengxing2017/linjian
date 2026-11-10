CREATE TABLE home_menu_imports (
 id UUID PRIMARY KEY, school_id UUID NOT NULL REFERENCES schools(id), actor_id UUID NOT NULL REFERENCES accounts(id),
 digest TEXT NOT NULL, file_digest TEXT NOT NULL, catalog_revision BIGINT NOT NULL, rows JSONB NOT NULL, errors JSONB NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(), expires_at TIMESTAMPTZ NOT NULL DEFAULT now() + interval '30 minutes',
 result JSONB, committed_at TIMESTAMPTZ
);
