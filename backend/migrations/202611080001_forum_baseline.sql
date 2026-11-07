-- Database baseline: application metadata and core business. Requires an empty schema.
CREATE TABLE application_metadata (
    singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    application TEXT NOT NULL DEFAULT 'linjian' CHECK(application='linjian'),
    schema_generation INTEGER NOT NULL CHECK (schema_generation > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO application_metadata (singleton, schema_generation) VALUES (TRUE, 1);

CREATE TABLE accounts (
 id UUID PRIMARY KEY, username TEXT UNIQUE NOT NULL, password_hash TEXT,
 display_name TEXT NOT NULL, platform_admin BOOLEAN NOT NULL DEFAULT FALSE,
 active BOOLEAN NOT NULL DEFAULT TRUE, wecom_member_id UUID UNIQUE, wecom_binding_revision BIGINT NOT NULL DEFAULT 0, wx_openid TEXT UNIQUE,
 bound_school_id UUID, created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 avatar_url TEXT NOT NULL DEFAULT ''
);
CREATE TABLE schools (
 id UUID PRIMARY KEY, name TEXT NOT NULL, active BOOLEAN NOT NULL DEFAULT TRUE,
 config JSONB NOT NULL DEFAULT '{}', revision BIGINT NOT NULL DEFAULT 1,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
ALTER TABLE accounts ADD FOREIGN KEY (bound_school_id) REFERENCES schools(id);
CREATE TABLE school_memberships (
 school_id UUID NOT NULL REFERENCES schools(id), account_id UUID NOT NULL REFERENCES accounts(id),
 role TEXT NOT NULL CHECK (role IN ('member','operator','admin','revoked')), PRIMARY KEY (school_id,account_id)
);
CREATE TABLE sessions (
 token_hash TEXT PRIMARY KEY, account_id UUID NOT NULL REFERENCES accounts(id), csrf TEXT NOT NULL,
 expires_at TIMESTAMPTZ NOT NULL, last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 credential_generation BIGINT, kind TEXT NOT NULL CHECK (kind IN ('browser','miniapp')), wecom_authenticated BOOLEAN NOT NULL DEFAULT FALSE, wecom_binding_revision BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX sessions_expiry ON sessions(expires_at);
CREATE TABLE login_attempts (username TEXT PRIMARY KEY, failures INTEGER NOT NULL, reset_at TIMESTAMPTZ NOT NULL);
CREATE TABLE audit_events (
 id UUID PRIMARY KEY, school_id UUID REFERENCES schools(id), actor_id UUID REFERENCES accounts(id),
 action TEXT NOT NULL, object_id UUID, detail JSONB NOT NULL DEFAULT '{}', created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 request_id TEXT
);
CREATE INDEX audit_scope ON audit_events(school_id,created_at DESC);
CREATE TABLE posts (
 id UUID PRIMARY KEY, school_id UUID NOT NULL REFERENCES schools(id), author_id UUID NOT NULL REFERENCES accounts(id),
 title TEXT NOT NULL, content TEXT NOT NULL, images JSONB NOT NULL DEFAULT '[]', tags JSONB NOT NULL DEFAULT '[]',
 audit_status TEXT NOT NULL DEFAULT 'pending' CHECK(audit_status IN ('pending','approved','rejected')),
 visibility TEXT NOT NULL DEFAULT 'visible' CHECK(visibility IN ('visible','hidden','deleted')),
 top_sort INTEGER NOT NULL DEFAULT 0, browse BIGINT NOT NULL DEFAULT 0, revision BIGINT NOT NULL DEFAULT 1,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(), updated_at TIMESTAMPTZ NOT NULL DEFAULT now(), UNIQUE(school_id,id)
);
CREATE INDEX post_feed ON posts(school_id,audit_status,visibility,top_sort DESC,created_at DESC,id);
CREATE TABLE comments (
 id UUID PRIMARY KEY, school_id UUID NOT NULL, post_id UUID NOT NULL, author_id UUID NOT NULL REFERENCES accounts(id),
 parent_id UUID REFERENCES comments(id) DEFERRABLE INITIALLY DEFERRED, content TEXT NOT NULL,
 audit_status TEXT NOT NULL DEFAULT 'pending' CHECK(audit_status IN ('pending','approved','rejected')),
 deleted BOOLEAN NOT NULL DEFAULT FALSE, revision BIGINT NOT NULL DEFAULT 1, created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 FOREIGN KEY(school_id,post_id) REFERENCES posts(school_id,id)
);
CREATE INDEX comment_post ON comments(school_id,post_id,created_at);
CREATE TABLE post_reactions (
 school_id UUID NOT NULL, post_id UUID NOT NULL, account_id UUID NOT NULL REFERENCES accounts(id),
 kind TEXT NOT NULL CHECK(kind IN ('like','collect')), PRIMARY KEY(post_id,account_id,kind),
 FOREIGN KEY(school_id,post_id) REFERENCES posts(school_id,id)
);
CREATE TABLE follows (
 school_id UUID NOT NULL REFERENCES schools(id), follower_id UUID NOT NULL REFERENCES accounts(id),
 target_id UUID NOT NULL REFERENCES accounts(id), PRIMARY KEY(school_id,follower_id,target_id), CHECK(follower_id<>target_id)
);
CREATE TABLE notifications (
 id UUID PRIMARY KEY, school_id UUID NOT NULL REFERENCES schools(id), account_id UUID NOT NULL REFERENCES accounts(id),
 kind TEXT NOT NULL, object_id UUID, read_at TIMESTAMPTZ, created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE moderation_events (
 id UUID PRIMARY KEY, school_id UUID NOT NULL REFERENCES schools(id), actor_id UUID NOT NULL REFERENCES accounts(id),
 object_id UUID NOT NULL, object_kind TEXT NOT NULL, decision TEXT NOT NULL, reason TEXT NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE cases (
 id UUID PRIMARY KEY, school_id UUID NOT NULL REFERENCES schools(id), account_id UUID NOT NULL REFERENCES accounts(id),
 kind TEXT NOT NULL CHECK(kind IN ('report','feedback')), post_id UUID, content TEXT NOT NULL, contact TEXT NOT NULL DEFAULT '',
 state TEXT NOT NULL DEFAULT 'open' CHECK(state IN ('open','resolved','dismissed')), resolution TEXT NOT NULL DEFAULT '',
 revision BIGINT NOT NULL DEFAULT 1, created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 FOREIGN KEY(school_id,post_id) REFERENCES posts(school_id,id),
 images JSONB NOT NULL DEFAULT '[]'
);
CREATE TABLE school_resources (
 id UUID PRIMARY KEY, school_id UUID REFERENCES schools(id),
 kind TEXT NOT NULL CHECK(kind IN ('banner','qrcode','service','legal','tag','blocked_word')),
 title TEXT NOT NULL, content TEXT NOT NULL DEFAULT '', url TEXT NOT NULL DEFAULT '', active BOOLEAN NOT NULL DEFAULT TRUE,
 sort_order INTEGER NOT NULL DEFAULT 0, starts_at TIMESTAMPTZ, ends_at TIMESTAMPTZ,
 revision BIGINT NOT NULL DEFAULT 1, clicks BIGINT NOT NULL DEFAULT 0, CHECK(ends_at IS NULL OR starts_at IS NULL OR ends_at>starts_at)
);
CREATE TABLE media_assets (
 id UUID PRIMARY KEY, school_id UUID NOT NULL REFERENCES schools(id), owner_id UUID NOT NULL REFERENCES accounts(id),
 mime_type TEXT NOT NULL, size_bytes BIGINT NOT NULL, content BYTEA NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE oauth_states (state_hash TEXT PRIMARY KEY, session_hash TEXT, return_path TEXT NOT NULL, expires_at TIMESTAMPTZ NOT NULL);
CREATE TABLE platform_cache (key TEXT PRIMARY KEY, value TEXT NOT NULL, expires_at TIMESTAMPTZ NOT NULL);
CREATE TABLE legacy_id_map (
 source_table TEXT NOT NULL, legacy_id TEXT NOT NULL, target_id UUID NOT NULL, batch_id UUID NOT NULL,
 PRIMARY KEY(source_table,legacy_id)
);
CREATE TABLE migration_runs (id UUID PRIMARY KEY, source_hash TEXT NOT NULL UNIQUE, summary JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now());

-- Single-enterprise directory. Runs stage remotely; publication is one transaction.
CREATE TABLE wecom_directory_state (
 singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK(singleton), corp_id TEXT NOT NULL, credential_scope TEXT NOT NULL,
 credential_generation BIGINT NOT NULL DEFAULT 1 CHECK(credential_generation>0),
 snapshot_id UUID, last_success_at TIMESTAMPTZ, next_call_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE wecom_directory_sync_runs (
 id UUID PRIMARY KEY, actor_id UUID REFERENCES accounts(id), generation BIGINT NOT NULL CHECK(generation>0), state TEXT NOT NULL DEFAULT 'queued' CHECK(state IN ('queued','running','succeeded','failed')),
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(), started_at TIMESTAMPTZ, finished_at TIMESTAMPTZ,
 lease_id UUID, lease_until TIMESTAMPTZ, error_code TEXT,
 department_count INTEGER NOT NULL DEFAULT 0, member_count INTEGER NOT NULL DEFAULT 0, snapshot JSONB
);
CREATE UNIQUE INDEX wecom_directory_one_job ON wecom_directory_sync_runs((TRUE)) WHERE state IN ('queued','running');
CREATE TABLE wecom_departments (
 id BIGINT PRIMARY KEY, parentid BIGINT NOT NULL, name TEXT NOT NULL, "order" BIGINT NOT NULL DEFAULT 0
);
CREATE TABLE wecom_members (
 id UUID PRIMARY KEY, userid TEXT NOT NULL, userid_key TEXT GENERATED ALWAYS AS (lower(userid COLLATE "C")) STORED UNIQUE,
 name TEXT NOT NULL, status INTEGER, department BIGINT[] NOT NULL DEFAULT '{}', main_department BIGINT,
 visibility TEXT NOT NULL DEFAULT 'visible' CHECK(visibility IN ('visible','not_seen')),
 verification TEXT NOT NULL DEFAULT 'unverified' CHECK(verification IN ('verified','unavailable','unverified')),
 checked_at TIMESTAMPTZ, snapshot_id UUID
);
ALTER TABLE accounts ADD FOREIGN KEY(wecom_member_id) REFERENCES wecom_members(id);

-- Public aliases and historical business records
-- Integer aliases preserve old mini-app routes without constraining new UUID entities.
-- A table-only development reset can leave this formerly standalone sequence.
-- Reuse its counter, then make its lifetime follow the column that uses it.
CREATE SEQUENCE IF NOT EXISTS legacy_public_id START 1000000000000;
CREATE TABLE legacy_aliases(kind TEXT NOT NULL,target_id UUID NOT NULL,public_id BIGINT NOT NULL DEFAULT nextval('legacy_public_id'),PRIMARY KEY(kind,target_id),UNIQUE(kind,public_id));
ALTER SEQUENCE legacy_public_id OWNED BY legacy_aliases.public_id;
CREATE TABLE comment_reactions(school_id UUID NOT NULL REFERENCES schools(id),comment_id UUID NOT NULL REFERENCES comments(id),account_id UUID NOT NULL REFERENCES accounts(id),PRIMARY KEY(comment_id,account_id));
CREATE TABLE media_access_tokens(token_hash TEXT PRIMARY KEY,school_id UUID NOT NULL REFERENCES schools(id),asset_id UUID NOT NULL REFERENCES media_assets(id),expires_at TIMESTAMPTZ NOT NULL);
CREATE INDEX media_access_expiry ON media_access_tokens(expires_at);
CREATE INDEX notification_owner ON notifications(school_id,account_id,created_at DESC);
CREATE INDEX cases_school ON cases(school_id,state,created_at DESC);


CREATE TABLE product_previews (
 id uuid PRIMARY KEY, actor_id uuid NOT NULL REFERENCES accounts(id), school_id uuid REFERENCES schools(id),
 kind text NOT NULL, payload jsonb NOT NULL, dependencies jsonb NOT NULL DEFAULT '{}', digest text NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(), expires_at timestamptz NOT NULL DEFAULT now()+interval '15 minutes',
 result jsonb, confirmed_at timestamptz, CHECK ((result IS NULL)=(confirmed_at IS NULL))
);
CREATE INDEX product_previews_expiry ON product_previews(expires_at) WHERE result IS NULL;

CREATE TABLE operational_media (
 id uuid PRIMARY KEY, school_id uuid REFERENCES schools(id), owner_id uuid NOT NULL REFERENCES accounts(id),
 mime_type text NOT NULL, size_bytes bigint NOT NULL CHECK(size_bytes BETWEEN 1 AND 5242880),
 width integer NOT NULL CHECK(width BETWEEN 1 AND 8192), height integer NOT NULL CHECK(height BETWEEN 1 AND 8192),
 content bytea NOT NULL, created_at timestamptz NOT NULL DEFAULT now(), CHECK(width::bigint*height<=24000000)
);
ALTER TABLE school_resources DROP COLUMN url;
ALTER TABLE school_resources ADD COLUMN image_ref jsonb;
ALTER TABLE school_resources ADD COLUMN action jsonb NOT NULL DEFAULT '{"kind":"none"}';
CREATE TABLE resource_publications (
 id uuid PRIMARY KEY, resource_id uuid NOT NULL REFERENCES school_resources(id), school_id uuid REFERENCES schools(id),
 actor_id uuid NOT NULL REFERENCES accounts(id), snapshot jsonb NOT NULL, created_at timestamptz NOT NULL DEFAULT now()
);
CREATE FUNCTION forum_events_immutable() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'immutable forum event'; END $$;
CREATE TRIGGER resource_publications_immutable BEFORE UPDATE OR DELETE ON resource_publications FOR EACH ROW EXECUTE FUNCTION forum_events_immutable();

