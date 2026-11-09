-- Independent miniapp registration records; backend login identities remain in accounts.
CREATE TABLE miniapp_avatar_assets (
 id uuid PRIMARY KEY, owner_id uuid NOT NULL REFERENCES accounts,
 mime_type text NOT NULL, content bytea NOT NULL, created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(owner_id,id), CHECK(octet_length(content) BETWEEN 1 AND 5242880)
);
CREATE TABLE miniapp_users (
 account_id uuid PRIMARY KEY REFERENCES accounts, nickname text NOT NULL CHECK(length(btrim(nickname)) BETWEEN 1 AND 100),
 avatar_asset_id uuid, mobile text CHECK(mobile ~ '^1[0-9]{10}$'),
 mobile_source text CHECK(mobile_source IN ('user_declared','admin_corrected')),
 mobile_updated_at timestamptz, status text NOT NULL DEFAULT 'normal' CHECK(status IN ('normal','banned')),
 revision bigint NOT NULL DEFAULT 1 CHECK(revision>0), created_at timestamptz NOT NULL DEFAULT now(),
 updated_at timestamptz NOT NULL DEFAULT now(), status_reason text, status_changed_at timestamptz,
 status_changed_by uuid REFERENCES accounts,
 FOREIGN KEY(account_id,avatar_asset_id) REFERENCES miniapp_avatar_assets(owner_id,id)
);
CREATE TABLE miniapp_identities (
 app_id text NOT NULL CHECK(length(app_id) BETWEEN 1 AND 128), openid text NOT NULL CHECK(length(openid) BETWEEN 1 AND 256),
 account_id uuid NOT NULL REFERENCES miniapp_users(account_id), created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(app_id,openid), UNIQUE(app_id,account_id)
);
CREATE INDEX miniapp_users_created ON miniapp_users(created_at DESC,account_id DESC);
CREATE INDEX accounts_bound_school ON accounts(bound_school_id,id);
CREATE INDEX posts_user_school_counts ON posts(school_id,author_id) WHERE visibility<>'deleted';
