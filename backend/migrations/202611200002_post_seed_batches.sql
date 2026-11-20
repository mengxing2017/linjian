ALTER TABLE accounts ADD COLUMN account_kind text NOT NULL DEFAULT 'ordinary' CHECK(account_kind IN ('ordinary','simulated'));
ALTER TABLE accounts ADD CONSTRAINT simulated_credentials CHECK(account_kind<>'simulated' OR (bound_school_id IS NOT NULL AND password_hash IS NULL AND NOT platform_admin AND wx_openid IS NULL));
CREATE FUNCTION linjian_guard_account_kind() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.account_kind IS DISTINCT FROM OLD.account_kind OR (OLD.account_kind='simulated' AND NEW.bound_school_id IS DISTINCT FROM OLD.bound_school_id) THEN
  RAISE EXCEPTION 'account kind and simulated school are immutable' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER account_kind_immutable BEFORE UPDATE ON accounts FOR EACH ROW EXECUTE FUNCTION linjian_guard_account_kind();
CREATE FUNCTION linjian_ordinary_actor() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE subject uuid;
BEGIN
 subject := (to_jsonb(NEW)->>TG_ARGV[0])::uuid;
 IF EXISTS(SELECT 1 FROM accounts WHERE id=subject AND account_kind='simulated') THEN
  RAISE EXCEPTION 'simulated account cannot authenticate or act' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER ordinary_sessions BEFORE INSERT OR UPDATE ON sessions FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');
CREATE TRIGGER ordinary_identity BEFORE INSERT OR UPDATE ON miniapp_identities FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');
CREATE TRIGGER ordinary_membership BEFORE INSERT OR UPDATE ON school_memberships FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');
CREATE TRIGGER ordinary_authorization BEFORE INSERT OR UPDATE ON school_member_authorizations FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');
CREATE TRIGGER ordinary_override BEFORE INSERT OR UPDATE ON school_member_permission_overrides FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');
CREATE TRIGGER ordinary_registration BEFORE INSERT OR UPDATE ON statistics_registrations FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');
CREATE TRIGGER ordinary_activity BEFORE INSERT OR UPDATE ON statistics_activity_events FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');
CREATE TRIGGER ordinary_comment BEFORE INSERT OR UPDATE ON comments FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('author_id');
CREATE TRIGGER ordinary_reaction BEFORE INSERT OR UPDATE ON post_reactions FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');
CREATE TRIGGER ordinary_comment_reaction BEFORE INSERT OR UPDATE ON comment_reactions FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');
CREATE TRIGGER ordinary_follow BEFORE INSERT OR UPDATE ON follows FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('follower_id');
CREATE TRIGGER ordinary_case BEFORE INSERT OR UPDATE ON cases FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');
CREATE OR REPLACE FUNCTION statistics_capture_registration() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 INSERT INTO statistics_registrations(account_id,registered_at,registration_school_id,school_assignment,time_quality)
 SELECT NEW.account_id,NEW.created_at,a.bound_school_id,CASE WHEN a.bound_school_id IS NULL THEN 'unassigned' ELSE 'known_school' END,'verified'
 FROM accounts a WHERE a.id=NEW.account_id AND a.account_kind='ordinary';
 RETURN NEW;
END $$;

