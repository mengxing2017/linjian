# 小程序审核内容来源推送

新建小程序帖子固定人工审核。管理后台展示标题、完整内容、学校、用户昵称和创建时间；审核详情独立读取当前全文及附件，通过时提交 `review_digest` 和帖子修订。版本冲突后必须重新加载全文核对，不自动批准新版本。审核页面不展示或依赖来源摘要。通过后，后端针对已锁定全文生成 `plain-v1` 来源摘要（最多 120 个 Unicode 标量，含截断省略号），并将审核快照、帖子状态与 Outbox 准备意图在同一数据库事务提交。审核页面操作说明见 [全文审核](content-review.md)。

准备 worker 在事务外调用微信，将正式版 `pages/index/detail?school=<uuid>&id=<uuid>` 生成链接；默认 Short Link，非永久、按官方策略 30 天。可显式配置 URL Link（1–30 天），没有自动降级。微信权限、配额或网络故障保留准备状态，不阻断论坛发帖或撤回。

链接就绪后冻结 v2 请求：`title` 为审核标题，`content` 为“摘要 + 换行 + 查看详情：+ 微信原始链接”，`detail_url` 保留论坛 HTTPS 网页地址。之后投递重试保持相同事件 ID、sequence 和请求字节。准备期间编辑、撤审、下架、停校会使旧机会失效；浏览、点赞等不创建推送。

## 配置和升级

`env.example` 和 Compose 包含全部选项。`LINJIAN_WX_APP_ID`、`LINJIAN_WX_SECRET` 是本项目后端小程序凭据，与登录交换共用凭据但不共用 session/token。stable_token 独立按凭据摘要缓存，带刷新租约和 generation 保护；缓存含敏感令牌，数据库及备份须保持现有访问权限。

- `LINJIAN_MINIAPP_SOURCE_ENABLED=true`：false 暂停链接准备，意图仍保留，来源投递及撤回继续运行。
- `LINJIAN_MINIAPP_LINK_TYPE=short_link`：可显式为 url_link。
- `LINJIAN_MINIAPP_SHORT_LINK_PERMANENT=false`：永久模式须由部署者显式启用并确认微信额度。
- `LINJIAN_MINIAPP_URL_LINK_DAYS=30`：1–30。
- `LINJIAN_MINIAPP_LINK_MIN_REMAINING_SECONDS=86400`：必须正数且小于所选有效期。
- `LINJIAN_MINIAPP_PREPARATION_CONCURRENCY=2`：每实例 1–16，跨实例通过数据库租约互斥。

新增 forward-only migration `202611090002_miniapp_publication.sql`。存量帖子保留 legacy 全文投影，不批量生成新机会。需协调发布配套后端、Web、contract、小程序，避免旧进程将新帖子写成 legacy。禁止回退到不认识新投影和状态的旧二进制。

未配置微信时论坛可启动，准备进入 `miniapp_not_configured`。配置补齐并重启后恢复该状态；已知 AppID 不允许偷偷切换为另一个 AppID。审核时的策略固定在快照；审核时 AppID 空缺则首次有配置时固定。其他永久失败修复后在来源发件记录选择“重试内容准备”，必须填写原因并携带准备修订；冻结后使用“重试同一事件”。两种恢复均保留事件身份。

首次投递前链接若已不足最小剩余时间，事件阻塞为 `link_expired_before_delivery`，不能修改冻结链接或直接重试；重新撤审、人工审核形成新机会。已经尝试投递的事件按原字节继续确认回执，不能重拼可能已被接收的请求。

准备记录保存受控错误码和尝试次数；来源发件详情显示摘要、链接和有效期。凭据、token、原始微信错误、冻结签名均不向浏览器回显。Outbox 仍是来源推送，不创建发送任务或发送授权。

小程序登录和正常学校选择完成后恢复受限详情目标；仅接受规范 UUID，30 分钟过期，不自动绑定链接中的学校。待审或隐藏帖子不分享标题。真机联调须确认小程序主体拥有所选接口权限、详情页已发布，且审核链路和 WeKeeper v2 来源接入配置可用。

## 本地验证

执行 `make check-web`、`make check-miniapp`、`devbox run -- make check-backend`。显式隔离数据库执行 `make test-integration`；也可使用 `LINJIAN_POSTGRES_BIN=/path/to/postgresql17/bin devbox run -- python3 tools/test_database.py` 自动建临时数据库。测试不调用真实微信或转发平台。
