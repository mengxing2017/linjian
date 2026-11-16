# 小程序审核内容来源推送

小程序帖子遵循本校 `post_audit_mode`：人工模式待审，自动模式通过基础校验后自动通过。自动通过写入无人工审核人的系统决定及策略版本，与人工通过共用不可变发布快照。管理后台展示标题、完整内容、学校、用户昵称和创建时间；审核详情独立读取当前全文及附件，通过时提交 `review_digest` 和帖子修订。版本冲突后必须重新加载全文核对，不自动批准新版本。审核页面不展示或依赖来源摘要。通过后，后端针对已锁定全文生成 `plain-v1` 来源摘要（最多 20 个 Unicode 标量，超过时取前 18 字加“……”，空格与标点计入），并将审核快照、帖子状态与 Outbox 准备意图在同一数据库事务提交。审核页面操作说明见 [全文审核](content-review.md)。

准备 worker 在事务外调用微信，将正式版 `pages/index/detail?school=<uuid>&id=<uuid>` 生成链接；默认 Short Link，非永久、按官方策略 30 天。可显式配置 URL Link（1–30 天），没有自动降级。微信权限、配额或网络故障保留准备状态，不阻断论坛发帖或撤回。

链接就绪后，由本项目构造“摘要 + 一个换行 + 微信原始链接”，例如：

```text
之前有人说军训是 1.10 开始 训……
mp://Ltxhoqe18kEGxlD
```

mp:// 和 #小程序:// 都作为小程序链接原样保留，不添加标题行或“查看详情：”。完整消息最多 200 个 Unicode 标量，先预留完整链接与换行，再裁剪摘要；链接本身不截断。链接过长保留准备记录并显示原因。

请求只包含 event_id、schema_version、route_id、event_type、occurred_at、content。收到合法 received 回执即完成来源投递，不继续等待素材处理或发送状态。受理不等于真实发送成功。准备期间编辑、撤审、下架、停校会取消未交付消息；已经请求但结果未知的消息保持原身份核对，不承诺撤回。浏览、点赞等不创建推送。

## 配置

`env.example` 和 Compose 包含全部选项。`LINJIAN_WX_APP_ID`、`LINJIAN_WX_SECRET` 是本项目后端小程序凭据，与登录交换共用凭据但不共用 session/token。stable_token 独立按凭据摘要缓存，带刷新租约和 generation 保护；缓存含敏感令牌，数据库及备份须保持现有访问权限。

- `LINJIAN_MINIAPP_SOURCE_ENABLED=true`：false 暂停链接准备，意图仍保留，既有来源投递继续运行；不产生上游撤回。
- `LINJIAN_MINIAPP_LINK_TYPE=short_link`：可显式为 url_link。
- `LINJIAN_MINIAPP_SHORT_LINK_PERMANENT=false`：永久模式须由部署者显式启用并确认微信额度。
- `LINJIAN_MINIAPP_URL_LINK_DAYS=30`：1–30。
- `LINJIAN_MINIAPP_LINK_MIN_REMAINING_SECONDS=86400`：必须正数且小于所选有效期。
- `LINJIAN_MINIAPP_PREPARATION_CONCURRENCY=2`：每实例 1–16，跨实例通过数据库租约互斥。

来源消息统一使用完整正文协议，由独立基线与后续 forward-only migration 初始化。Web 和小程序共用消息构造器，均保存审核快照；Web 无链接入口只生成摘要，小程序入口等待微信链接。
未配置微信时论坛可启动，准备进入 `miniapp_not_configured`。配置补齐并重启后恢复该状态；已知 AppID 不允许偷偷切换为另一个 AppID。审核时的策略固定在快照；审核时 AppID 空缺则首次有配置时固定。其他永久失败修复后在来源发件记录选择“重试内容准备”，必须填写原因并携带准备修订；冻结后使用“重试同一事件”。两种恢复均保留事件身份。

首次投递前链接若已不足最小剩余时间，事件阻塞为 `link_expired_before_delivery`，不能修改冻结链接或直接重试；重新撤审、人工通过形成新机会。已经尝试投递的事件按原字节继续确认回执，不能重拼可能已被接收的请求。

准备记录保存受控错误码和尝试次数；来源发件详情显示摘要、链接和有效期。凭据、token、原始微信错误、冻结签名均不向浏览器回显。Outbox 仍是来源推送，不创建发送任务或发送授权。

小程序登录和正常学校选择完成后恢复受限详情目标；仅接受规范 UUID，30 分钟过期，不自动绑定链接中的学校。待审或隐藏帖子不分享标题。真机联调须确认小程序主体拥有所选接口权限、详情页已发布，且审核链路和 WeKeeper v2 来源接入配置可用。

## 本地验证

执行 `make check-web`、`make check-miniapp`、`devbox run -- make check-backend`。显式隔离数据库执行 `make test-integration`；也可使用 `LINJIAN_POSTGRES_BIN=/path/to/postgresql17/bin devbox run -- python3 tools/test_database.py` 自动建临时数据库。测试不调用真实微信或转发平台。

## 审核模式

学校策略切换与投稿沿用学校行锁串行，自动决定保留当时策略版本；切回人工不会使旧批准证据失效。正文编辑先使旧证据失效，再按当前模式生成批准证据与来源意图，全部同事务完成；无内容变化保存不推送。
