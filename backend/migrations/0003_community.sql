CREATE TABLE blocked_word_imports (
    id uuid NOT NULL,
    school_id uuid,
    actor_id uuid NOT NULL,
    digest text NOT NULL,
    order_revision bigint NOT NULL,
    rows jsonb NOT NULL,
    errors jsonb NOT NULL,
    expires_at timestamp with time zone DEFAULT (now() + '00:30:00'::interval) NOT NULL,
    result jsonb,
    CONSTRAINT blocked_word_imports_pkey PRIMARY KEY (id)
);

CREATE TABLE blocked_word_order_versions (
    scope text NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    CONSTRAINT blocked_word_order_versions_pkey PRIMARY KEY (scope)
);

CREATE TABLE blocked_words (
    id uuid NOT NULL,
    school_id uuid,
    word text NOT NULL,
    remark text DEFAULT ''::text NOT NULL,
    active boolean DEFAULT true NOT NULL,
    sort_order integer DEFAULT 0 NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT blocked_words_remark_check CHECK ((char_length(remark) <= 255)),
    CONSTRAINT blocked_words_word_check CHECK ((((char_length(word) >= 1) AND (char_length(word) <= 100)) AND (word = btrim(word)))),
    CONSTRAINT blocked_words_pkey PRIMARY KEY (id)
);

CREATE UNIQUE INDEX blocked_words_global_unique ON blocked_words USING btree (lower(word)) WHERE (school_id IS NULL);

CREATE INDEX blocked_words_order ON blocked_words USING btree (school_id, sort_order DESC, id);

CREATE UNIQUE INDEX blocked_words_school_unique ON blocked_words USING btree (school_id, lower(word)) WHERE (school_id IS NOT NULL);

CREATE TABLE cases (
    id uuid NOT NULL,
    school_id uuid NOT NULL,
    account_id uuid NOT NULL,
    kind text NOT NULL,
    post_id uuid,
    content text NOT NULL,
    contact text DEFAULT ''::text NOT NULL,
    state text DEFAULT 'open'::text NOT NULL,
    resolution text DEFAULT ''::text NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    images jsonb DEFAULT '[]'::jsonb NOT NULL,
    handled_at timestamp with time zone,
    handled_by uuid,
    feedback_type text,
    submitted_ip inet,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    deleted_at timestamp with time zone,
    deleted_by uuid,
    submission_key uuid,
    CONSTRAINT cases_check CHECK (((feedback_type IS NULL) OR ((kind = 'feedback'::text) AND (feedback_type = ANY (ARRAY['product_suggestion'::text, 'bug_report'::text, 'other'::text]))))),
    CONSTRAINT cases_handling_metadata CHECK ((((handled_at IS NULL) = (handled_by IS NULL)) AND ((state <> 'open'::text) OR (handled_at IS NULL)))),
    CONSTRAINT cases_kind_check CHECK ((kind = ANY (ARRAY['report'::text, 'feedback'::text]))),
    CONSTRAINT cases_state_check CHECK ((state = ANY (ARRAY['open'::text, 'resolved'::text, 'dismissed'::text]))),
    CONSTRAINT cases_pkey PRIMARY KEY (id)
);

CREATE INDEX cases_school ON cases USING btree (school_id, state, created_at DESC);

CREATE INDEX feedback_management ON cases USING btree (school_id, feedback_type, state, created_at DESC, id) WHERE ((kind = 'feedback'::text) AND (deleted_at IS NULL));

CREATE UNIQUE INDEX feedback_submission ON cases USING btree (account_id, school_id, submission_key) WHERE (submission_key IS NOT NULL);

CREATE TABLE comment_reactions (
    school_id uuid NOT NULL,
    comment_id uuid NOT NULL,
    account_id uuid NOT NULL,
    CONSTRAINT comment_reactions_pkey PRIMARY KEY (comment_id, account_id)
);

