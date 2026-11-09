ALTER TABLE comments ADD COLUMN visibility TEXT NOT NULL DEFAULT 'visible' CHECK (visibility IN ('visible','hidden'));
-- Visibility changes must not change the provenance of an existing audit decision.
ALTER TABLE comments ADD COLUMN decision_revision BIGINT NOT NULL DEFAULT 1;
UPDATE comments SET decision_revision=revision;
CREATE INDEX comment_management_visibility ON comments(school_id,visibility,created_at DESC,id DESC) WHERE NOT deleted;
