-- Preserve the old resources and their publication snapshots.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM school_resources
        WHERE kind = 'tag' AND school_id IS NOT NULL AND active
          AND (starts_at IS NULL OR starts_at <= now())
          AND (ends_at IS NULL OR ends_at > now())
          AND char_length(btrim(title)) NOT BETWEEN 1 AND 50
    ) THEN
        RAISE EXCEPTION 'Cannot migrate school tags: a name is empty or exceeds 50 characters';
    END IF;
    IF EXISTS (
        SELECT 1 FROM school_resources
        WHERE kind = 'tag' AND school_id IS NOT NULL AND active
          AND (starts_at IS NULL OR starts_at <= now())
          AND (ends_at IS NULL OR ends_at > now())
        GROUP BY school_id, lower(btrim(title)) HAVING count(*) > 1
    ) THEN
        RAISE EXCEPTION 'Cannot migrate school tags: duplicate normalized names within a school';
    END IF;
END $$;

CREATE TABLE home_menu_settings (
    school_id UUID PRIMARY KEY REFERENCES schools(id),
    latest_label TEXT NOT NULL DEFAULT '最新' CHECK (char_length(btrim(latest_label)) BETWEEN 1 AND 50),
    latest_visible BOOLEAN NOT NULL DEFAULT TRUE,
    revision BIGINT NOT NULL DEFAULT 1 CHECK (revision > 0),
    catalog_revision BIGINT NOT NULL DEFAULT 1 CHECK (catalog_revision > 0)
);

CREATE TABLE home_menu_tags (
    id UUID PRIMARY KEY,
    school_id UUID NOT NULL REFERENCES schools(id),
    name TEXT NOT NULL CHECK (name = btrim(name) AND char_length(name) BETWEEN 1 AND 50),
    menu_status TEXT NOT NULL DEFAULT 'shown' CHECK (menu_status IN ('shown', 'hot', 'hidden')),
    source_type TEXT NOT NULL DEFAULT 'admin' CHECK (source_type IN ('admin', 'user')),
    enable_status TEXT NOT NULL DEFAULT 'enabled' CHECK (enable_status IN ('enabled', 'pending', 'disabled')),
    sort_order BIGINT NOT NULL DEFAULT 0,
    revision BIGINT NOT NULL DEFAULT 1 CHECK (revision > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);
CREATE UNIQUE INDEX home_menu_tags_school_name ON home_menu_tags (school_id, lower(btrim(name))) WHERE deleted_at IS NULL;
CREATE INDEX home_menu_tags_school_order ON home_menu_tags (school_id, sort_order, id) WHERE deleted_at IS NULL;

INSERT INTO home_menu_tags (id, school_id, name, sort_order)
SELECT id, school_id, btrim(title), sort_order
FROM school_resources
WHERE kind = 'tag' AND school_id IS NOT NULL AND active
  AND (starts_at IS NULL OR starts_at <= now())
  AND (ends_at IS NULL OR ends_at > now());