CREATE TABLE post_seed_batches (
 id uuid PRIMARY KEY, school_id uuid NOT NULL REFERENCES schools, created_by uuid NOT NULL REFERENCES accounts,
 name text NOT NULL CHECK(length(btrim(name)) BETWEEN 1 AND 100),
 state text NOT NULL DEFAULT 'preview' CHECK(state IN ('preview','committed','revoked')), revision bigint NOT NULL DEFAULT 1,
 preview_digest text NOT NULL, policy_digest text NOT NULL, preview_payload jsonb NOT NULL,
 expires_at timestamptz NOT NULL DEFAULT now()+interval '30 minutes',
 commit_idempotency_key uuid, commit_request_digest text,
 created_at timestamptz NOT NULL DEFAULT now(), committed_at timestamptz,
 revoked_at timestamptz, revoked_by uuid REFERENCES accounts, revoke_reason text,
 UNIQUE(school_id,id), UNIQUE(school_id,created_by,commit_idempotency_key),
 CHECK((state='preview')=(committed_at IS NULL)),
 CHECK((state='revoked')=(revoked_at IS NOT NULL)),
 CHECK((state='preview')=(commit_idempotency_key IS NULL))
);
CREATE INDEX post_seed_batch_list ON post_seed_batches(school_id,created_at DESC,id);
CREATE TRIGGER ordinary_batch_actor BEFORE INSERT OR UPDATE ON post_seed_batches FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('created_by');
ALTER TABLE posts DROP CONSTRAINT posts_created_via_check;
ALTER TABLE posts ADD CONSTRAINT posts_created_via_check CHECK(created_via IN ('legacy','web','miniapp','seed'));
ALTER TABLE posts ADD COLUMN seed_batch_id uuid, ADD COLUMN display_published_at timestamptz NOT NULL DEFAULT now();
ALTER TABLE posts ADD FOREIGN KEY(school_id,seed_batch_id) REFERENCES post_seed_batches(school_id,id);
ALTER TABLE posts ADD CHECK((created_via='seed')=(seed_batch_id IS NOT NULL));
CREATE INDEX post_display_feed ON posts(school_id,is_top DESC,sort_order DESC,display_published_at DESC,id);
CREATE INDEX post_seed_batch_posts ON posts(seed_batch_id,id) WHERE seed_batch_id IS NOT NULL;
ALTER TABLE posts ALTER COLUMN sort_order SET DEFAULT 0;
CREATE OR REPLACE FUNCTION forum_post_order_metadata() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' THEN
  NEW.is_top:=(NEW.is_top OR NEW.top_sort>0) AND NEW.visibility<>'deleted';
 ELSIF NEW.top_sort IS DISTINCT FROM OLD.top_sort AND NEW.is_top IS NOT DISTINCT FROM OLD.is_top THEN
  NEW.is_top:=NEW.top_sort>0;
 END IF;
 IF NEW.visibility='deleted' THEN NEW.is_top:=false; END IF;
 NEW.top_sort:=CASE WHEN NEW.is_top THEN greatest(NEW.top_sort,1) ELSE 0 END;
 IF TG_OP='UPDATE' AND NEW.is_top IS DISTINCT FROM OLD.is_top THEN NEW.sort_order:=0; END IF;
 RETURN NEW;
END $$;
DROP SEQUENCE post_sort_order_seq;
CREATE TABLE post_seed_items (
 batch_id uuid NOT NULL, school_id uuid NOT NULL, row_number integer NOT NULL CHECK(row_number BETWEEN 1 AND 50),
 post_id uuid NOT NULL UNIQUE, author_id uuid NOT NULL REFERENCES accounts,
 PRIMARY KEY(batch_id,row_number),
 FOREIGN KEY(school_id,batch_id) REFERENCES post_seed_batches(school_id,id),
 FOREIGN KEY(school_id,post_id) REFERENCES posts(school_id,id)
);
CREATE TRIGGER seed_item_immutable BEFORE UPDATE OR DELETE ON post_seed_items FOR EACH ROW EXECUTE FUNCTION forum_events_immutable();
CREATE FUNCTION linjian_guard_seed_post() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE kind text; scope uuid; batch_state text;
BEGIN
 IF TG_OP='UPDATE' AND (NEW.seed_batch_id,NEW.author_id,NEW.display_published_at,NEW.created_at) IS DISTINCT FROM (OLD.seed_batch_id,OLD.author_id,OLD.display_published_at,OLD.created_at) THEN
  RAISE EXCEPTION 'post identity and time are immutable' USING ERRCODE='23514';
 END IF;
 SELECT account_kind,bound_school_id INTO kind,scope FROM accounts WHERE id=NEW.author_id;
 IF NEW.created_via='seed' THEN
  SELECT state INTO batch_state FROM post_seed_batches WHERE id=NEW.seed_batch_id AND school_id=NEW.school_id FOR SHARE;
  IF kind<>'simulated' OR scope IS DISTINCT FROM NEW.school_id OR batch_state IS NULL
   OR (TG_OP='INSERT' AND batch_state<>'preview') OR NEW.display_published_at>now() THEN
   RAISE EXCEPTION 'invalid seed provenance' USING ERRCODE='23514';
  END IF;
  IF batch_state='revoked' AND (NEW.visibility='visible' OR NEW.audit_status='approved' AND (TG_OP='INSERT' OR OLD.audit_status<>'approved')) THEN
   RAISE EXCEPTION 'revoked seed content cannot be republished' USING ERRCODE='23514';
  END IF;
 ELSIF kind='simulated' THEN RAISE EXCEPTION 'simulated author requires seed provenance' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER seed_post_guard BEFORE INSERT OR UPDATE ON posts FOR EACH ROW EXECUTE FUNCTION linjian_guard_seed_post();
