use super::Repository;
use crate::{
    business::{Actor, Error, Result},
    community::domain::interaction::PostInteractionState,
};
use serde_json::Value;
use uuid::Uuid;

impl Repository {
    pub async fn interaction_state(
        &self,
        actor: &Actor,
        school: Uuid,
        post: Uuid,
    ) -> Result<PostInteractionState> {
        crate::schools::south::adapter::postgres::Repository(self.0.clone())
            .require_active(school)
            .await?;
        let value: Value = sqlx::query_scalar(
            "SELECT jsonb_build_object(
                'id',p.id,'school_id',p.school_id,'audit_status',p.audit_status,'visibility',p.visibility,
                'like_count',(SELECT count(*) FROM post_reactions WHERE school_id=p.school_id AND post_id=p.id AND kind='like'),
                'comment_count',(SELECT count(*) FROM comments WHERE school_id=p.school_id AND post_id=p.id AND audit_status='approved' AND visibility='visible' AND NOT deleted),
                'liked',EXISTS(SELECT 1 FROM post_reactions WHERE school_id=p.school_id AND post_id=p.id AND account_id=$3 AND kind='like'),
                'collected',EXISTS(SELECT 1 FROM post_reactions WHERE school_id=p.school_id AND post_id=p.id AND account_id=$3 AND kind='collect'),
                'author_followed',EXISTS(SELECT 1 FROM follows WHERE school_id=p.school_id AND follower_id=$3 AND target_id=p.author_id),
                'can_interact',false,'can_share',false,
                'active_member',($5 OR EXISTS(SELECT 1 FROM miniapp_users u JOIN accounts a ON a.id=u.account_id JOIN school_memberships m ON m.account_id=a.id JOIN sessions se ON se.account_id=a.id WHERE a.id=$3 AND a.active AND u.status='normal' AND m.school_id=p.school_id AND m.role<>'revoked' AND se.token_hash=$6 AND se.kind='miniapp' AND se.expires_at>now() AND se.last_seen_at>now()-interval '30 minutes'))
            ) FROM posts p JOIN schools s ON s.id=p.school_id
            WHERE p.school_id=$1 AND p.id=$2 AND s.active AND p.visibility<>'deleted'
            AND ($5 OR EXISTS(SELECT 1 FROM accounts a JOIN school_memberships m ON m.account_id=a.id JOIN sessions se ON se.account_id=a.id WHERE a.id=$3 AND a.active AND m.school_id=p.school_id AND m.role<>'revoked' AND se.token_hash=$6 AND se.kind='miniapp' AND se.expires_at>now() AND se.last_seen_at>now()-interval '30 minutes'))
            AND ($4 OR p.author_id=$3 OR (p.audit_status='approved' AND p.visibility='visible'))",
        ).bind(school).bind(post).bind(actor.id)
            .bind(actor.has_permission(school,"content.post.read"))
            .bind(actor.is_admin_browser()).bind(&actor.session_hash)
            .fetch_optional(&self.0).await?.ok_or(Error::NotFound)?;
        let mut state: PostInteractionState =
            serde_json::from_value(value.clone()).map_err(|_| Error::Unavailable)?;
        state.set_capabilities(value["active_member"].as_bool().ok_or(Error::Unavailable)?);
        Ok(state)
    }
}
