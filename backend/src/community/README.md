# 社区业务

状态：已实现并在 bootstrap/http 装配。运行和验收见 根 README.md 与 docs/runbooks/migration.md。

职责：帖子、评论、互动、关注与站内通知。

## 帖子搜索

`GET /api/v1/schools/{school}/posts?search=关键词` 复用帖子列表接口，只搜索当前授权学校内的标题或完整正文。关键词按 ECMAScript 边界空白规则规范化，非空时最多 100 个 Unicode 标量值；空值仍表示不筛选，兼容首页及个人列表。客户端和 Rust 共用规范化 fixture。

查询使用参数绑定和显式 `ESCAPE '!'`，`!`、`%`、`_`、反斜杠均按字面处理，英文匹配大小写不敏感。不拆词、不按相关度排序，沿用置顶、人工顺序、展示发布时间和 ID。服务层保留成员授权，adapter 校验学校有效性及现有作者／读取权限可见范围；列表查询不增加浏览计数或生成 Outbox。

小程序首页学校标题旁进入 `/pages/index/search` 的独立搜索页，空输入不请求列表；`view=mine`、`view=collect` 仍进入个人列表。搜索结果只保存在页面内，分页及重试使用已提交关键词，切换学校或账号时使旧结果失效。`backend/tests/post_search/mod.rs` 覆盖字面匹配、权限、JSON:API 分页、排序与只读边界。

## 通知未读汇总

`GET /api/v1/schools/{school}/notification-summary` 返回当前认证账号在指定学校可访问通知的未读总数，JSON:API 类型为 `notification-summaries`，资源 ID 与 `school_id` 均为学校 UUID，`unread_count` 为非负整数。该响应是个人汇总而非学校公共数据，使用 `private, no-store`。

服务层执行成员授权，adapter 校验学校有效性；账号来自 Actor。列表与 COUNT 共用 `south/adapter/postgres/notification_visibility.sql` 中的可见性条件，计数额外要求 `read_at IS NULL`。沿用现有单条已读幂等接口，不生成 Outbox、发送任务或额外通知；本次不新增统计表或数据库 migration。

`backend/tests/notification_summary/mod.rs` 在显式隔离测试库内验证跨学校、跨账号、分页总数、重复已读、审核/删除可见性、JSON:API fixture、HTTP 认证、停用学校及无来源推送副作用。

约束：审核状态与展示状态分离；浏览、点赞和计数更新不得触发发送。

模块边界：

- domain/：领域对象、约束与状态转换，不依赖 Axum/SQLx。
- north/local/：用例编排与事务边界。
- north/remote/：路由、DTO 与身份提取，不写 SQL。
- south/port/：仓储与外部能力接口。
- south/adapter/postgres/：SQL 与行转换。

业务与权限回归位于 backend/tests/forum_database.rs 与 forum_boundary.rs，共享 client 位于 api-contract/src/modules/business。

## 本校热榜

`GET /api/v1/schools/{school}/hot-posts?limit=10` 是独立只读 JSON:API 集合，默认 10 条，允许 1–20 条，不分页、不接受筛选或自定义排序。每次检查学校授权和可用性，仅查询审核通过、可见且展示发布时间在最近连续 7 天内的本校帖子；作者和管理员不能绕过公开资格。

排名使用当前有效点赞 `L` 和有效评论条数 `C`，`E=L+2*C`，最低 `E>=3`，分数为 `E/(age_hours+2)^0.6`。时间统一使用 `display_published_at`，不是入库时间；这是近期发布帖的累计互动榜，不是近期发生互动榜。分数相同时按展示时间降序、帖子 ID 升序；置顶、人工顺序、浏览和收藏不加权。

查询在短时只读可重复读事务中完成，语句超时为 1500ms。`ranked_at`、`validated_at`、`window_end` 均为同一数据库事务时点，计数和资格来自同一读取快照。响应使用 `private, no-store`，不保存排名缓存或快照，不增加浏览、通知或 Outbox。并发下架以读取快照为界，下架提交后新读取不再返回；已打开首页通过重新显示、下拉及前台 60 秒刷新复核。

`backend/tests/hot_posts/mod.rs` 覆盖固定时间窗口、聚合、排序、权限、协议及无副作用。专用容量测试使用合成的 3 万帖子（热点学校 1 万候选）、10 万点赞及 11 万评论，输出查询计划与 20 并发下的 HTTP 延迟；仅在明确配置的隔离测试库中执行：

```sh
devbox run -- cargo test --locked --manifest-path backend/Cargo.toml --test hot_posts_performance -- --ignored --nocapture
```

该命令只接受 `LINJIAN_TEST_DATABASE_URL`；在合成数据写入后执行 `VACUUM ANALYZE`，测量稳定态而非受控冷缓存。拟定 p95 预算为 500ms，须结合测试硬件解释；失败时保留带 `forum_hot_capacity_` 前缀的隔离 schema 便于诊断。容量测试和发布验收不进入业务启动或写入门禁。

热榜通过候选 ID 数组分别聚合点赞、评论，完整计算后才取 Top N。新增公开帖子时间窗口覆盖索引、有效评论部分索引及点赞部分索引；点赞索引只含学校/帖子 ID，避免计数时读取包含账号和 kind 的完整主键。索引仅优化读取，不改变互动写入和审核资格。
