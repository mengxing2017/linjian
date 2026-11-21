CREATE TABLE customer_config (
    singleton boolean DEFAULT true NOT NULL,
    enabled boolean DEFAULT false NOT NULL,
    business_asset_id uuid,
    join_asset_id uuid,
    service_asset_id uuid,
    fallback_asset_id uuid,
    note text DEFAULT '添加客服微信后，请备注来意'::text NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_by uuid,
    CONSTRAINT customer_config_note_check CHECK ((char_length(note) <= 255)),
    CONSTRAINT customer_config_singleton_check CHECK (singleton),
    CONSTRAINT customer_config_pkey PRIMARY KEY (singleton)
);

CREATE TABLE marketing_events (
    event_id uuid NOT NULL,
    slot_id uuid NOT NULL,
    school_id uuid NOT NULL,
    account_id uuid NOT NULL,
    event_type text NOT NULL,
    occurred_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT marketing_events_event_type_check CHECK ((event_type = ANY (ARRAY['loaded'::text, 'error'::text, 'show_api_success'::text, 'closed_early'::text, 'client_completed'::text]))),
    CONSTRAINT marketing_events_pkey PRIMARY KEY (event_id)
);

CREATE INDEX marketing_event_retention ON marketing_events USING btree (created_at);

CREATE TABLE marketing_slots (
    id uuid NOT NULL,
    name text NOT NULL,
    ad_unit_id text NOT NULL,
    format text NOT NULL,
    reward_scene text,
    qualification text NOT NULL,
    active boolean NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    CONSTRAINT marketing_slots_check CHECK ((((format = 'custom'::text) AND (reward_scene IS NULL)) OR ((format = 'rewarded'::text) AND (reward_scene = 'service_theme'::text)))),
    CONSTRAINT marketing_slots_format_check CHECK ((format = ANY (ARRAY['custom'::text, 'rewarded'::text]))),
    CONSTRAINT marketing_slots_pkey PRIMARY KEY (id)
);

CREATE TABLE operation_action_events (
    event_id uuid NOT NULL,
    click_id uuid NOT NULL,
    stage text NOT NULL,
    attempt_id uuid NOT NULL,
    outcome text NOT NULL,
    occurred_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT operation_action_events_outcome_check CHECK ((outcome = ANY (ARRAY['api_success'::text, 'cancel'::text, 'fail'::text, 'copy_success'::text, 'qr_displayed'::text, 'webview_loaded'::text, 'webview_failed'::text]))),
    CONSTRAINT operation_action_events_stage_check CHECK ((stage = ANY (ARRAY['navigation_api'::text, 'clipboard_api'::text, 'qr_display'::text, 'webview_load'::text]))),
    CONSTRAINT operation_action_events_click_id_stage_key UNIQUE (click_id, stage),
    CONSTRAINT operation_action_events_pkey PRIMARY KEY (event_id)
);

CREATE TABLE operation_config_revisions (
    id uuid NOT NULL,
    revision bigint DEFAULT 0 NOT NULL,
    CONSTRAINT operation_config_revisions_pkey PRIMARY KEY (id)
);

CREATE TABLE operation_cooperations (
    id uuid NOT NULL,
    partner_id uuid NOT NULL,
    name text NOT NULL,
    school_ids uuid[] DEFAULT '{}'::uuid[] NOT NULL,
    model text NOT NULL,
    disclosure text NOT NULL,
    channel_code text NOT NULL,
    active boolean NOT NULL,
    starts_at timestamp with time zone,
    ends_at timestamp with time zone,
    revision bigint DEFAULT 1 NOT NULL,
    CONSTRAINT operation_cooperations_check CHECK (((starts_at IS NULL) OR (ends_at IS NULL) OR (starts_at < ends_at))),
    CONSTRAINT operation_cooperations_model_check CHECK ((model = ANY (ARRAY['service'::text, 'sponsor'::text, 'cps'::text]))),
    CONSTRAINT operation_cooperations_pkey PRIMARY KEY (id)
);

