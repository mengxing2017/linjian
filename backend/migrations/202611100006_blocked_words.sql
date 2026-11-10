-- Stop before changing data if legacy records cannot satisfy the new model.
DO $$ BEGIN
 IF EXISTS (SELECT 1 FROM school_resources WHERE kind='blocked_word' AND (char_length(btrim(title)) NOT BETWEEN 1 AND 100 OR char_length(btrim(content)) > 255 OR title <> btrim(title))) THEN
   RAISE EXCEPTION '屏蔽词迁移失败：存在空词、超长词、超长备注或首尾空白，请先核对 school_resources';
 END IF;
 IF EXISTS (SELECT 1 FROM school_resources WHERE kind='blocked_word' GROUP BY school_id, lower(btrim(title)) HAVING count(*) > 1) THEN
   RAISE EXCEPTION '屏蔽词迁移失败：同一范围存在重复词，请先核对 school_resources';
 END IF;
END $$;

CREATE TABLE blocked_words (
 id UUID PRIMARY KEY, school_id UUID REFERENCES schools(id), word TEXT NOT NULL,
 remark TEXT NOT NULL DEFAULT '', active BOOLEAN NOT NULL DEFAULT TRUE,
 sort_order INTEGER NOT NULL DEFAULT 0, revision BIGINT NOT NULL DEFAULT 1,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(), updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 CHECK (char_length(word) BETWEEN 1 AND 100 AND word=btrim(word)),
 CHECK (char_length(remark)<=255)
);
CREATE UNIQUE INDEX blocked_words_global_unique ON blocked_words(lower(word)) WHERE school_id IS NULL;
CREATE UNIQUE INDEX blocked_words_school_unique ON blocked_words(school_id,lower(word)) WHERE school_id IS NOT NULL;
CREATE INDEX blocked_words_order ON blocked_words(school_id,sort_order DESC,id);
CREATE TABLE blocked_word_order_versions (scope TEXT PRIMARY KEY, revision BIGINT NOT NULL DEFAULT 1);
CREATE TABLE blocked_word_imports (
 id UUID PRIMARY KEY, school_id UUID REFERENCES schools(id), actor_id UUID NOT NULL REFERENCES accounts(id),
 digest TEXT NOT NULL, order_revision BIGINT NOT NULL, rows JSONB NOT NULL, errors JSONB NOT NULL,
 expires_at TIMESTAMPTZ NOT NULL DEFAULT now()+interval '30 minutes', result JSONB
);
CREATE TABLE blocked_word_legacy_snapshot AS SELECT * FROM school_resources WHERE kind='blocked_word';
INSERT INTO blocked_words(id,school_id,word,remark,active,sort_order,revision)
 SELECT id,school_id,title,content,active,sort_order,revision FROM school_resources WHERE kind='blocked_word';
-- Retain legacy rows because publication history references their IDs. They are
-- hidden from resource reads and no longer participate in content validation.
