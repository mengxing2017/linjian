CREATE TABLE home_menu_imports (
    id uuid NOT NULL,
    school_id uuid NOT NULL,
    actor_id uuid NOT NULL,
    digest text NOT NULL,
    file_digest text NOT NULL,
    catalog_revision bigint NOT NULL,
    rows jsonb NOT NULL,
    errors jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    expires_at timestamp with time zone DEFAULT (now() + '00:30:00'::interval) NOT NULL,
    result jsonb,
    committed_at timestamp with time zone,
    CONSTRAINT home_menu_imports_pkey PRIMARY KEY (id)
);

CREATE TABLE home_menu_settings (
    school_id uuid NOT NULL,
    latest_label text DEFAULT '最新'::text NOT NULL,
    latest_visible boolean DEFAULT true NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    catalog_revision bigint DEFAULT 1 NOT NULL,
    CONSTRAINT home_menu_settings_catalog_revision_check CHECK ((catalog_revision > 0)),
    CONSTRAINT home_menu_settings_latest_label_check CHECK (((char_length(btrim(latest_label)) >= 1) AND (char_length(btrim(latest_label)) <= 50))),
    CONSTRAINT home_menu_settings_revision_check CHECK ((revision > 0)),
    CONSTRAINT home_menu_settings_pkey PRIMARY KEY (school_id)
);

CREATE TABLE home_menu_tags (
    id uuid NOT NULL,
    school_id uuid NOT NULL,
    name text NOT NULL,
    menu_status text DEFAULT 'shown'::text NOT NULL,
    source_type text DEFAULT 'admin'::text NOT NULL,
    enable_status text DEFAULT 'enabled'::text NOT NULL,
    sort_order bigint DEFAULT 0 NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    deleted_at timestamp with time zone,
    CONSTRAINT home_menu_tags_enable_status_check CHECK ((enable_status = ANY (ARRAY['enabled'::text, 'pending'::text, 'disabled'::text]))),
    CONSTRAINT home_menu_tags_menu_status_check CHECK ((menu_status = ANY (ARRAY['shown'::text, 'hot'::text, 'hidden'::text]))),
    CONSTRAINT home_menu_tags_name_check CHECK (((name = btrim(name)) AND ((char_length(name) >= 1) AND (char_length(name) <= 50)))),
    CONSTRAINT home_menu_tags_revision_check CHECK ((revision > 0)),
    CONSTRAINT home_menu_tags_source_type_check CHECK ((source_type = ANY (ARRAY['admin'::text, 'user'::text]))),
    CONSTRAINT home_menu_tags_pkey PRIMARY KEY (id)
);

CREATE UNIQUE INDEX home_menu_tags_school_name ON home_menu_tags USING btree (school_id, lower(btrim(name))) WHERE (deleted_at IS NULL);

CREATE INDEX home_menu_tags_school_order ON home_menu_tags USING btree (school_id, sort_order, id) WHERE (deleted_at IS NULL);

CREATE TABLE miniapp_avatar_assets (
    id uuid NOT NULL,
    owner_id uuid NOT NULL,
    mime_type text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT miniapp_avatar_assets_owner_id_id_key UNIQUE (owner_id, id),
    CONSTRAINT miniapp_avatar_assets_pkey PRIMARY KEY (id)
);

CREATE TABLE miniapp_identities (
    app_id text NOT NULL,
    openid text NOT NULL,
    account_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT miniapp_identities_app_id_check CHECK (((length(app_id) >= 1) AND (length(app_id) <= 128))),
    CONSTRAINT miniapp_identities_openid_check CHECK (((length(openid) >= 1) AND (length(openid) <= 256))),
    CONSTRAINT miniapp_identities_app_id_account_id_key UNIQUE (app_id, account_id),
    CONSTRAINT miniapp_identities_pkey PRIMARY KEY (app_id, openid)
);

CREATE TABLE miniapp_link_assets (
    id uuid NOT NULL,
    cache_key text NOT NULL,
    app_id text NOT NULL,
    policy jsonb NOT NULL,
    page_path text NOT NULL,
    canonical_query text NOT NULL,
    page_title text NOT NULL,
    link_type text NOT NULL,
    permanent boolean NOT NULL,
    link text NOT NULL,
    generated_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone,
    expiry_basis text DEFAULT 'provider_policy'::text NOT NULL,
    CONSTRAINT miniapp_link_assets_check CHECK ((permanent = (expires_at IS NULL))),
    CONSTRAINT miniapp_link_assets_link_check CHECK (((length(link) >= 1) AND (length(link) <= 4096))),
    CONSTRAINT miniapp_link_assets_link_type_check CHECK ((link_type = ANY (ARRAY['short_link'::text, 'url_link'::text]))),
    CONSTRAINT miniapp_link_assets_pkey PRIMARY KEY (id)
);

