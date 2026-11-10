ALTER TABLE school_resources
 ADD COLUMN popup_strategy TEXT NOT NULL DEFAULT 'daily' CHECK (popup_strategy IN ('always','daily','interval')),
 ADD COLUMN popup_interval_days INTEGER NOT NULL DEFAULT 1 CHECK (popup_interval_days BETWEEN 1 AND 365),
 ADD COLUMN views BIGINT NOT NULL DEFAULT 0 CHECK (views >= 0),
 ADD COLUMN deleted_at TIMESTAMPTZ;
CREATE INDEX operations_resource_scope ON school_resources(school_id,kind,sort_order DESC,id) WHERE deleted_at IS NULL;
CREATE TABLE customer_config (
 singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK(singleton),
 enabled BOOLEAN NOT NULL DEFAULT FALSE,
 business_asset_id UUID REFERENCES operational_media(id),
 join_asset_id UUID REFERENCES operational_media(id),
 service_asset_id UUID REFERENCES operational_media(id),
 fallback_asset_id UUID REFERENCES operational_media(id),
 note TEXT NOT NULL DEFAULT '添加客服微信后，请备注来意' CHECK(char_length(note)<=255),
 revision BIGINT NOT NULL DEFAULT 1,
 updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 updated_by UUID REFERENCES accounts(id)
);
-- Absence represents the default hidden configuration, keeping an empty target importable.
ALTER TABLE cases
 ADD COLUMN feedback_type TEXT CHECK (feedback_type IS NULL OR (kind='feedback' AND feedback_type IN ('product_suggestion','bug_report','other'))),
 ADD COLUMN submitted_ip INET,
 ADD COLUMN updated_at TIMESTAMPTZ,
 ADD COLUMN deleted_at TIMESTAMPTZ,
 ADD COLUMN deleted_by UUID REFERENCES accounts(id),
 ADD COLUMN submission_key UUID;
UPDATE cases SET updated_at=coalesce(handled_at,created_at);
ALTER TABLE cases ALTER COLUMN updated_at SET NOT NULL, ALTER COLUMN updated_at SET DEFAULT now();
CREATE UNIQUE INDEX feedback_submission ON cases(account_id,school_id,submission_key) WHERE submission_key IS NOT NULL;
CREATE INDEX feedback_management ON cases(school_id,feedback_type,state,created_at DESC,id) WHERE kind='feedback' AND deleted_at IS NULL;
CREATE TABLE banner_metric_events (
 event_id UUID PRIMARY KEY,
 resource_id UUID NOT NULL REFERENCES school_resources(id),
 school_id UUID NOT NULL REFERENCES schools(id),
 account_id UUID NOT NULL REFERENCES accounts(id),
 event_type TEXT NOT NULL CHECK(event_type IN ('view','click')),
 display_cycle UUID NOT NULL,
 occurred_at TIMESTAMPTZ NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX banner_view_once ON banner_metric_events(account_id,school_id,resource_id,display_cycle) WHERE event_type='view';
CREATE INDEX banner_event_retention ON banner_metric_events(created_at);
