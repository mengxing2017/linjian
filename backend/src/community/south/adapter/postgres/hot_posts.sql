WITH candidates AS MATERIALIZED (
    SELECT id, school_id, title, display_published_at
    FROM posts
    WHERE school_id=$1 AND audit_status='approved' AND visibility='visible'
      AND display_published_at BETWEEN $2 AND $3
    ORDER BY id
), likes AS (
    SELECT reaction.post_id, count(*) AS like_count
    FROM post_reactions reaction
    WHERE reaction.school_id=$1 AND reaction.kind='like'
      AND reaction.post_id=ANY(ARRAY(SELECT id FROM candidates))
    GROUP BY reaction.post_id
), replies AS (
    SELECT comment.post_id, count(*) AS comment_count
    FROM comments comment
    WHERE comment.school_id=$1 AND comment.audit_status='approved'
      AND comment.visibility='visible' AND NOT comment.deleted
      AND comment.post_id=ANY(ARRAY(SELECT id FROM candidates))
    GROUP BY comment.post_id
), counted AS (
    SELECT candidate.*, coalesce(likes.like_count,0) AS like_count,
        coalesce(replies.comment_count,0) AS comment_count
    FROM candidates candidate
    LEFT JOIN likes ON likes.post_id=candidate.id
    LEFT JOIN replies ON replies.post_id=candidate.id
), scored AS (
    SELECT *, (like_count + $4 * comment_count)::double precision /
        power(EXTRACT(EPOCH FROM ($3 - display_published_at))::double precision / 3600.0 + $5, $6) AS score
    FROM counted
    WHERE like_count + $4 * comment_count >= $7
), selected AS MATERIALIZED (
    SELECT * FROM scored
    ORDER BY score DESC, display_published_at DESC, id ASC LIMIT $8
)
SELECT id, school_id,
    row_number() OVER (ORDER BY score DESC, display_published_at DESC, id ASC) AS rank,
    title, like_count, comment_count
FROM selected ORDER BY rank
