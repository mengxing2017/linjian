CREATE TABLE forum_outbox (
    id uuid NOT NULL,
    school_id uuid NOT NULL,
    post_id uuid NOT NULL,
    sequence bigint NOT NULL,
    payload jsonb NOT NULL,
    request_body text,
    attempts integer DEFAULT 0 NOT NULL,
    next_attempt_at timestamp with time zone DEFAULT now() NOT NULL,
    receipt_id text,
    receipt_status text,
    receipt_summary jsonb,
    last_error text,
    accepted_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    connection_version uuid,
    source_id uuid,
    route_id text,
    schema_version integer DEFAULT 1 NOT NULL,
    signature_version integer DEFAULT 1 NOT NULL,
    body_sha256 text,
    blocked_reason text,
    blocked_at timestamp with time zone,
    auth_failure_count integer DEFAULT 0 NOT NULL,
    binding_version uuid,
    delivery_state text DEFAULT 'ready'::text NOT NULL,
    frozen_at timestamp with time zone,
    lease_token uuid,
    lease_until timestamp with time zone,
    recovery_reason text,
    source_origin text,
    publication_snapshot_id uuid,
    projection_version text DEFAULT 'message_v1'::text NOT NULL,
    message_version text,
    trigger_type text NOT NULL,
    CONSTRAINT forum_message_projection CHECK ((projection_version = 'message_v1'::text)),
    CONSTRAINT forum_outbox_complete_snapshot CHECK (((schema_version <> 2) OR (request_body IS NULL) OR ((connection_version IS NOT NULL) AND (source_id IS NOT NULL) AND (route_id IS NOT NULL) AND (body_sha256 IS NOT NULL)))),
    CONSTRAINT forum_outbox_delivery_state CHECK ((delivery_state = ANY (ARRAY['waiting_binding'::text, 'waiting_configuration'::text, 'paused'::text, 'preparing_content'::text, 'ready'::text, 'in_flight'::text, 'retry_wait'::text, 'blocked'::text, 'accepted'::text, 'cancelled'::text]))),
    CONSTRAINT forum_outbox_trigger_type_check CHECK ((trigger_type = ANY (ARRAY['review_publication'::text, 'manual_recommendation'::text]))),
    CONSTRAINT forum_prepared_before_delivery CHECK (((delivery_state <> ALL (ARRAY['ready'::text, 'in_flight'::text, 'retry_wait'::text, 'accepted'::text])) OR (request_body IS NOT NULL))),
    CONSTRAINT forum_received_receipt CHECK (((receipt_status IS NULL) OR (receipt_status = 'received'::text))),
    CONSTRAINT forum_outbox_pkey PRIMARY KEY (id)
);

CREATE UNIQUE INDEX forum_outbox_bound_version ON forum_outbox USING btree (school_id, post_id, sequence, source_origin, source_id, route_id);

CREATE INDEX forum_outbox_due ON forum_outbox USING btree (next_attempt_at, created_at);

CREATE INDEX forum_outbox_schedulable ON forum_outbox USING btree (next_attempt_at, id) WHERE (delivery_state = ANY (ARRAY['ready'::text, 'retry_wait'::text, 'in_flight'::text]));

CREATE INDEX forum_outbox_school_state ON forum_outbox USING btree (school_id, delivery_state, created_at);

CREATE UNIQUE INDEX forum_outbox_single_intent ON forum_outbox USING btree (school_id, post_id, sequence);

CREATE UNIQUE INDEX forum_outbox_unbound_version ON forum_outbox USING btree (school_id, post_id, sequence) WHERE (connection_version IS NULL);

CREATE INDEX forum_recommendation_history ON forum_outbox USING btree (school_id, post_id, created_at DESC, id DESC);

CREATE TABLE forum_post_recommendations (
    id uuid NOT NULL,
    school_id uuid NOT NULL,
    post_id uuid NOT NULL,
    actor_id uuid NOT NULL,
    reason text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    publication_snapshot_id uuid NOT NULL,
    post_revision bigint NOT NULL,
    content_revision bigint NOT NULL,
    idempotency_key uuid NOT NULL,
    request_digest text NOT NULL,
    event_id uuid NOT NULL,
    CONSTRAINT forum_post_recommendations_reason_check CHECK (((length(btrim(reason)) >= 1) AND (length(btrim(reason)) <= 500))),
    CONSTRAINT forum_post_recommendations_event_id_key UNIQUE (event_id),
    CONSTRAINT forum_post_recommendations_pkey PRIMARY KEY (id),
    CONSTRAINT forum_post_recommendations_school_id_actor_id_idempotency_k_key UNIQUE (school_id, actor_id, idempotency_key)
);

