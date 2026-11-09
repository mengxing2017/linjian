# 论坛来源推送协议

当前新连接使用 v2；Linjian 只产生来源事件，不创建群发送任务或发送授权。上游故障不影响帖子事务，Outbox 持久保留事件。

## v2 机器契约

WeKeeper 发布 POST `/api/v2/source-authentications`、POST `/api/v2/source-route-checks`、POST `/api/v2/source-events` 和 GET `/api/v2/source-receipts/{receipt_id}`。JSON:API schema、错误目录和发布清单位于 `api-contract/source-v2/`，字节向量位于 `api-contract/fixtures/source-v2.json`。Rust 与 TypeScript 都读取同一发布版本的测试材料。

事件使用 `route_id`，不再提交 `external_organization_id` 或学校字段；也不能指定通道、群、设备和广告。每个下游对应一条调用方凭据，在后端部署环境配置 LINJIAN_WEKEEPER_ORIGIN、LINJIAN_WEKEEPER_KEY_ID 和 LINJIAN_WEKEEPER_SECRET。key_id 同时标识调用方与其凭据；内部 source_id 由后端从已认证的能力元数据自动取得，不在连接页面填写或展示。上游按当前 key 与路由明确关联鉴权，不授予同平台全部路由。

签名头为 X-Source-Version: 2、X-Source-Key-Id、X-Source-Timestamp、X-Source-Nonce、Idempotency-Key、X-Source-Signature。签名 secret 是至少 32 随机字节的无填充 base64url 交付文本，双方解码后使用原始字节。

规范字符串依次为版本、key_id、大写方法、完整外部路径、Unix 秒、nonce、幂等键、原文 SHA-256 小写十六进制，以 LF 分隔且无尾 LF。HMAC-SHA256 签名输出小写十六进制。GET 正文为零字节，幂等头为原 event_id；认证和范围检查使用独立请求 UUID。请求时间绝对偏差不超过 300 秒。每次尝试生成新 nonce，重试不改变原文、事件身份或路由。

平台管理员在只读连接页面点击“验证连接”会验证部署配置中的调用方凭据，保存的提示能力只在 60 秒内用于界面提示；每个真实请求仍由 WeKeeper 按当前凭据及授权校验。HTTPS origin 禁止用户信息、query、fragment，所有请求禁止自动重定向。secret 仅从后端配置读取用于签名，不在数据库保存，也不使用本实例加密主密钥；普通读取接口不回显。部署步骤见 [WeKeeper 配置](wekeeper-configuration.md)。

每所学校仅有一条当前消息路由；一个 Linjian 实例只对应一个由后端配置指定的 WeKeeper；各校共用此配置，只维护独立 route_id 与启停状态，无需选择连接。保存配置更新原绑定，停用仍保留该唯一记录。业务事务按学校绑定冻结连接版本、公开内容 origin、来源、路由、正文和摘要。正文有意义变更产生新 event_id 与单调 sequence；浏览、点赞和计数不产生事件。停用学校产生内容撤回，上游不维护学校对象。

同 `(source_id,event_id)` 同文重试返回原 receipt；不同原文冲突。收到合法 JSON:API 回执才记录 receipt_id。received 继续查询；applied 不代表发送成功，无发送摘要能力时停止轮询，有待处理摘要时最多低频同步一天；ignored_stale/rejected 停止无效重投。429、网络及服务错误退避；鉴权、授权、结构或版本问题保留事件并阻塞等待恢复。重试操作不会清空原 receipt_id 或生成新事件。

来源或 key 停用、删除后新请求均拒绝。移除路由关联禁止该 key 继续提交或读取该路由回执。路由暂停禁止新消息但有效关联仍可查历史。既有任务按上游原任务规则处理，凭据删除不隐式撤回内容。

## v1 历史兼容

v1 DTO、规范签名函数及 `source-event-v1.json` fixture 保留供识别和迁移验证，签名仍使用原有文本 secret 字节。v1 包含 external_organization_id，不能原地加入 v2 route_id。

当前 worker 只投递 v2；已有 v1 行保留原 ID、正文和 receipt，标记 legacy_version_blocked，不默认转投 v2。没有已核实可用的 v1 服务时须保留队列并制定显式迁移安排，不能在重试中改正文或生成新 ID 隐藏冲突。旧全局来源环境变量不自动变成动态 v2 连接。

调用方向：所有认证、路由检查、事件推送和回执查询均由 Linjian 主动请求上游 WeKeeper。WeKeeper 不回调 Linjian，不探测其在线状态，不配置下游可信网址或路径前缀。事件 detail_url 仅为消息内容链接，不作为回调目标。
