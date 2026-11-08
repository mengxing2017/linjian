CREATE TABLE source_connections (
 id uuid PRIMARY KEY, name text NOT NULL, source_id uuid NOT NULL, origin text NOT NULL,
 key_id text NOT NULL, encrypted_secret bytea NOT NULL, encryption_version text NOT NULL DEFAULT 'v1', revision bigint NOT NULL DEFAULT 1,
 checked_at timestamptz, expires_at timestamptz, capability jsonb, created_at timestamptz NOT NULL DEFAULT now(), updated_at timestamptz NOT NULL DEFAULT now(), UNIQUE(id,source_id,origin)
);
CREATE TABLE source_connection_versions (
 id uuid PRIMARY KEY, connection_id uuid NOT NULL REFERENCES source_connections, revision bigint NOT NULL,
 source_id uuid NOT NULL, origin text NOT NULL, public_origin text NOT NULL, schema_version integer NOT NULL DEFAULT 2, signature_version integer NOT NULL DEFAULT 2,
 UNIQUE(connection_id,revision), UNIQUE(id,connection_id,source_id,origin),
 FOREIGN KEY(connection_id,source_id,origin) REFERENCES source_connections(id,source_id,origin)
);
CREATE TABLE school_source_bindings (
 id uuid PRIMARY KEY, school_id uuid NOT NULL REFERENCES schools, connection_id uuid NOT NULL REFERENCES source_connections,
 connection_version uuid NOT NULL REFERENCES source_connection_versions, source_id uuid NOT NULL, origin text NOT NULL,
 route_id text NOT NULL, active boolean NOT NULL DEFAULT true, revision bigint NOT NULL DEFAULT 1,
 UNIQUE(origin,source_id,route_id), UNIQUE(school_id,connection_id,route_id),
 FOREIGN KEY(connection_version,connection_id,source_id,origin) REFERENCES source_connection_versions(id,connection_id,source_id,origin)
);
ALTER TABLE forum_outbox ADD COLUMN connection_version uuid REFERENCES source_connection_versions,
 ADD COLUMN source_id uuid, ADD COLUMN route_id text, ADD COLUMN schema_version integer NOT NULL DEFAULT 1,
 ADD COLUMN signature_version integer NOT NULL DEFAULT 1, ADD COLUMN body_sha256 text,
 ADD COLUMN blocked_reason text, ADD COLUMN blocked_at timestamptz, ADD COLUMN auth_failure_count integer NOT NULL DEFAULT 0;
ALTER TABLE forum_outbox DROP CONSTRAINT forum_outbox_school_id_post_id_sequence_key;
CREATE UNIQUE INDEX forum_outbox_version_route ON forum_outbox(school_id,post_id,sequence,coalesce(route_id,''));
-- Existing events keep their original identity/version. Never convert legacy frozen rows.
CREATE FUNCTION forum_snapshot_source_v2() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE b record; configured boolean := false; p jsonb; new_id uuid; body text;
BEGIN
 IF NEW.schema_version=2 THEN RETURN NEW; END IF;
 FOR b IN SELECT sb.*,v.public_origin FROM school_source_bindings sb JOIN source_connection_versions v ON v.id=sb.connection_version WHERE sb.school_id=NEW.school_id AND sb.active ORDER BY sb.id LOOP
  configured := true; new_id := gen_random_uuid();
  p := (NEW.payload - 'external_organization_id') || jsonb_build_object('event_id',new_id,'schema_version',2,'route_id',b.route_id,'detail_url',b.public_origin || '/community/' || NEW.school_id || '/posts/' || NEW.post_id);
  body := jsonb_build_object('data',jsonb_build_object('type','source-events','id',new_id,'attributes',p))::text;
  INSERT INTO forum_outbox(id,school_id,post_id,sequence,payload,connection_version,source_id,route_id,schema_version,signature_version,request_body,body_sha256)
  VALUES(new_id,NEW.school_id,NEW.post_id,NEW.sequence,p,b.connection_version,b.source_id,b.route_id,2,2,body,encode(sha256(convert_to(body,'UTF8')),'hex'));
 END LOOP;
 IF configured THEN RETURN NULL; END IF;
 NEW.blocked_reason := 'legacy_version_blocked'; NEW.blocked_at := now(); RETURN NEW;
END $$;
CREATE TRIGGER forum_v2_snapshot BEFORE INSERT ON forum_outbox FOR EACH ROW EXECUTE FUNCTION forum_snapshot_source_v2();
UPDATE forum_outbox SET blocked_reason='legacy_version_blocked',blocked_at=now() WHERE schema_version=1;