CREATE FUNCTION linjian_seed_item_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM posts p JOIN post_seed_items i ON i.post_id=p.id AND i.batch_id=p.seed_batch_id AND i.school_id=p.school_id AND i.author_id=p.author_id WHERE p.id=NEW.post_id) THEN
  RAISE EXCEPTION 'seed item mismatch' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE CONSTRAINT TRIGGER seed_item_consistent AFTER INSERT ON post_seed_items DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION linjian_seed_item_consistent();
CREATE FUNCTION linjian_seed_post_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.created_via='seed' AND NOT EXISTS(SELECT 1 FROM post_seed_items i WHERE i.post_id=NEW.id AND i.batch_id=NEW.seed_batch_id AND i.school_id=NEW.school_id AND i.author_id=NEW.author_id) THEN
  RAISE EXCEPTION 'seed post requires batch item' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE CONSTRAINT TRIGGER seed_post_consistent AFTER INSERT ON posts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION linjian_seed_post_consistent();
CREATE FUNCTION linjian_seed_batch_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF (NEW.school_id,NEW.created_by,NEW.preview_digest,NEW.policy_digest,NEW.preview_payload,NEW.created_at,NEW.name,NEW.expires_at) IS DISTINCT FROM (OLD.school_id,OLD.created_by,OLD.preview_digest,OLD.policy_digest,OLD.preview_payload,OLD.created_at,OLD.name,OLD.expires_at)
 OR NEW.revision<>OLD.revision+1
 OR (OLD.state='committed' AND (NEW.committed_at,NEW.commit_idempotency_key,NEW.commit_request_digest) IS DISTINCT FROM (OLD.committed_at,OLD.commit_idempotency_key,OLD.commit_request_digest))
 OR NOT ((OLD.state='preview' AND NEW.state='committed') OR (OLD.state='committed' AND NEW.state='revoked')) THEN
  RAISE EXCEPTION 'invalid batch transition' USING ERRCODE='23514';
 END IF;
 IF NEW.state='revoked' AND EXISTS(SELECT 1 FROM posts WHERE seed_batch_id=NEW.id AND visibility NOT IN ('offline','deleted')) THEN
  RAISE EXCEPTION 'batch posts must be offline before revoke' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER seed_batch_guard BEFORE UPDATE ON post_seed_batches FOR EACH ROW EXECUTE FUNCTION linjian_seed_batch_guard();

CREATE FUNCTION linjian_public_post(item posts) RETURNS jsonb LANGUAGE sql STABLE AS $$
 SELECT jsonb_build_object('id',item.id,'school_id',item.school_id,'author_id',item.author_id,'title',item.title,'content',item.content,'images',item.images,'tags',item.tags,'audit_status',item.audit_status,'visibility',item.visibility,'browse',item.browse,'revision',item.revision,'created_at',item.created_at,'updated_at',item.updated_at,'display_published_at',item.display_published_at)
$$;

