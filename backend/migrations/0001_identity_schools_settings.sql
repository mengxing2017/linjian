CREATE SEQUENCE legacy_public_id
    START WITH 1000000000000
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;

CREATE TABLE accounts (
    id uuid NOT NULL,
    username text NOT NULL,
    password_hash text,
    display_name text NOT NULL,
    platform_admin boolean DEFAULT false NOT NULL,
    active boolean DEFAULT true NOT NULL,
    wx_openid text,
    bound_school_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    avatar_url text DEFAULT ''::text NOT NULL,
    account_kind text DEFAULT 'ordinary'::text NOT NULL,
    CONSTRAINT accounts_account_kind_check CHECK ((account_kind = ANY (ARRAY['ordinary'::text, 'simulated'::text]))),
    CONSTRAINT simulated_credentials CHECK (((account_kind <> 'simulated'::text) OR ((bound_school_id IS NOT NULL) AND (password_hash IS NULL) AND (NOT platform_admin) AND (wx_openid IS NULL)))),
    CONSTRAINT accounts_pkey PRIMARY KEY (id),
    CONSTRAINT accounts_username_key UNIQUE (username),
    CONSTRAINT accounts_wx_openid_key UNIQUE (wx_openid)
);

CREATE INDEX accounts_bound_school ON accounts USING btree (bound_school_id, id);

CREATE TABLE application_metadata (
    singleton boolean DEFAULT true NOT NULL,
    application text DEFAULT 'linjian'::text NOT NULL,
    schema_generation integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT application_metadata_application_check CHECK ((application = 'linjian'::text)),
    CONSTRAINT application_metadata_schema_generation_check CHECK ((schema_generation > 0)),
    CONSTRAINT application_metadata_singleton_check CHECK (singleton),
    CONSTRAINT application_metadata_pkey PRIMARY KEY (singleton)
);

CREATE TABLE audit_events (
    id uuid NOT NULL,
    school_id uuid,
    actor_id uuid,
    action text NOT NULL,
    object_id uuid,
    detail jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    request_id text,
    CONSTRAINT audit_events_pkey PRIMARY KEY (id)
);

CREATE INDEX audit_scope ON audit_events USING btree (school_id, created_at DESC);

CREATE INDEX site_audit_history ON audit_events USING btree (action, object_id, created_at DESC);

CREATE TABLE legacy_aliases (
    kind text NOT NULL,
    target_id uuid NOT NULL,
    public_id bigint DEFAULT nextval('legacy_public_id'::regclass) NOT NULL,
    CONSTRAINT legacy_aliases_kind_public_id_key UNIQUE (kind, public_id),
    CONSTRAINT legacy_aliases_pkey PRIMARY KEY (kind, target_id)
);

CREATE TABLE legacy_id_map (
    source_table text NOT NULL,
    legacy_id text NOT NULL,
    target_id uuid NOT NULL,
    batch_id uuid NOT NULL,
    CONSTRAINT legacy_id_map_pkey PRIMARY KEY (source_table, legacy_id)
);

CREATE TABLE login_attempts (
    username text NOT NULL,
    failures integer NOT NULL,
    reset_at timestamp with time zone NOT NULL,
    CONSTRAINT login_attempts_pkey PRIMARY KEY (username)
);

CREATE TABLE migration_runs (
    id uuid NOT NULL,
    source_hash text NOT NULL,
    summary jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT migration_runs_pkey PRIMARY KEY (id),
    CONSTRAINT migration_runs_source_hash_key UNIQUE (source_hash)
);

CREATE TABLE platform_cache (
    key text NOT NULL,
    value text NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    CONSTRAINT platform_cache_pkey PRIMARY KEY (key)
);

CREATE TABLE school_authorization_guards (
    school_id uuid NOT NULL,
    CONSTRAINT school_authorization_guards_pkey PRIMARY KEY (school_id)
);

CREATE TABLE school_member_authorizations (
    school_id uuid NOT NULL,
    account_id uuid NOT NULL,
    revision bigint NOT NULL,
    updated_by uuid NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT school_member_authorizations_revision_check CHECK ((revision > 0)),
    CONSTRAINT school_member_authorizations_pkey PRIMARY KEY (school_id, account_id)
);

