CREATE TABLE file_asset_leases (
    asset_id uuid NOT NULL,
    owner_id uuid NOT NULL,
    draft_id uuid NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    CONSTRAINT file_asset_leases_pkey PRIMARY KEY (asset_id, owner_id, draft_id)
);

CREATE INDEX file_lease_expiry ON file_asset_leases USING btree (expires_at, asset_id);

CREATE TABLE file_asset_references (
    asset_id uuid NOT NULL,
    object_type text NOT NULL,
    object_id uuid NOT NULL,
    CONSTRAINT file_asset_references_pkey PRIMARY KEY (asset_id, object_type, object_id)
);

CREATE TABLE file_assets (
    id uuid NOT NULL,
    school_id uuid,
    owner_id uuid NOT NULL,
    purpose text NOT NULL,
    store_id uuid NOT NULL,
    mime_type text NOT NULL,
    size_bytes bigint NOT NULL,
    width integer NOT NULL,
    height integer NOT NULL,
    sha256 text NOT NULL,
    state text NOT NULL,
    idempotency_key uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    retry_at timestamp with time zone DEFAULT now() NOT NULL,
    attempts integer DEFAULT 0 NOT NULL,
    ever_bound boolean DEFAULT false NOT NULL,
    gc_after timestamp with time zone,
    artwork_permission text,
    CONSTRAINT file_assets_artwork_permission_check CHECK ((artwork_permission = ANY (ARRAY['resource.school.write'::text, 'school.profile.update'::text, 'operation.qrcode.create'::text, 'operation.qrcode.update'::text, 'operation.banner.create'::text, 'operation.banner.update'::text]))),
    CONSTRAINT file_assets_check CHECK ((((width)::bigint * height) <= 24000000)),
    CONSTRAINT file_assets_check1 CHECK (((purpose = ANY (ARRAY['avatar'::text, 'operation_artwork'::text])) OR (school_id IS NOT NULL))),
    CONSTRAINT file_assets_check2 CHECK (((purpose <> 'avatar'::text) OR (school_id IS NULL))),
    CONSTRAINT file_assets_height_check CHECK (((height >= 1) AND (height <= 8192))),
    CONSTRAINT file_assets_mime_type_check CHECK ((mime_type = ANY (ARRAY['image/png'::text, 'image/jpeg'::text, 'image/gif'::text, 'image/webp'::text]))),
    CONSTRAINT file_assets_purpose_check CHECK ((purpose = ANY (ARRAY['post_image'::text, 'feedback_evidence'::text, 'operation_artwork'::text, 'avatar'::text]))),
    CONSTRAINT file_assets_sha256_check CHECK ((sha256 ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT file_assets_size_bytes_check CHECK (((size_bytes >= 1) AND (size_bytes <= 5242880))),
    CONSTRAINT file_assets_state_check CHECK ((state = ANY (ARRAY['uploading'::text, 'ready'::text, 'pending_delete'::text, 'deleted'::text]))),
    CONSTRAINT file_assets_width_check CHECK (((width >= 1) AND (width <= 8192))),
    CONSTRAINT file_assets_pkey PRIMARY KEY (id)
);

CREATE INDEX file_cleanup ON file_assets USING btree (state, expires_at, retry_at);

CREATE INDEX file_gc_due ON file_assets USING btree (store_id, gc_after, id) WHERE (state = ANY (ARRAY['ready'::text, 'uploading'::text]));

CREATE INDEX file_gc_retry ON file_assets USING btree (store_id, retry_at, id) WHERE (state = 'pending_delete'::text);

CREATE UNIQUE INDEX file_upload_idempotency ON file_assets USING btree (owner_id, COALESCE(school_id, '00000000-0000-0000-0000-000000000000'::uuid), purpose, idempotency_key);

CREATE TABLE media_access_tokens (
    token_hash text NOT NULL,
    school_id uuid NOT NULL,
    asset_id uuid NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    post_id uuid NOT NULL,
    CONSTRAINT media_access_tokens_pkey PRIMARY KEY (token_hash)
);

CREATE INDEX media_access_expiry ON media_access_tokens USING btree (expires_at);

CREATE INDEX media_token_asset_expiry ON media_access_tokens USING btree (asset_id, expires_at);

CREATE TABLE media_assets (
    id uuid NOT NULL,
    school_id uuid NOT NULL,
    owner_id uuid NOT NULL,
    mime_type text NOT NULL,
    size_bytes bigint NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT media_assets_pkey PRIMARY KEY (id)
);

CREATE TABLE operational_media (
    id uuid NOT NULL,
    school_id uuid,
    owner_id uuid NOT NULL,
    mime_type text NOT NULL,
    size_bytes bigint NOT NULL,
    width integer NOT NULL,
    height integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT operational_media_check CHECK ((((width)::bigint * height) <= 24000000)),
    CONSTRAINT operational_media_height_check CHECK (((height >= 1) AND (height <= 8192))),
    CONSTRAINT operational_media_size_bytes_check CHECK (((size_bytes >= 1) AND (size_bytes <= 5242880))),
    CONSTRAINT operational_media_width_check CHECK (((width >= 1) AND (width <= 8192))),
    CONSTRAINT operational_media_pkey PRIMARY KEY (id),
    CONSTRAINT operational_media_school_id UNIQUE (school_id, id)
);

CREATE TABLE product_previews (
    id uuid NOT NULL,
    actor_id uuid NOT NULL,
    school_id uuid,
    kind text NOT NULL,
    payload jsonb NOT NULL,
    dependencies jsonb DEFAULT '{}'::jsonb NOT NULL,
    digest text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    expires_at timestamp with time zone DEFAULT (now() + '00:15:00'::interval) NOT NULL,
    result jsonb,
    confirmed_at timestamp with time zone,
    CONSTRAINT product_previews_check CHECK (((result IS NULL) = (confirmed_at IS NULL))),
    CONSTRAINT product_previews_pkey PRIMARY KEY (id)
);

CREATE INDEX product_preview_expiry ON product_previews USING btree (expires_at, id);

CREATE INDEX product_previews_expiry ON product_previews USING btree (expires_at) WHERE (result IS NULL);