CREATE INDEX miniapp_link_assets_cache ON miniapp_link_assets USING btree (cache_key, generated_at DESC);

CREATE TABLE miniapp_link_leases (
    cache_key text NOT NULL,
    lease_token uuid NOT NULL,
    lease_until timestamp with time zone NOT NULL,
    CONSTRAINT miniapp_link_leases_pkey PRIMARY KEY (cache_key)
);

CREATE TABLE miniapp_registration_settings (
    id smallint NOT NULL,
    nickname_prefix text NOT NULL,
    revision bigint DEFAULT 0 NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_by uuid,
    CONSTRAINT miniapp_registration_settings_id_check CHECK ((id = 1)),
    CONSTRAINT miniapp_registration_settings_nickname_prefix_check CHECK (((nickname_prefix = btrim(nickname_prefix)) AND ((char_length(nickname_prefix) >= 1) AND (char_length(nickname_prefix) <= 91)))),
    CONSTRAINT miniapp_registration_settings_revision_check CHECK ((revision >= 0)),
    CONSTRAINT miniapp_registration_settings_pkey PRIMARY KEY (id)
);

CREATE TABLE miniapp_school_change_documents (
    id uuid NOT NULL,
    owner_id uuid NOT NULL,
    request_id uuid,
    state text NOT NULL,
    purpose text DEFAULT 'school_change'::text NOT NULL,
    mime text NOT NULL,
    size_bytes bigint NOT NULL,
    sha256 text NOT NULL,
    store_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    expires_at timestamp with time zone,
    purge_after timestamp with time zone,
    deleted_at timestamp with time zone,
    attempts integer DEFAULT 0 NOT NULL,
    retry_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT miniapp_school_change_documents_check CHECK (((request_id IS NULL) OR (expires_at IS NULL))),
    CONSTRAINT miniapp_school_change_documents_purpose_check CHECK ((purpose = 'school_change'::text)),
    CONSTRAINT miniapp_school_change_documents_size_bytes_check CHECK ((size_bytes > 0)),
    CONSTRAINT miniapp_school_change_documents_state_check CHECK ((state = ANY (ARRAY['uploading'::text, 'ready'::text, 'deleting'::text, 'deleted'::text]))),
    CONSTRAINT miniapp_school_change_documents_pkey PRIMARY KEY (id)
);

CREATE INDEX miniapp_school_change_document_cleanup ON miniapp_school_change_documents USING btree (state, retry_at);

CREATE INDEX miniapp_school_change_document_owner ON miniapp_school_change_documents USING btree (owner_id, created_at DESC);

CREATE INDEX miniapp_school_change_document_request ON miniapp_school_change_documents USING btree (request_id);

CREATE TABLE miniapp_school_change_events (
    id uuid NOT NULL,
    request_id uuid,
    document_id uuid,
    actor_id uuid,
    action text NOT NULL,
    reason_code text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    purge_after timestamp with time zone NOT NULL,
    CONSTRAINT miniapp_school_change_events_pkey PRIMARY KEY (id)
);

CREATE INDEX miniapp_school_change_events_purge ON miniapp_school_change_events USING btree (purge_after);

CREATE INDEX miniapp_school_change_events_request ON miniapp_school_change_events USING btree (request_id, created_at, id);