CREATE TABLE operation_daily_metrics (
    school_id uuid NOT NULL,
    day date NOT NULL,
    resource_id uuid NOT NULL,
    channel text NOT NULL,
    target_version text NOT NULL,
    dimension_key text NOT NULL,
    dimensions jsonb NOT NULL,
    views bigint DEFAULT 0 NOT NULL,
    clicks bigint DEFAULT 0 NOT NULL,
    api_success bigint DEFAULT 0 NOT NULL,
    cancel bigint DEFAULT 0 NOT NULL,
    fail bigint DEFAULT 0 NOT NULL,
    copy_success bigint DEFAULT 0 NOT NULL,
    qr_displayed bigint DEFAULT 0 NOT NULL,
    webview_loaded bigint DEFAULT 0 NOT NULL,
    webview_failed bigint DEFAULT 0 NOT NULL,
    CONSTRAINT operation_daily_metrics_pkey PRIMARY KEY (school_id, day, resource_id, channel, target_version, dimension_key)
);

CREATE TABLE operation_metric_events (
    event_id uuid NOT NULL,
    resource_id uuid NOT NULL,
    school_id uuid NOT NULL,
    account_id uuid NOT NULL,
    event_type text NOT NULL,
    display_cycle uuid NOT NULL,
    occurred_at timestamp with time zone NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    channel text,
    target_version_id uuid,
    dimensions jsonb DEFAULT '{}'::jsonb NOT NULL,
    placement text NOT NULL,
    attempt_id uuid,
    CONSTRAINT banner_metric_events_event_type_check CHECK ((event_type = ANY (ARRAY['view'::text, 'click'::text]))),
    CONSTRAINT operation_attempt_kind CHECK ((((event_type = 'view'::text) AND (attempt_id IS NULL)) OR ((event_type = 'click'::text) AND (attempt_id IS NOT NULL)))),
    CONSTRAINT operation_metric_events_placement_check CHECK ((placement = ANY (ARRAY['home_banner'::text, 'home_shortcut'::text, 'more_services'::text, 'service_detail'::text]))),
    CONSTRAINT banner_metric_events_pkey PRIMARY KEY (event_id)
);

CREATE INDEX banner_event_retention ON operation_metric_events USING btree (created_at);

CREATE UNIQUE INDEX operation_attempt_once ON operation_metric_events USING btree (account_id, school_id, attempt_id) WHERE (attempt_id IS NOT NULL);

CREATE UNIQUE INDEX operation_view_once ON operation_metric_events USING btree (account_id, school_id, resource_id, placement, display_cycle) WHERE (event_type = 'view'::text);

CREATE TABLE operation_partners (
    id uuid NOT NULL,
    name text NOT NULL,
    contact text NOT NULL,
    active boolean NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    CONSTRAINT operation_partners_pkey PRIMARY KEY (id)
);

CREATE TABLE operation_report_snapshots (
    id uuid NOT NULL,
    school_id uuid,
    actor_id uuid NOT NULL,
    report jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    expires_at timestamp with time zone DEFAULT (now() + '00:10:00'::interval) NOT NULL,
    CONSTRAINT operation_report_snapshots_pkey PRIMARY KEY (id)
);

CREATE TABLE operation_service_categories (
    id uuid NOT NULL,
    school_id uuid,
    name text NOT NULL,
    active boolean NOT NULL,
    sort_order integer NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    CONSTRAINT operation_service_categories_name_check CHECK (((char_length(btrim(name)) >= 1) AND (char_length(btrim(name)) <= 40))),
    CONSTRAINT operation_service_categories_pkey PRIMARY KEY (id)
);

CREATE TABLE operation_service_targets (
    id uuid NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    CONSTRAINT operation_service_targets_pkey PRIMARY KEY (id)
);

CREATE TABLE operation_target_reviews (
    id uuid NOT NULL,
    version_id uuid NOT NULL,
    actor_id uuid NOT NULL,
    decision text NOT NULL,
    reason text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT operation_target_reviews_decision_check CHECK ((decision = ANY (ARRAY['submit'::text, 'approve'::text, 'reject'::text, 'withdraw'::text]))),
    CONSTRAINT operation_target_reviews_pkey PRIMARY KEY (id)
);

CREATE TABLE operation_target_versions (
    id uuid NOT NULL,
    target_id uuid NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    version_number bigint NOT NULL,
    content_digest text NOT NULL,
    config jsonb NOT NULL,
    state text NOT NULL,
    creator_id uuid NOT NULL,
    submitter_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT operation_target_versions_state_check CHECK ((state = ANY (ARRAY['draft'::text, 'pending'::text, 'approved'::text, 'rejected'::text, 'withdrawn'::text]))),
    CONSTRAINT operation_target_versions_pkey PRIMARY KEY (id),
    CONSTRAINT operation_target_versions_target_id_version_number_key UNIQUE (target_id, version_number)
);

