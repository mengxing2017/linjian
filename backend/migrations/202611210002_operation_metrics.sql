ALTER TABLE banner_metric_events RENAME TO operation_metric_events;
ALTER TABLE operation_metric_events ADD COLUMN channel text, ADD COLUMN target_version_id uuid REFERENCES operation_target_versions(id), ADD COLUMN dimensions jsonb NOT NULL DEFAULT '{}';
ALTER TABLE operation_metric_events ADD COLUMN placement text NOT NULL CHECK(placement IN ('home_banner','home_shortcut','more_services','service_detail')), ADD COLUMN attempt_id uuid;
ALTER TABLE operation_metric_events ADD CONSTRAINT operation_attempt_kind CHECK((event_type='view' AND attempt_id IS NULL) OR (event_type='click' AND attempt_id IS NOT NULL));
CREATE UNIQUE INDEX operation_attempt_once ON operation_metric_events(account_id,school_id,attempt_id) WHERE attempt_id IS NOT NULL;
DROP INDEX banner_view_once;
CREATE UNIQUE INDEX operation_view_once ON operation_metric_events(account_id,school_id,resource_id,placement,display_cycle) WHERE event_type='view';
CREATE TABLE operation_action_events (
 event_id uuid PRIMARY KEY, click_id uuid NOT NULL REFERENCES operation_metric_events(event_id) ON DELETE CASCADE,
 stage text NOT NULL CHECK(stage IN ('navigation_api','clipboard_api','qr_display','webview_load')), attempt_id uuid NOT NULL,
 outcome text NOT NULL CHECK(outcome IN ('api_success','cancel','fail','copy_success','qr_displayed','webview_loaded','webview_failed')), occurred_at timestamptz NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(), UNIQUE(click_id,stage)
);
CREATE TABLE operation_daily_metrics (
 school_id uuid NOT NULL REFERENCES schools(id), day date NOT NULL, resource_id uuid NOT NULL REFERENCES school_resources(id),
 channel text NOT NULL, target_version text NOT NULL, dimension_key text NOT NULL, dimensions jsonb NOT NULL,
 views bigint NOT NULL DEFAULT 0, clicks bigint NOT NULL DEFAULT 0, api_success bigint NOT NULL DEFAULT 0,
 cancel bigint NOT NULL DEFAULT 0, fail bigint NOT NULL DEFAULT 0, copy_success bigint NOT NULL DEFAULT 0, qr_displayed bigint NOT NULL DEFAULT 0,
 webview_loaded bigint NOT NULL DEFAULT 0, webview_failed bigint NOT NULL DEFAULT 0,
 PRIMARY KEY(school_id,day,resource_id,channel,target_version,dimension_key)
);
CREATE TABLE operation_report_snapshots (
 id uuid PRIMARY KEY, school_id uuid REFERENCES schools(id), actor_id uuid NOT NULL REFERENCES accounts(id),
 report jsonb NOT NULL, created_at timestamptz NOT NULL DEFAULT now(), expires_at timestamptz NOT NULL DEFAULT now()+interval '10 minutes'
);
CREATE TABLE marketing_events (
 event_id uuid PRIMARY KEY, slot_id uuid NOT NULL REFERENCES marketing_slots(id), school_id uuid NOT NULL REFERENCES schools(id),
 account_id uuid NOT NULL REFERENCES accounts(id), event_type text NOT NULL CHECK(event_type IN ('loaded','error','show_api_success','closed_early','client_completed')),
 occurred_at timestamptz NOT NULL, created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX marketing_event_retention ON marketing_events(created_at);