CREATE TABLE miniapp_school_change_requests (
    id uuid NOT NULL,
    account_id uuid NOT NULL,
    from_school_id uuid NOT NULL,
    to_school_id uuid NOT NULL,
    declared_student_name text,
    declared_student_number text,
    reason text,
    status text DEFAULT 'pending'::text NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    submission_key uuid NOT NULL,
    submission_digest text NOT NULL,
    consent_version text NOT NULL,
    consent_notice text NOT NULL,
    review_guidance text NOT NULL,
    consented_at timestamp with time zone DEFAULT now() NOT NULL,
    submitted_at timestamp with time zone DEFAULT now() NOT NULL,
    review_due_at timestamp with time zone NOT NULL,
    reviewed_by uuid,
    reviewed_at timestamp with time zone,
    closed_at timestamp with time zone,
    result_reason_code text,
    result_reason text,
    document_retention_seconds bigint NOT NULL,
    personal_retention_seconds bigint NOT NULL,
    audit_retention_seconds bigint NOT NULL,
    personal_data_purged_at timestamp with time zone,
    student_relationship text,
    CONSTRAINT miniapp_school_change_requests_audit_retention_seconds_check CHECK ((audit_retention_seconds > 0)),
    CONSTRAINT miniapp_school_change_requests_check CHECK ((from_school_id <> to_school_id)),
    CONSTRAINT miniapp_school_change_requests_check1 CHECK (((status = 'pending'::text) = (closed_at IS NULL))),
    CONSTRAINT miniapp_school_change_requests_check3 CHECK (((reviewed_by IS NULL) = (reviewed_at IS NULL))),
    CONSTRAINT miniapp_school_change_requests_check4 CHECK (((reviewed_by IS NULL) OR (reviewed_by <> account_id))),
    CONSTRAINT miniapp_school_change_requests_document_retention_seconds_check CHECK ((document_retention_seconds > 0)),
    CONSTRAINT miniapp_school_change_requests_personal_retention_seconds_check CHECK ((personal_retention_seconds > 0)),
    CONSTRAINT miniapp_school_change_requests_revision_check CHECK ((revision > 0)),
    CONSTRAINT miniapp_school_change_requests_status_check CHECK ((status = ANY (ARRAY['pending'::text, 'approved'::text, 'rejected'::text, 'cancelled'::text, 'invalidated'::text]))),
    CONSTRAINT miniapp_school_change_requests_student_relationship_check CHECK ((student_relationship = ANY (ARRAY['enrolled'::text, 'new_student'::text]))),
    CONSTRAINT miniapp_school_change_review_outcome CHECK ((((status = ANY (ARRAY['approved'::text, 'rejected'::text])) AND (reviewed_by IS NOT NULL) AND (reviewed_at IS NOT NULL) AND (result_reason_code IS DISTINCT FROM 'review_timeout'::text)) OR ((status = 'rejected'::text) AND (reviewed_by IS NULL) AND (reviewed_at IS NULL) AND (NOT (result_reason_code IS DISTINCT FROM 'review_timeout'::text))) OR ((status <> ALL (ARRAY['approved'::text, 'rejected'::text])) AND (reviewed_by IS NULL) AND (reviewed_at IS NULL) AND (result_reason_code IS DISTINCT FROM 'review_timeout'::text)))),
    CONSTRAINT miniapp_school_change_requests_account_id_id_key UNIQUE (account_id, id),
    CONSTRAINT miniapp_school_change_requests_account_id_submission_key_key UNIQUE (account_id, submission_key),
    CONSTRAINT miniapp_school_change_requests_pkey PRIMARY KEY (id)
);

CREATE UNIQUE INDEX miniapp_school_change_pending ON miniapp_school_change_requests USING btree (account_id) WHERE (status = 'pending'::text);

CREATE INDEX miniapp_school_change_queue ON miniapp_school_change_requests USING btree (status, submitted_at, id);

CREATE INDEX miniapp_school_change_recent ON miniapp_school_change_requests USING btree (account_id, submitted_at DESC, id DESC);

CREATE INDEX miniapp_school_change_review_deadline ON miniapp_school_change_requests USING btree (review_due_at, id) WHERE (status = 'pending'::text);

CREATE TABLE miniapp_token_cache (
    credential_scope text NOT NULL,
    token text,
    expires_at timestamp with time zone,
    generation uuid,
    lease_token uuid,
    lease_until timestamp with time zone,
    CONSTRAINT miniapp_token_cache_pkey PRIMARY KEY (credential_scope)
);

CREATE TABLE miniapp_users (
    account_id uuid NOT NULL,
    nickname text NOT NULL,
    avatar_asset_id uuid,
    mobile text,
    mobile_source text,
    mobile_updated_at timestamp with time zone,
    status text DEFAULT 'normal'::text NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    status_reason text,
    status_changed_at timestamp with time zone,
    status_changed_by uuid,
    CONSTRAINT miniapp_users_mobile_check CHECK ((mobile ~ '^1[0-9]{10}$'::text)),
    CONSTRAINT miniapp_users_mobile_source_check CHECK ((mobile_source = ANY (ARRAY['user_declared'::text, 'admin_corrected'::text]))),
    CONSTRAINT miniapp_users_nickname_check CHECK (((length(btrim(nickname)) >= 1) AND (length(btrim(nickname)) <= 100))),
    CONSTRAINT miniapp_users_revision_check CHECK ((revision > 0)),
    CONSTRAINT miniapp_users_status_check CHECK ((status = ANY (ARRAY['normal'::text, 'banned'::text]))),
    CONSTRAINT miniapp_users_pkey PRIMARY KEY (account_id)
);

CREATE INDEX miniapp_users_created ON miniapp_users USING btree (created_at DESC, account_id DESC);

CREATE INDEX miniapp_users_nickname_lookup ON miniapp_users USING btree (nickname);