CREATE UNIQUE INDEX service_target_one_approved ON operation_target_versions USING btree (target_id) WHERE (state = 'approved'::text);

CREATE TABLE resource_publications (
    id uuid NOT NULL,
    resource_id uuid NOT NULL,
    school_id uuid,
    actor_id uuid NOT NULL,
    snapshot jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT resource_publications_pkey PRIMARY KEY (id)
);

CREATE TABLE school_marketing_settings (
    id uuid NOT NULL,
    school_id uuid NOT NULL,
    slot_id uuid NOT NULL,
    enabled boolean NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    CONSTRAINT school_marketing_settings_pkey PRIMARY KEY (id),
    CONSTRAINT school_marketing_settings_school_id_slot_id_key UNIQUE (school_id, slot_id)
);

CREATE TABLE school_resources (
    id uuid NOT NULL,
    school_id uuid,
    kind text NOT NULL,
    title text NOT NULL,
    content text DEFAULT ''::text NOT NULL,
    active boolean DEFAULT true NOT NULL,
    sort_order integer DEFAULT 0 NOT NULL,
    starts_at timestamp with time zone,
    ends_at timestamp with time zone,
    revision bigint DEFAULT 1 NOT NULL,
    clicks bigint DEFAULT 0 NOT NULL,
    image_ref jsonb,
    action jsonb DEFAULT '{"kind": "none"}'::jsonb NOT NULL,
    popup_strategy text DEFAULT 'daily'::text NOT NULL,
    popup_interval_days integer DEFAULT 1 NOT NULL,
    views bigint DEFAULT 0 NOT NULL,
    deleted_at timestamp with time zone,
    subtitle text DEFAULT ''::text NOT NULL,
    badge text DEFAULT ''::text NOT NULL,
    featured boolean DEFAULT false NOT NULL,
    category_id uuid,
    cooperation_id uuid,
    target_version_id uuid,
    CONSTRAINT banner_target_boundary CHECK (((kind <> 'banner'::text) OR ((target_version_id IS NULL) AND (cooperation_id IS NULL)))),
    CONSTRAINT operation_action_boundary CHECK (COALESCE((((kind <> 'banner'::text) OR ((action ->> 'kind'::text) = ANY (ARRAY['none'::text, 'internal_page'::text]))) AND ((kind <> 'qrcode'::text) OR ((action ->> 'kind'::text) = 'none'::text)) AND ((kind <> 'shortcut'::text) OR ((action ->> 'kind'::text) = ANY (ARRAY['none'::text, 'internal_page'::text, 'target_channel'::text])))), false)),
    CONSTRAINT school_resources_check CHECK (((ends_at IS NULL) OR (starts_at IS NULL) OR (ends_at > starts_at))),
    CONSTRAINT school_resources_kind_check CHECK ((kind = ANY (ARRAY['banner'::text, 'qrcode'::text, 'service'::text, 'legal'::text, 'tag'::text, 'blocked_word'::text, 'shortcut'::text]))),
    CONSTRAINT school_resources_popup_interval_days_check CHECK (((popup_interval_days >= 1) AND (popup_interval_days <= 365))),
    CONSTRAINT school_resources_popup_strategy_check CHECK ((popup_strategy = ANY (ARRAY['always'::text, 'daily'::text, 'interval'::text]))),
    CONSTRAINT school_resources_views_check CHECK ((views >= 0)),
    CONSTRAINT shortcut_target_reference CHECK (COALESCE(((kind <> 'shortcut'::text) OR ((((action ->> 'kind'::text) = 'target_channel'::text) AND (target_version_id IS NOT NULL) AND ((target_version_id)::text = (action ->> 'version_id'::text))) OR (((action ->> 'kind'::text) = ANY (ARRAY['none'::text, 'internal_page'::text])) AND (target_version_id IS NULL)))), false)),
    CONSTRAINT school_resources_pkey PRIMARY KEY (id)
);

CREATE INDEX operations_resource_scope ON school_resources USING btree (school_id, kind, sort_order DESC, id) WHERE (deleted_at IS NULL);

CREATE TABLE school_shortcut_overrides (
    id uuid NOT NULL,
    school_id uuid NOT NULL,
    resource_id uuid NOT NULL,
    presentation jsonb NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    CONSTRAINT school_shortcut_overrides_pkey PRIMARY KEY (id),
    CONSTRAINT school_shortcut_overrides_school_id_resource_id_key UNIQUE (school_id, resource_id)
);