CREATE OR REPLACE FUNCTION forum_permission_catalog() RETURNS jsonb LANGUAGE sql IMMUTABLE AS $catalog$
 SELECT '[{"id":"P01","key":"content.post.read","name":"查看帖子","description":"管理列表、筛选、详情、待审/隐藏内容、审核记录及授权附件；推荐记录只读，不含重试/终止 Outbox","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P02","key":"content.post.review","name":"审核帖子","description":"通过、驳回帖子；不含修改正文或上下架","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.post.read"],"high_risk":false},{"id":"P03","key":"content.post.edit","name":"编辑帖子","description":"修改已有本校帖子的正文、标题、图片与标签；不含平台代发/导入","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.post.read"],"high_risk":false},{"id":"P04","key":"content.post.visibility","name":"帖子上架/下架","description":"改变帖子展示状态；不替代审核，不包含删除","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.post.read"],"high_risk":false},{"id":"P05","key":"content.post.pin","name":"帖子置顶与排序","description":"置顶、取消置顶及帖子管理排序","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.post.read"],"high_risk":false},{"id":"P06","key":"content.post.delete","name":"删除帖子","description":"删除已有帖子；不授予恢复或清库能力","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.post.read"],"high_risk":true},{"id":"P07","key":"post.recommend","name":"推荐/重推帖子","description":"推荐预览、确认提交；仍校验发布证据、频控、幂等和来源就绪条件","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["content.post.read"],"high_risk":true},{"id":"P08","key":"content.comment.read","name":"查看评论","description":"评论管理列表、筛选、正文、审核记录及所属帖子最小上下文","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P09","key":"content.comment.review","name":"审核评论","description":"通过、驳回评论","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.comment.read"],"high_risk":false},{"id":"P10","key":"content.comment.visibility","name":"评论显示/隐藏","description":"隐藏、恢复展示；不等于审核或删除","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.comment.read"],"high_risk":false},{"id":"P11","key":"content.comment.delete","name":"删除评论","description":"管理员删除本校评论；运营获授权也可执行；不影响社区删除本人评论规则","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.comment.read"],"high_risk":true},{"id":"P12","key":"content.case.read","name":"查看举报与反馈工单","description":"case 列表、详情、联系信息及证据；不附赠目标帖子完整管理读取","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P13","key":"content.case.resolve","name":"处理举报与反馈工单","description":"resolved/dismissed 与处理说明；不包含删除工单、直接删帖或封禁","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.case.read"],"high_risk":false},{"id":"P14","key":"operation.qrcode.read","name":"查看群二维码","description":"本校二维码列表、详情和管理图片","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P15","key":"operation.qrcode.create","name":"新增群二维码","description":"新建本校二维码，含该业务必需上传；不含平台默认二维码","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.qrcode.read"],"high_risk":false},{"id":"P16","key":"operation.qrcode.update","name":"编辑群二维码","description":"修改、启停、展示时机与排序字段","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.qrcode.read"],"high_risk":false},{"id":"P17","key":"operation.qrcode.delete","name":"删除群二维码","description":"删除本校二维码","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.qrcode.read"],"high_risk":true},{"id":"P18","key":"operation.banner.read","name":"查看首页广告位","description":"本校广告列表、详情、图片和已有曝光/点击汇总","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P19","key":"operation.banner.create","name":"新增首页广告位","description":"新建本校广告和必需上传；只可选择本校已发布可见帖作为落地目标","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.banner.read"],"high_risk":false},{"id":"P20","key":"operation.banner.update","name":"编辑首页广告位","description":"修改、启停、投放时间和排序字段","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.banner.read"],"high_risk":false},{"id":"P21","key":"operation.banner.delete","name":"删除首页广告位","description":"删除本校广告","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.banner.read"],"high_risk":true},{"id":"P22","key":"operation.feedback.read","name":"查看投诉反馈","description":"运营投诉列表、详情及授权图片；与 case 工单区分","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P23","key":"operation.feedback.delete","name":"删除投诉反馈","description":"删除运营投诉记录；不提供 case 删除或用户封禁","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.feedback.read"],"high_risk":true},{"id":"P24","key":"resource.school.read","name":"查看学校展示资源","description":"服务/法律等现有学校资源与管理素材；不授予平台资源编辑","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":[],"high_risk":false},{"id":"P25","key":"resource.school.write","name":"配置并发布学校展示资源","description":"现有预览/确认流程中的新增、修改、启停、排序与发布；含所需素材上传，不虚构独立删除/草稿流程","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["resource.school.read"],"high_risk":true},{"id":"P26","key":"school.home_menu.read","name":"查看首页菜单/话题","description":"标签菜单、筛选项及现有配置读取","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P27","key":"school.home_menu.write","name":"新增/编辑首页菜单","description":"标签名称、展示状态、启用状态；不包含删除、批量排序或导入","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.home_menu.read"],"high_risk":false},{"id":"P28","key":"school.home_menu.delete","name":"删除首页菜单项","description":"删除本校菜单标签","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.home_menu.read"],"high_risk":true},{"id":"P29","key":"school.home_menu.reorder","name":"调整首页菜单顺序","description":"现有菜单排序提交","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.home_menu.read"],"high_risk":false},{"id":"P30","key":"school.home_menu.settings.update","name":"修改首页菜单设置","description":"菜单模块现有独立设置，不等于学校审核策略","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.home_menu.read"],"high_risk":false},{"id":"P31","key":"school.home_menu.import","name":"导入首页菜单","description":"本项目现有导入预览及确认；不连接 wxrobot 数据库或服务","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.home_menu.read","school.home_menu.write"],"high_risk":false},{"id":"P32","key":"school.blocked_word.read","name":"查看屏蔽词","description":"本校词库及对本校生效的平台规则只读","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P33","key":"school.blocked_word.write","name":"新增/编辑屏蔽词","description":"本校词条、备注、启停；不改变平台词库","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.blocked_word.read"],"high_risk":false},{"id":"P34","key":"school.blocked_word.delete","name":"删除屏蔽词","description":"删除本校词条","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.blocked_word.read"],"high_risk":true},{"id":"P35","key":"school.blocked_word.reorder","name":"调整屏蔽词顺序","description":"本校现有排序流程","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.blocked_word.read"],"high_risk":false},{"id":"P36","key":"school.blocked_word.import","name":"导入屏蔽词","description":"词库导入预览与确认","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.blocked_word.read","school.blocked_word.write"],"high_risk":false},{"id":"P37","key":"school.profile.read","name":"查看学校后台资料","description":"本校后台资料；公开目录另走公开/社区规则","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P38","key":"school.profile.update","name":"修改学校展示资料","description":"学校名称、地区、图标、主题等学校可改字段；不含创建、启停、删除学校","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.profile.read"],"high_risk":false},{"id":"P39","key":"school.review_settings.update","name":"修改帖子/评论审核策略","description":"手动/自动审核设置，字段白名单严格分离","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.profile.read"],"high_risk":true},{"id":"P40","key":"school.user.read","name":"查看本校用户","description":"用户列表、详情、已有状态历史和授权头像；不包含平台范围搜索","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P41","key":"school.user.status.update","name":"封禁/解封本校用户","description":"本校用户状态修改；不含手机号纠正、学校纠正、后台账号启停","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.user.read"],"high_risk":true},{"id":"P42","key":"school.member.read","name":"查看本校后台成员与权限","description":"本校后台成员及业务权限，不含全局候选账号、密码和跨校授权","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":[],"high_risk":false},{"id":"P43","key":"school.member.recommendation.assign","name":"为运营配置推荐权限","description":"本校管理员的受限推荐授权，不能改其他权限或给本人授权","group":"学校与管理","applicable_roles":["admin"],"default_by_role":{"admin":true,"operator":false},"requires":["school.member.read","post.recommend"],"high_risk":true},{"id":"P44","key":"statistics.school.read","name":"查看本校业务统计","description":"学校统计与当前可读快照；不包含全校汇总、跨校比较","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P45","key":"statistics.school.export","name":"导出本校业务统计","description":"已有学校统计快照 CSV 导出；下载时重查","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["statistics.school.read"],"high_risk":false},{"id":"P46","key":"content.simulation.manage","name":"管理仿真发帖","description":"学校管理员及运营发布和管理仿真帖子、模拟作者与批次；不含审核或来源推送","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.post.read"],"high_risk":true}]'::jsonb