CREATE TABLE school_member_permission_overrides (
    school_id uuid NOT NULL,
    account_id uuid NOT NULL,
    permission_key text NOT NULL,
    effect text NOT NULL,
    updated_by uuid NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT school_member_permission_overrides_effect_check CHECK ((effect = ANY (ARRAY['allow'::text, 'deny'::text]))),
    CONSTRAINT school_member_permission_overrides_pkey PRIMARY KEY (school_id, account_id, permission_key)
);

CREATE TABLE school_memberships (
    school_id uuid NOT NULL,
    account_id uuid NOT NULL,
    role text NOT NULL,
    CONSTRAINT school_memberships_role_check CHECK ((role = ANY (ARRAY['member'::text, 'operator'::text, 'admin'::text, 'revoked'::text]))),
    CONSTRAINT school_memberships_pkey PRIMARY KEY (school_id, account_id)
);

CREATE TABLE schools (
    id uuid NOT NULL,
    name text NOT NULL,
    active boolean DEFAULT true NOT NULL,
    config jsonb DEFAULT '{"post_audit_mode": "manual", "comment_audit_mode": "manual"}'::jsonb NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    short_name text DEFAULT ''::text NOT NULL,
    province text DEFAULT ''::text NOT NULL,
    city text DEFAULT ''::text NOT NULL,
    area text DEFAULT ''::text NOT NULL,
    latitude numeric(9,6),
    longitude numeric(9,6),
    logo_asset_id uuid,
    header_bg_asset_id uuid,
    theme_primary text DEFAULT '#6A5AF9'::text NOT NULL,
    theme_secondary text DEFAULT '#8F6BFF'::text NOT NULL,
    theme_text_color text DEFAULT '#FFFFFF'::text NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT school_coordinates_pair CHECK (((latitude IS NULL) = (longitude IS NULL))),
    CONSTRAINT school_name_length CHECK (((length(btrim(name)) >= 1) AND (length(btrim(name)) <= 100))),
    CONSTRAINT schools_area_check CHECK ((length(area) <= 100)),
    CONSTRAINT schools_city_check CHECK ((length(city) <= 100)),
    CONSTRAINT schools_latitude_check CHECK (((latitude >= ('-90'::integer)::numeric) AND (latitude <= (90)::numeric))),
    CONSTRAINT schools_longitude_check CHECK (((longitude >= ('-180'::integer)::numeric) AND (longitude <= (180)::numeric))),
    CONSTRAINT schools_province_check CHECK ((length(province) <= 100)),
    CONSTRAINT schools_short_name_check CHECK ((length(short_name) <= 50)),
    CONSTRAINT schools_theme_primary_check CHECK ((theme_primary ~ '^#[0-9A-F]{6}$'::text)),
    CONSTRAINT schools_theme_secondary_check CHECK ((theme_secondary ~ '^#[0-9A-F]{6}$'::text)),
    CONSTRAINT schools_theme_text_color_check CHECK ((theme_text_color ~ '^#[0-9A-F]{6}$'::text)),
    CONSTRAINT schools_pkey PRIMARY KEY (id)
);

CREATE INDEX schools_status_region ON schools USING btree (active, province, city, area, name, id);

CREATE TABLE sessions (
    token_hash text NOT NULL,
    account_id uuid NOT NULL,
    csrf text NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    kind text NOT NULL,
    login_origin text,
    CONSTRAINT sessions_client_origin CHECK ((((kind = 'admin_browser'::text) AND (login_origin IS NOT NULL) AND (length(login_origin) > 0)) OR ((kind = 'miniapp'::text) AND (login_origin IS NULL)))),
    CONSTRAINT sessions_kind_check CHECK ((kind = ANY (ARRAY['admin_browser'::text, 'miniapp'::text]))),
    CONSTRAINT sessions_pkey PRIMARY KEY (token_hash)
);

CREATE INDEX sessions_expiry ON sessions USING btree (expires_at);

