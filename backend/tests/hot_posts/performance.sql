CREATE TEMP TABLE hot_fixture ON COMMIT DROP AS
SELECT gen_random_uuid() AS id, gen_random_uuid() AS event_id,
       gen_random_uuid() AS snapshot_id, sequence
FROM generate_series(1, 30000) AS sequence;

CREATE TEMP TABLE hot_readers ON COMMIT DROP AS
SELECT gen_random_uuid() AS id, sequence FROM generate_series(1, 10) AS sequence;

INSERT INTO accounts(id, username, display_name)
SELECT id, 'hot-reader-' || sequence, '合成测试读者' FROM hot_readers;

INSERT INTO posts(id, school_id, author_id, title, content, display_published_at)
SELECT id, CASE WHEN sequence <= 25000 THEN '{school}'::uuid ELSE '{other}'::uuid END,
       '{author}'::uuid, '合成性能测试帖', '合成性能测试正文',
       CASE WHEN sequence BETWEEN 10001 AND 20000
            THEN '{time}'::timestamptz - interval '30 days'
            ELSE '{time}'::timestamptz - interval '1 hour' END
FROM hot_fixture;

INSERT INTO moderation_events(id, school_id, actor_id, object_id, object_kind,
    decision, reason, decision_source, content_revision, review_digest)
SELECT fixture.event_id, post.school_id, '{author}'::uuid, post.id, 'post',
       'approved', '隔离性能测试', 'human', post.content_revision, repeat('a', 64)
FROM hot_fixture fixture JOIN posts post ON post.id=fixture.id
WHERE fixture.sequence NOT BETWEEN 20001 AND 25000;

INSERT INTO forum_publication_snapshots(id, school_id, post_id, author_id,
    content_revision, approved_post_revision, moderation_event_id, moderator_id,
    title, content, images, tags, content_digest, summary, summary_algorithm,
    preview_digest, link_policy, decision_source)
SELECT fixture.snapshot_id, post.school_id, post.id, post.author_id,
       post.content_revision, post.revision+1, fixture.event_id, '{author}'::uuid,
       post.title, post.content, post.images, post.tags, repeat('a', 64),
       post.content, 'plain-v1', repeat('a', 64), '{{}}'::jsonb, 'human'
FROM hot_fixture fixture JOIN posts post ON post.id=fixture.id
WHERE fixture.sequence NOT BETWEEN 20001 AND 25000;

UPDATE posts post SET audit_status='approved', approved_snapshot_id=fixture.snapshot_id,
    revision=post.revision+1
FROM hot_fixture fixture WHERE post.id=fixture.id
  AND fixture.sequence NOT BETWEEN 20001 AND 25000;

INSERT INTO post_reactions(school_id, post_id, account_id, kind)
SELECT '{school}'::uuid, fixture.id, reader.id, 'like'
FROM hot_fixture fixture CROSS JOIN hot_readers reader WHERE fixture.sequence<=10000;

INSERT INTO comments(id, school_id, post_id, author_id, content, audit_status)
SELECT gen_random_uuid(), '{school}'::uuid, fixture.id, '{author}'::uuid,
       '合成有效评论', 'approved'
FROM hot_fixture fixture CROSS JOIN generate_series(1, 10) WHERE fixture.sequence<=10000;

INSERT INTO comments(id, school_id, post_id, author_id, content, audit_status)
SELECT gen_random_uuid(), '{school}'::uuid, fixture.id, '{author}'::uuid,
       '合成热点评论', 'approved'
FROM hot_fixture fixture CROSS JOIN generate_series(1, 10000) WHERE fixture.sequence=1;
