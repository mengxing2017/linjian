WITH windows AS (
 SELECT * FROM jsonb_to_recordset($1::jsonb) AS w(key text,start_at timestamptz,actual_end_at timestamptz,school_ids uuid[],full_site boolean)
)
SELECT jsonb_build_object('key',w.key,
 'unknown_assignments',EXISTS(SELECT 1 FROM statistics_registrations WHERE school_assignment='unknown' AND registered_at>=w.start_at AND registered_at<w.actual_end_at),
 'unknown_base',EXISTS(SELECT 1 FROM statistics_registrations WHERE school_assignment='unknown' AND registered_at<w.actual_end_at),
 'metrics',jsonb_build_object(
 'posts',p.n,'comments',c.n,'post_authors',p.authors,'comment_authors',c.authors,
 'participants',(SELECT count(DISTINCT author_id) FROM (
   SELECT author_id FROM posts WHERE created_at>=w.start_at AND created_at<w.actual_end_at AND school_id=ANY(w.school_ids)
   UNION SELECT author_id FROM comments WHERE created_at>=w.start_at AND created_at<w.actual_end_at AND school_id=ANY(w.school_ids)
 ) u),
 'registrations',r.n,'cumulative_registrations',r.total,'new_participants',r.participants,
 'active_users',a.n,'registered_participants',a.participants))
FROM windows w
CROSS JOIN LATERAL (
 SELECT count(*) n,count(DISTINCT author_id) authors FROM posts
 WHERE created_at>=w.start_at AND created_at<w.actual_end_at AND school_id=ANY(w.school_ids)
) p
CROSS JOIN LATERAL (
 SELECT count(*) n,count(DISTINCT author_id) authors FROM comments
 WHERE created_at>=w.start_at AND created_at<w.actual_end_at AND school_id=ANY(w.school_ids)
) c
CROSS JOIN LATERAL (
 SELECT count(*) FILTER(WHERE registered_at>=w.start_at) n,count(*) total,
 count(*) FILTER(WHERE registered_at>=w.start_at AND EXISTS (
  SELECT 1 FROM posts WHERE author_id=r.account_id AND created_at>=r.registered_at AND created_at>=w.start_at AND created_at<w.actual_end_at AND school_id=ANY(w.school_ids)
  UNION ALL SELECT 1 FROM comments WHERE author_id=r.account_id AND created_at>=r.registered_at AND created_at>=w.start_at AND created_at<w.actual_end_at AND school_id=ANY(w.school_ids)
 )) participants
 FROM statistics_registrations r WHERE registered_at<w.actual_end_at AND time_quality='verified'
 AND (w.full_site OR registration_school_id=ANY(w.school_ids))
) r
CROSS JOIN LATERAL (
 SELECT count(DISTINCT account_id) n,count(DISTINCT account_id) FILTER(WHERE kind IN ('post_create','comment_create')) participants
 FROM statistics_activity_events WHERE occurred_at>=w.start_at AND occurred_at<w.actual_end_at
 AND (w.full_site OR school_id=ANY(w.school_ids))
) a