CREATE TABLE site_settings (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    singleton boolean DEFAULT true NOT NULL,
    revision bigint DEFAULT 0 NOT NULL,
    site_name text,
    title_subtitle text,
    login_intro text,
    icp_number text,
    police_number text,
    police_url text,
    copyright_owner text,
    copyright_start_year integer,
    copyright_end_mode text,
    copyright_end_year integer,
    copyright_notice text,
    updated_by uuid,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    logo_asset_id uuid,
    CONSTRAINT site_settings_check CHECK (((police_number IS NULL) = (police_url IS NULL))),
    CONSTRAINT site_settings_check1 CHECK ((((copyright_owner IS NULL) AND (copyright_start_year IS NULL) AND (copyright_end_mode IS NULL) AND (copyright_end_year IS NULL) AND (copyright_notice IS NULL)) OR ((copyright_owner IS NOT NULL) AND (copyright_start_year IS NOT NULL) AND (copyright_end_mode IS NOT NULL)))),
    CONSTRAINT site_settings_check2 CHECK (((copyright_end_mode IS DISTINCT FROM 'fixed'::text) OR (copyright_end_year IS NOT NULL))),
    CONSTRAINT site_settings_check3 CHECK (((copyright_end_mode IS DISTINCT FROM 'auto'::text) OR (copyright_end_year IS NULL))),
    CONSTRAINT site_settings_check4 CHECK (((copyright_end_year IS NULL) OR (copyright_start_year IS NULL) OR (copyright_end_year >= copyright_start_year))),
    CONSTRAINT site_settings_copyright_end_mode_check CHECK ((copyright_end_mode = ANY (ARRAY['auto'::text, 'fixed'::text]))),
    CONSTRAINT site_settings_copyright_end_year_check CHECK (((copyright_end_year IS NULL) OR ((copyright_end_year >= 1900) AND (copyright_end_year <= 9999)))),
    CONSTRAINT site_settings_copyright_notice_check CHECK (((copyright_notice IS NULL) OR ((char_length(btrim(copyright_notice)) >= 1) AND (char_length(btrim(copyright_notice)) <= 200)))),
    CONSTRAINT site_settings_copyright_owner_check CHECK (((copyright_owner IS NULL) OR ((char_length(btrim(copyright_owner)) >= 1) AND (char_length(btrim(copyright_owner)) <= 100)))),
    CONSTRAINT site_settings_copyright_start_year_check CHECK (((copyright_start_year IS NULL) OR ((copyright_start_year >= 1900) AND (copyright_start_year <= 9999)))),
    CONSTRAINT site_settings_icp_number_check CHECK (((icp_number IS NULL) OR ((char_length(btrim(icp_number)) >= 1) AND (char_length(btrim(icp_number)) <= 64)))),
    CONSTRAINT site_settings_login_intro_check CHECK (((login_intro IS NULL) OR ((char_length(btrim(login_intro)) >= 1) AND (char_length(btrim(login_intro)) <= 100)))),
    CONSTRAINT site_settings_police_number_check CHECK (((police_number IS NULL) OR ((char_length(btrim(police_number)) >= 1) AND (char_length(btrim(police_number)) <= 64)))),
    CONSTRAINT site_settings_police_url_check CHECK (((police_url IS NULL) OR (((char_length(police_url) >= 1) AND (char_length(police_url) <= 2048)) AND (police_url ~~ 'https://%'::text)))),
    CONSTRAINT site_settings_revision_check CHECK ((revision >= 0)),
    CONSTRAINT site_settings_singleton_check CHECK (singleton),
    CONSTRAINT site_settings_site_name_check CHECK (((site_name IS NULL) OR ((char_length(btrim(site_name)) >= 1) AND (char_length(btrim(site_name)) <= 30)))),
    CONSTRAINT site_settings_title_subtitle_check CHECK (((title_subtitle IS NULL) OR ((char_length(btrim(title_subtitle)) >= 1) AND (char_length(btrim(title_subtitle)) <= 40)))),
    CONSTRAINT site_settings_pkey PRIMARY KEY (id),
    CONSTRAINT site_settings_singleton_key UNIQUE (singleton)
);

ALTER SEQUENCE legacy_public_id OWNED BY legacy_aliases.public_id;
