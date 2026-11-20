WITH windows AS (
 SELECT * FROM jsonb_to_recordset($1::jsonb) AS w(key text,start_at timestamptz,actual_end_at timestamptz,school_ids uuid[],full_site boolean,content boolean,registration boolean,activity boolean,cohort boolean,detail boolean)
)
SELECT jsonb_build_object('key',w.key,
 'unknown_assignments',w.registration AND EXISTS(SELECT 1 FROM statistics_registrations WHERE school_assignment='unknown' AND registered_at>=w.start_at AND registered_at<w.actual_end_at),
 'unknown_base',w.registration AND EXISTS(SELECT 1 FROM statistics_registrations WHERE school_assignment='unknown' AND registered_at<w.actual_end_at),
 'metrics',jsonb_build_object(
 'posts',CASE WHEN w.content THEN p.n END,'miniapp_posts',CASE WHEN w.content THEN p.miniapp END,'web_posts',CASE WHEN w.content THEN p.web END,'seed_posts',CASE WHEN w.content THEN p.seed END,'unknown_posts',CASE WHEN w.content THEN p.unknown END,'comments',CASE WHEN w.content THEN c.n END,
 'post_authors',CASE WHEN w.content AND w.detail THEN p.authors END,'comment_authors',CASE WHEN w.content AND w.detail THEN c.authors END,
 'participants',CASE WHEN w.content AND w.detail THEN (SELECT count(DISTINCT author_id) FROM (
   SELECT author_id FROM posts WHERE EXISTS(SELECT 1 FROM accounts WHERE id=posts.author_id AND account_kind='ordinary') AND w.content AND created_at>=w.start_at AND created_at<w.actual_end_at AND school_id=ANY(w.school_ids)
   UNION SELECT author_id FROM comments WHERE w.content AND created_at>=w.start_at AND created_at<w.actual_end_at AND school_id=ANY(w.school_ids)
 ) u) END,
 'registrations',CASE WHEN w.registration THEN r.n END,'cumulative_registrations',CASE WHEN w.registration AND w.detail THEN r.total END,
 'new_participants',CASE WHEN w.cohort THEN r.participants END,
 'active_users',CASE WHEN w.activity THEN a.n END,'registered_participants',CASE WHEN w.activity AND w.detail THEN a.participants END))
FROM windows w
CROSS JOIN LATERAL (
 SELECT count(*) n,count(*) FILTER(WHERE created_via='miniapp') miniapp,count(*) FILTER(WHERE created_via='web') web,count(*) FILTER(WHERE created_via='seed') seed,count(*) FILTER(WHERE created_via='legacy') unknown,count(DISTINCT author_id) FILTER(WHERE w.detail AND EXISTS(SELECT 1 FROM accounts WHERE id=posts.author_id AND account_kind='ordinary')) authors FROM posts
 WHERE w.content AND created_at>=w.start_at AND created_at<w.actual_end_at AND school_id=ANY(w.school_ids)
) p
CROSS JOIN LATERAL (
 SELECT count(*) n,count(DISTINCT author_id) FILTER(WHERE w.detail) authors FROM comments
 WHERE w.content AND created_at>=w.start_at AND created_at<w.actual_end_at AND school_id=ANY(w.school_ids)
) c
CROSS JOIN LATERAL (
 SELECT count(*) FILTER(WHERE registered_at>=w.start_at) n,count(*) total,
 count(*) FILTER(WHERE w.cohort AND registered_at>=w.start_at AND EXISTS (
  SELECT 1 FROM posts WHERE author_id=r.account_id AND created_at>=r.registered_at AND created_at>=w.start_at AND created_at<w.actual_end_at AND school_id=ANY(w.school_ids)
  UNION ALL SELECT 1 FROM comments WHERE author_id=r.account_id AND created_at>=r.registered_at AND created_at>=w.start_at AND created_at<w.actual_end_at AND school_id=ANY(w.school_ids)
 )) participants
 FROM statistics_registrations r WHERE w.registration AND registered_at<w.actual_end_at AND (w.detail OR registered_at>=w.start_at) AND time_quality='verified'
 AND (w.full_site OR registration_school_id=ANY(w.school_ids))
) r
CROSS JOIN LATERAL (
 SELECT count(DISTINCT account_id) n,count(DISTINCT account_id) FILTER(WHERE kind IN ('post_create','comment_create')) participants
 FROM statistics_activity_events WHERE w.activity AND occurred_at>=w.start_at AND occurred_at<w.actual_end_at
 AND (w.full_site OR school_id=ANY(w.school_ids))
) a