$catalog$;
CREATE OR REPLACE FUNCTION forum_create_source_intent(item posts, trigger_kind text) RETURNS uuid LANGUAGE plpgsql AS $$
DECLARE seq bigint; b school_source_bindings; bv uuid; event uuid:=gen_random_uuid();
BEGIN
 IF item.created_via='seed' OR EXISTS(SELECT 1 FROM posts WHERE id=item.id AND created_via='seed') THEN RAISE EXCEPTION 'seed_content_not_recommendable' USING ERRCODE='23514'; END IF;
 IF trigger_kind NOT IN ('review_publication','manual_recommendation') THEN RAISE EXCEPTION 'invalid trigger'; END IF;
 PERFORM id FROM schools WHERE id=item.school_id FOR UPDATE;
 IF item.audit_status<>'approved' OR item.visibility<>'visible' OR item.approved_snapshot_id IS NULL
 OR NOT EXISTS(SELECT 1 FROM schools WHERE id=item.school_id AND active) THEN RAISE EXCEPTION 'publication not eligible'; END IF;
 IF NOT forum_post_has_publication_evidence(item) THEN RAISE EXCEPTION 'publication evidence mismatch'; END IF;
 INSERT INTO forum_source_versions(school_id,post_id,sequence,available) VALUES(item.school_id,item.id,1,true)
 ON CONFLICT(school_id,post_id) DO UPDATE SET sequence=forum_source_versions.sequence+1,available=true,delivery_intent=true RETURNING sequence INTO seq;
 SELECT sb.* INTO b FROM school_source_bindings sb JOIN source_active_configuration ac ON ac.connection_id=sb.connection_id WHERE sb.school_id=item.school_id FOR SHARE OF sb,ac;
 IF b.id IS NOT NULL THEN SELECT id INTO bv FROM school_source_binding_versions WHERE binding_id=b.id AND revision=b.revision; END IF;
 INSERT INTO forum_outbox(id,school_id,post_id,sequence,payload,schema_version,signature_version,delivery_state,publication_snapshot_id,projection_version,trigger_type)
 VALUES(event,item.school_id,item.id,seq,jsonb_build_object('event_id',event,'schema_version',2,'event_type','content.upsert','occurred_at',now()),2,2,'waiting_binding',item.approved_snapshot_id,'message_v1',trigger_kind);
 INSERT INTO forum_publication_preparations(event_id,snapshot_id) VALUES(event,item.approved_snapshot_id);
 IF b.id IS NOT NULL THEN PERFORM forum_bind_prepared_source(event,b.connection_version,bv,b.route_id); END IF;
 IF b.id IS NOT NULL AND NOT b.active THEN UPDATE forum_outbox SET delivery_state='paused' WHERE id=event; END IF;
 RETURN event;
END $$;
CREATE OR REPLACE FUNCTION forum_enqueue_post() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND (NEW.title,NEW.content,NEW.images,NEW.tags,NEW.audit_status,NEW.visibility,NEW.approved_snapshot_id) IS NOT DISTINCT FROM
 (OLD.title,OLD.content,OLD.images,OLD.tags,OLD.audit_status,OLD.visibility,OLD.approved_snapshot_id) THEN RETURN NEW; END IF;
 PERFORM id FROM schools WHERE id=NEW.school_id FOR UPDATE;
 UPDATE forum_outbox SET delivery_state='cancelled' WHERE school_id=NEW.school_id AND post_id=NEW.id
 AND attempts=0 AND lease_token IS NULL AND delivery_state NOT IN ('accepted','cancelled');
 IF NEW.created_via='seed' THEN RETURN NEW; END IF;
 IF NEW.audit_status='approved' AND NEW.visibility='visible' AND EXISTS(SELECT 1 FROM schools WHERE id=NEW.school_id AND active) THEN
  PERFORM forum_create_source_intent(NEW,'review_publication');
 ELSE
  UPDATE forum_source_versions SET available=false WHERE school_id=NEW.school_id AND post_id=NEW.id;
 END IF;
 RETURN NEW;
END $$;
