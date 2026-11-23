# 论坛来源推送协议

完整 `content` 可在原有消息末尾换行追加本校经平台审核的广告全文。广告只属于本地消息投影，不增加协议字段。追加后总长仍不超过 200 个 Unicode 标量，超限则整段省略广告。选择版本、采用快照和最终字节在冻结事务中一并提交；已冻结事件不因广告变化改写。详见[学校消息尾部广告](../guide/message-advertisements.md)。

当前新连接使用 v2；Linjian 只产生来源事件，不创建群发送任务或发送授权。上游故障不影响帖子事务，Outbox 持久保留事件。

## v2 机器契约

WeKeeper 发布 POST `/api/v2/source-authentications`、POST `/api/v2/source-route-checks`、POST `/api/v2/source-events` 和 GET `/api/v2/source-receipts/{receipt_id}`。JSON:API schema、错误目录和发布清单位于 `api-contract/source-v2/`，字节向量位于 `api-contract/fixtures/source-v2.json`。Rust 与 TypeScript 都读取同一发布版本的测试材料。

事件使用 `route_id`，不再提交 `external_organization_id` 或学校字段；也不能指定通道、群、设备和广告。每个下游对应一条调用方凭据，在后端部署环境配置 LINJIAN_WEKEEPER_ORIGIN、LINJIAN_WEKEEPER_KEY_ID 和 LINJIAN_WEKEEPER_SECRET。key_id 同时标识调用方与其凭据；内部 source_id 由后端从已认证的能力元数据自动取得，不在连接页面填写或展示。上游按当前 key 与路由明确关联鉴权，不授予同平台全部路由。

签名头为 X-Source-Version: 2、X-Source-Key-Id、X-Source-Timestamp、X-Source-Nonce、Idempotency-Key、X-Source-Signature。签名 secret 是至少 32 随机字节的无填充 base64url 交付文本，双方解码后使用原始字节。

规范字符串依次为版本、key_id、大写方法、完整外部路径、Unix 秒、nonce、幂等键、原文 SHA-256 小写十六进制，以 LF 分隔且无尾 LF。HMAC-SHA256 签名输出小写十六进制。GET 正文为零字节，幂等头为原 event_id；认证和范围检查使用独立请求 UUID。请求时间绝对偏差不超过 300 秒。每次尝试生成新 nonce，重试不改变原文、事件身份或路由。

平台管理员在只读连接页面点击“验证连接”会验证部署配置中的调用方凭据，保存的提示能力只在 60 秒内用于界面提示；每个真实请求仍由 WeKeeper 按当前凭据及授权校验。HTTPS origin 禁止用户信息、query、fragment，所有请求禁止自动重定向。secret 仅从后端配置读取用于签名，不在数据库保存，也不使用本实例加密主密钥；普通读取接口不回显。部署步骤见 [WeKeeper 配置](../runbooks/wekeeper-configuration.md)。

每所学校仅有一条当前消息路由；一个 Linjian 实例只对应后端配置指定的 WeKeeper，各校维护独立 route_id 与启停状态。帖子事务中保存审核快照与来源意图；本项目内容发布领域构造完整消息并冻结请求字节，学校、帖子 ID、修订与 sequence 只留在本地。

事件 attributes 仅包含 event_id、schema_version=2、route_id、event_type=content.upsert、occurred_at、content。禁止发送 title、detail_url、external_content_id、version、sequence 或学校、通道、群、设备字段。content 为本项目生成的“摘要 + 换行 + 原始链接”，不单独增加标题或“查看详情：”；无链接入口只发送摘要。摘要最多 20 个 Unicode 标量，超过时取前 18 字加“……”，空格、标点与省略号计入上限。完整正文最多 200 个 Unicode 标量，先预留完整链接与换行预算，不能截断链接。mp://、#小程序:// 等内容链接原样保存，来源层不进行 URL 校验。

首次受理返回 202，相同事件、相同请求字节幂等重投返回 200；不同原文冲突。回执只有 event_id、status=received、received_at，匹配事件与回执身份后本地记为 accepted，结束来源投递，不周期性查询内部处理或发送结果。响应丢失且没有 receipt_id 时使用同一身份、同一请求字节重投。GET 回执只核对受理事实。

下架、停用学校、路由改绑只取消未尝试的本地消息，不发送 content.withdrawn，不撤回已受理消息，不批量向新路由重发历史内容。暂停绑定保留未交付积压；已经尝试但响应未知的请求保留原身份以便幂等核对。浏览、点赞、计数和重复审核不推送。

content_empty、content_too_long、forbidden_field、unsupported_event_type 等明确错误码保留用于诊断；429、网络和服务错误退避，鉴权或配置错误阻塞等待修复。确定性内容错误不能通过重试改写冻结字节。受理不表示实际发送成功，本项目不维护上游发送状态。

所有认证、路由检查、消息提交和回执查询均由 Linjian 主动请求上游。WeKeeper 不回调或探测 Linjian，不配置下游可信网址或内容路径。连接认证不依赖 LINJIAN_ADMIN_WEB_ORIGIN；浏览器 Origin/CSRF 校验继续独立生效。
