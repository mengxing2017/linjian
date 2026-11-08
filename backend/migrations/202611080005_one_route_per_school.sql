-- A school has one current route; a connection can serve many schools.
-- Preserve superseded legacy configuration separately; frozen outbox rows reference versions.
CREATE TABLE school_source_binding_archive (
 binding_id uuid PRIMARY KEY,
 school_id uuid NOT NULL,
 snapshot jsonb NOT NULL,
 archived_at timestamptz NOT NULL DEFAULT now()
);
WITH ranked AS (
 SELECT id,row_number() OVER (PARTITION BY school_id ORDER BY active DESC,id DESC) AS position
 FROM school_source_bindings
)
INSERT INTO school_source_binding_archive(binding_id,school_id,snapshot)
SELECT b.id,b.school_id,to_jsonb(b) FROM school_source_bindings b JOIN ranked r USING(id) WHERE r.position>1;
DELETE FROM school_source_bindings b USING school_source_binding_archive a WHERE b.id=a.binding_id;
ALTER TABLE school_source_bindings ADD CONSTRAINT school_source_bindings_one_per_school UNIQUE(school_id);