CREATE TABLE comments (
    id uuid NOT NULL,
    school_id uuid NOT NULL,
    post_id uuid NOT NULL,
    author_id uuid NOT NULL,
    parent_id uuid,
    content text NOT NULL,
    audit_status text DEFAULT 'pending'::text NOT NULL,
    deleted boolean DEFAULT false NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    visibility text DEFAULT 'visible'::text NOT NULL,
    decision_revision bigint DEFAULT 1 NOT NULL,
    CONSTRAINT comments_audit_status_check CHECK ((audit_status = ANY (ARRAY['pending'::text, 'approved'::text, 'rejected'::text]))),
    CONSTRAINT comments_visibility_check CHECK ((visibility = ANY (ARRAY['visible'::text, 'hidden'::text]))),
    CONSTRAINT comments_pkey PRIMARY KEY (id)
);

CREATE INDEX comment_hot_valid ON comments USING btree (school_id, post_id) WHERE ((audit_status = 'approved'::text) AND (visibility = 'visible'::text) AND (NOT deleted));

CREATE INDEX comment_management_visibility ON comments USING btree (school_id, visibility, created_at DESC, id DESC) WHERE (NOT deleted);

CREATE INDEX comment_post ON comments USING btree (school_id, post_id, created_at);

CREATE INDEX statistics_comments_author_time ON comments USING btree (author_id, created_at, school_id);

CREATE INDEX statistics_comments_time ON comments USING btree (school_id, created_at);

CREATE TABLE follows (
    school_id uuid NOT NULL,
    follower_id uuid NOT NULL,
    target_id uuid NOT NULL,
    CONSTRAINT follows_check CHECK ((follower_id <> target_id)),
    CONSTRAINT follows_pkey PRIMARY KEY (school_id, follower_id, target_id)
);

CREATE TABLE moderation_events (
    id uuid NOT NULL,
    school_id uuid NOT NULL,
    actor_id uuid,
    object_id uuid NOT NULL,
    object_kind text NOT NULL,
    decision text NOT NULL,
    reason text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    decision_source text DEFAULT 'human'::text NOT NULL,
    content_revision bigint,
    policy_revision bigint,
    policy_mode text,
    review_digest text,
    CONSTRAINT moderation_evidence CHECK (((decision_source = 'legacy'::text) OR ((decision_source = 'human'::text) AND (actor_id IS NOT NULL) AND COALESCE((content_revision > 0), false)) OR ((decision_source = 'automatic'::text) AND (actor_id IS NULL) AND (decision = 'approved'::text) AND COALESCE(((content_revision > 0) AND (policy_revision > 0) AND (policy_mode = 'auto'::text)), false) AND (review_digest IS NOT NULL)))),
    CONSTRAINT moderation_events_pkey PRIMARY KEY (id)
);

CREATE INDEX moderation_object_history ON moderation_events USING btree (school_id, object_kind, object_id, created_at DESC, id DESC);

CREATE TABLE notifications (
    id uuid NOT NULL,
    school_id uuid NOT NULL,
    account_id uuid NOT NULL,
    kind text NOT NULL,
    object_id uuid,
    read_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT notifications_pkey PRIMARY KEY (id)
);

CREATE INDEX notification_owner ON notifications USING btree (school_id, account_id, created_at DESC);

CREATE TABLE post_import_jobs (
    id uuid NOT NULL,
    school_id uuid NOT NULL,
    actor_id uuid NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    digest text NOT NULL,
    rows jsonb NOT NULL,
    errors jsonb NOT NULL,
    state text DEFAULT 'preview'::text NOT NULL,
    result jsonb DEFAULT '[]'::jsonb NOT NULL,
    idempotency_key uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    expires_at timestamp with time zone DEFAULT (now() + '00:30:00'::interval) NOT NULL,
    CONSTRAINT post_import_jobs_state_check CHECK ((state = ANY (ARRAY['preview'::text, 'committed'::text, 'expired'::text]))),
    CONSTRAINT post_import_jobs_actor_id_school_id_idempotency_key_key UNIQUE (actor_id, school_id, idempotency_key),
    CONSTRAINT post_import_jobs_pkey PRIMARY KEY (id)
);

CREATE INDEX post_import_expiration ON post_import_jobs USING btree (expires_at) WHERE (state = 'preview'::text);

