CREATE TABLE statistics_activity_events (
    event_id uuid NOT NULL,
    account_id uuid NOT NULL,
    occurred_at timestamp with time zone NOT NULL,
    school_id uuid,
    post_id uuid,
    kind text NOT NULL,
    client_kind text DEFAULT 'miniapp'::text NOT NULL,
    recorded_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT statistics_activity_events_client_kind_check CHECK ((client_kind = 'miniapp'::text)),
    CONSTRAINT statistics_activity_events_kind_check CHECK ((kind = ANY (ARRAY['login'::text, 'feed_open'::text, 'post_open'::text, 'post_create'::text, 'comment_create'::text]))),
    CONSTRAINT statistics_activity_events_pkey PRIMARY KEY (event_id)
);

CREATE INDEX statistics_activity_school ON statistics_activity_events USING btree (school_id, occurred_at, account_id);

CREATE INDEX statistics_activity_time ON statistics_activity_events USING btree (occurred_at, account_id);

CREATE TABLE statistics_activity_runs (
    id uuid NOT NULL,
    started_at timestamp with time zone NOT NULL,
    sealed_at timestamp with time zone NOT NULL,
    heartbeat_at timestamp with time zone NOT NULL,
    closed_at timestamp with time zone,
    failed boolean DEFAULT false NOT NULL,
    CONSTRAINT statistics_activity_runs_pkey PRIMARY KEY (id)
);

CREATE TABLE statistics_coverage (
    id uuid NOT NULL,
    family text NOT NULL,
    start_at timestamp with time zone NOT NULL,
    end_at timestamp with time zone,
    status text NOT NULL,
    reason text NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT statistics_coverage_check CHECK (((end_at IS NULL) OR (end_at >= start_at))),
    CONSTRAINT statistics_coverage_family_check CHECK ((family = ANY (ARRAY['content'::text, 'registration'::text, 'activity'::text]))),
    CONSTRAINT statistics_coverage_status_check CHECK ((status = ANY (ARRAY['complete'::text, 'partial'::text, 'unknown'::text]))),
    CONSTRAINT statistics_coverage_pkey PRIMARY KEY (id)
);

CREATE TABLE statistics_registrations (
    account_id uuid NOT NULL,
    registered_at timestamp with time zone NOT NULL,
    registration_school_id uuid,
    school_assignment text NOT NULL,
    time_quality text NOT NULL,
    recorded_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT statistics_registrations_check CHECK (((school_assignment = 'known_school'::text) = (registration_school_id IS NOT NULL))),
    CONSTRAINT statistics_registrations_school_assignment_check CHECK ((school_assignment = ANY (ARRAY['known_school'::text, 'unassigned'::text, 'unknown'::text]))),
    CONSTRAINT statistics_registrations_time_quality_check CHECK ((time_quality = ANY (ARRAY['verified'::text, 'unknown'::text]))),
    CONSTRAINT statistics_registrations_pkey PRIMARY KEY (account_id)
);

CREATE INDEX statistics_registration_school ON statistics_registrations USING btree (registration_school_id, registered_at, account_id);

CREATE INDEX statistics_registration_time ON statistics_registrations USING btree (registered_at, account_id);

CREATE INDEX statistics_registration_unknown ON statistics_registrations USING btree (registered_at) WHERE (school_assignment = 'unknown'::text);

CREATE TABLE statistics_snapshots (
    id uuid NOT NULL,
    owner_id uuid NOT NULL,
    school_id uuid,
    as_of timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    definition_version integer NOT NULL,
    result jsonb NOT NULL,
    CONSTRAINT statistics_snapshots_pkey PRIMARY KEY (id)
);

CREATE INDEX statistics_snapshot_expiry ON statistics_snapshots USING btree (expires_at);