CREATE TABLE forum_publication_preparations (
    event_id uuid NOT NULL,
    snapshot_id uuid NOT NULL,
    state text DEFAULT 'pending'::text NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    attempts integer DEFAULT 0 NOT NULL,
    next_attempt_at timestamp with time zone DEFAULT now() NOT NULL,
    last_error_code text,
    lease_token uuid,
    lease_until timestamp with time zone,
    link_asset_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    completed_at timestamp with time zone,
    resolved_policy jsonb,
    CONSTRAINT forum_publication_preparations_state_check CHECK ((state = ANY (ARRAY['pending'::text, 'generating'::text, 'retry_wait'::text, 'blocked'::text, 'completed'::text, 'superseded'::text]))),
    CONSTRAINT forum_publication_preparations_pkey PRIMARY KEY (event_id)
);

CREATE INDEX forum_preparation_due ON forum_publication_preparations USING btree (next_attempt_at, event_id) WHERE (state = ANY (ARRAY['pending'::text, 'retry_wait'::text, 'generating'::text]));

CREATE TABLE forum_source_terminations (
    event_id uuid NOT NULL,
    actor_id uuid NOT NULL,
    reason text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT forum_source_terminations_reason_check CHECK (((length(btrim(reason)) >= 1) AND (length(btrim(reason)) <= 500))),
    CONSTRAINT forum_source_terminations_pkey PRIMARY KEY (event_id)
);

CREATE TABLE forum_source_versions (
    school_id uuid NOT NULL,
    post_id uuid NOT NULL,
    sequence bigint NOT NULL,
    available boolean NOT NULL,
    delivery_intent boolean DEFAULT true NOT NULL,
    CONSTRAINT forum_source_versions_sequence_check CHECK ((sequence > 0)),
    CONSTRAINT forum_source_versions_pkey PRIMARY KEY (school_id, post_id)
);

CREATE TABLE school_source_binding_versions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    binding_id uuid NOT NULL,
    school_id uuid NOT NULL,
    revision bigint NOT NULL,
    connection_version uuid NOT NULL,
    source_id uuid NOT NULL,
    origin text NOT NULL,
    route_id text NOT NULL,
    active boolean NOT NULL,
    checked_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    actor_id uuid,
    baseline boolean DEFAULT false NOT NULL,
    CONSTRAINT school_source_binding_versions_binding_id_revision_key UNIQUE (binding_id, revision),
    CONSTRAINT school_source_binding_versions_pkey PRIMARY KEY (id)
);

CREATE TABLE school_source_bindings (
    id uuid NOT NULL,
    school_id uuid NOT NULL,
    connection_id uuid NOT NULL,
    connection_version uuid NOT NULL,
    source_id uuid NOT NULL,
    origin text NOT NULL,
    route_id text NOT NULL,
    active boolean DEFAULT true NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    checked_at timestamp with time zone,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT school_source_bindings_one_per_school UNIQUE (school_id),
    CONSTRAINT school_source_bindings_origin_source_id_route_id_key UNIQUE (origin, source_id, route_id),
    CONSTRAINT school_source_bindings_pkey PRIMARY KEY (id),
    CONSTRAINT school_source_bindings_school_id_connection_id_route_id_key UNIQUE (school_id, connection_id, route_id)
);

CREATE TABLE source_active_configuration (
    singleton boolean DEFAULT true NOT NULL,
    connection_id uuid,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT source_active_configuration_singleton_check CHECK (singleton),
    CONSTRAINT source_active_configuration_pkey PRIMARY KEY (singleton)
);

CREATE TABLE source_connection_versions (
    id uuid NOT NULL,
    connection_id uuid NOT NULL,
    revision bigint NOT NULL,
    source_id uuid NOT NULL,
    origin text NOT NULL,
    schema_version integer DEFAULT 2 NOT NULL,
    signature_version integer DEFAULT 2 NOT NULL,
    key_id text NOT NULL,
    CONSTRAINT source_connection_versions_connection_id_revision_key UNIQUE (connection_id, revision),
    CONSTRAINT source_connection_versions_id_connection_id_source_id_origi_key UNIQUE (id, connection_id, source_id, origin),
    CONSTRAINT source_connection_versions_pkey PRIMARY KEY (id)
);

CREATE TABLE source_connections (
    id uuid NOT NULL,
    name text NOT NULL,
    source_id uuid NOT NULL,
    origin text NOT NULL,
    key_id text NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    checked_at timestamp with time zone,
    expires_at timestamp with time zone,
    capability jsonb,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT source_connections_id_source_id_origin_key UNIQUE (id, source_id, origin),
    CONSTRAINT source_connections_pkey PRIMARY KEY (id)
);

CREATE TABLE source_import_batches (
    id uuid NOT NULL,
    delivery_intent boolean DEFAULT false NOT NULL,
    CONSTRAINT source_import_batches_delivery_intent_check CHECK ((NOT delivery_intent)),
    CONSTRAINT source_import_batches_pkey PRIMARY KEY (id)
);

CREATE TABLE source_imported_posts (
    post_id uuid NOT NULL,
    batch_id uuid NOT NULL,
    CONSTRAINT source_imported_posts_pkey PRIMARY KEY (post_id)
);