CREATE TABLE post_order_versions (
    school_id uuid NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    CONSTRAINT post_order_versions_pkey PRIMARY KEY (school_id)
);

CREATE TABLE post_reactions (
    school_id uuid NOT NULL,
    post_id uuid NOT NULL,
    account_id uuid NOT NULL,
    kind text NOT NULL,
    CONSTRAINT post_reactions_kind_check CHECK ((kind = ANY (ARRAY['like'::text, 'collect'::text]))),
    CONSTRAINT post_reactions_pkey PRIMARY KEY (post_id, account_id, kind)
);

CREATE INDEX reaction_hot_likes ON post_reactions USING btree (school_id, post_id) WHERE (kind = 'like'::text);

CREATE TABLE post_seed_batches (
    id uuid NOT NULL,
    school_id uuid NOT NULL,
    created_by uuid NOT NULL,
    name text NOT NULL,
    state text DEFAULT 'preview'::text NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    preview_digest text NOT NULL,
    policy_digest text NOT NULL,
    preview_payload jsonb NOT NULL,
    expires_at timestamp with time zone DEFAULT (now() + '00:30:00'::interval) NOT NULL,
    commit_idempotency_key uuid,
    commit_request_digest text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    committed_at timestamp with time zone,
    revoked_at timestamp with time zone,
    revoked_by uuid,
    revoke_reason text,
    CONSTRAINT post_seed_batches_check CHECK (((state = 'preview'::text) = (committed_at IS NULL))),
    CONSTRAINT post_seed_batches_check1 CHECK (((state = 'revoked'::text) = (revoked_at IS NOT NULL))),
    CONSTRAINT post_seed_batches_check2 CHECK (((state = 'preview'::text) = (commit_idempotency_key IS NULL))),
    CONSTRAINT post_seed_batches_name_check CHECK (((length(btrim(name)) >= 1) AND (length(btrim(name)) <= 100))),
    CONSTRAINT post_seed_batches_state_check CHECK ((state = ANY (ARRAY['preview'::text, 'committed'::text, 'revoked'::text]))),
    CONSTRAINT post_seed_batches_pkey PRIMARY KEY (id),
    CONSTRAINT post_seed_batches_school_id_created_by_commit_idempotency_k_key UNIQUE (school_id, created_by, commit_idempotency_key),
    CONSTRAINT post_seed_batches_school_id_id_key UNIQUE (school_id, id)
);

CREATE INDEX post_seed_batch_list ON post_seed_batches USING btree (school_id, created_at DESC, id);

CREATE TABLE post_seed_items (
    batch_id uuid NOT NULL,
    school_id uuid NOT NULL,
    row_number integer NOT NULL,
    post_id uuid NOT NULL,
    author_id uuid NOT NULL,
    CONSTRAINT post_seed_items_row_number_check CHECK (((row_number >= 1) AND (row_number <= 50))),
    CONSTRAINT post_seed_items_pkey PRIMARY KEY (batch_id, row_number),
    CONSTRAINT post_seed_items_post_id_key UNIQUE (post_id)
);

CREATE TABLE posts (
    id uuid NOT NULL,
    school_id uuid NOT NULL,
    author_id uuid NOT NULL,
    title text NOT NULL,
    content text NOT NULL,
    images jsonb DEFAULT '[]'::jsonb NOT NULL,
    tags jsonb DEFAULT '[]'::jsonb NOT NULL,
    audit_status text DEFAULT 'pending'::text NOT NULL,
    visibility text DEFAULT 'visible'::text NOT NULL,
    top_sort integer DEFAULT 0 NOT NULL,
    browse bigint DEFAULT 0 NOT NULL,
    revision bigint DEFAULT 1 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    created_via text DEFAULT 'legacy'::text NOT NULL,
    content_revision bigint DEFAULT 1 NOT NULL,
    approved_snapshot_id uuid,
    source_projection text DEFAULT 'message_v1'::text NOT NULL,
    is_top boolean DEFAULT false NOT NULL,
    sort_order bigint DEFAULT 0 NOT NULL,
    seed_batch_id uuid,
    display_published_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT posts_audit_status_check CHECK ((audit_status = ANY (ARRAY['pending'::text, 'approved'::text, 'rejected'::text]))),
    CONSTRAINT posts_check CHECK (((created_via = 'seed'::text) = (seed_batch_id IS NOT NULL))),
    CONSTRAINT posts_content_revision_check CHECK ((content_revision > 0)),
    CONSTRAINT posts_created_via_check CHECK ((created_via = ANY (ARRAY['legacy'::text, 'web'::text, 'miniapp'::text, 'seed'::text]))),
    CONSTRAINT posts_source_projection_check CHECK ((source_projection = 'message_v1'::text)),
    CONSTRAINT posts_visibility_check CHECK ((visibility = ANY (ARRAY['visible'::text, 'hidden'::text, 'offline'::text, 'deleted'::text]))),
    CONSTRAINT posts_pkey PRIMARY KEY (id),
    CONSTRAINT posts_school_id_id_key UNIQUE (school_id, id)
);

