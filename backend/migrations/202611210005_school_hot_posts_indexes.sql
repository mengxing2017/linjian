CREATE INDEX post_hot_public_window ON posts(school_id, display_published_at DESC, id) INCLUDE(title)
WHERE audit_status='approved' AND visibility='visible';

CREATE INDEX comment_hot_valid ON comments(school_id, post_id)
WHERE audit_status='approved' AND visibility='visible' AND NOT deleted;

CREATE INDEX reaction_hot_likes ON post_reactions(school_id, post_id)
WHERE kind='like';