CREATE INDEX post_display_feed ON posts USING btree (school_id, is_top DESC, sort_order DESC, display_published_at DESC, id);

CREATE INDEX post_feed ON posts USING btree (school_id, audit_status, visibility, top_sort DESC, created_at DESC, id);

CREATE INDEX post_hot_public_window ON posts USING btree (school_id, display_published_at DESC, id) INCLUDE (title) WHERE ((audit_status = 'approved'::text) AND (visibility = 'visible'::text));

CREATE INDEX post_management_order ON posts USING btree (school_id, is_top DESC, sort_order DESC, created_at DESC, id) WHERE (visibility <> 'deleted'::text);

CREATE INDEX post_public_order ON posts USING btree (school_id, is_top DESC, sort_order DESC, created_at DESC, id) WHERE ((visibility = 'visible'::text) AND (audit_status = 'approved'::text));

CREATE INDEX post_seed_batch_posts ON posts USING btree (seed_batch_id, id) WHERE (seed_batch_id IS NOT NULL);

CREATE INDEX posts_user_school_counts ON posts USING btree (school_id, author_id) WHERE (visibility <> 'deleted'::text);

CREATE INDEX statistics_posts_author_time ON posts USING btree (author_id, created_at, school_id);

CREATE INDEX statistics_posts_time ON posts USING btree (school_id, created_at);

CREATE TABLE forum_publication_snapshots (
    id uuid NOT NULL,
    school_id uuid NOT NULL,
    post_id uuid NOT NULL,
    author_id uuid NOT NULL,
    content_revision bigint NOT NULL,
    approved_post_revision bigint NOT NULL,
    moderation_event_id uuid NOT NULL,
    moderator_id uuid,
    approved_at timestamp with time zone DEFAULT now() NOT NULL,
    title text NOT NULL,
    content text NOT NULL,
    images jsonb NOT NULL,
    tags jsonb NOT NULL,
    content_format text DEFAULT 'plain'::text NOT NULL,
    content_digest text NOT NULL,
    summary text NOT NULL,
    summary_algorithm text NOT NULL,
    preview_digest text NOT NULL,
    link_policy jsonb NOT NULL,
    decision_source text DEFAULT 'human'::text NOT NULL,
    policy_revision bigint,
    CONSTRAINT forum_publication_snapshots_content_format_check CHECK ((content_format = 'plain'::text)),
    CONSTRAINT forum_publication_snapshots_summary_algorithm_check CHECK ((summary_algorithm = 'plain-v1'::text)),
    CONSTRAINT publication_decision_source CHECK (((decision_source = 'legacy'::text) OR ((decision_source = 'human'::text) AND (moderator_id IS NOT NULL)) OR ((decision_source = 'automatic'::text) AND (moderator_id IS NULL) AND COALESCE((policy_revision > 0), false)))),
    CONSTRAINT forum_publication_snapshots_pkey PRIMARY KEY (id),
    CONSTRAINT forum_publication_snapshots_post_id_approved_post_revision_key UNIQUE (post_id, approved_post_revision),
    CONSTRAINT forum_publication_snapshots_school_id_post_id_id_key UNIQUE (school_id, post_id, id)
);
