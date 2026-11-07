# 论坛来源推送协议 v1

本项目实现发件端；接收端在论坛迁移完成后由 wekeeper 独立实现。未配置接收端时，论坛正常运行，Outbox 持久积压。发布和审核通过只产生来源事件，不产生群发送任务或授权。

`POST /api/v1/source-events` 使用 `application/vnd.api+json`，data.type 为 `source-events`，data.id 与 attributes.event_id 一致。attributes 包含 schema_version=1、event_type、external_organization_id、external_content_id、version、sequence、occurred_at、title、content、detail_url。撤回正文为 null。签名及 JSON 字节向量见 `api-contract/fixtures/source-event-v1.json`，由 Rust 与 TypeScript 两侧测试核对。

请求头：`X-Source-Version: 1`、`X-Source-Key-Id`、`X-Source-Timestamp`（UTC Unix 秒十进制）、`X-Source-Nonce`（每次请求新 UUID）、`Idempotency-Key`（稳定 event_id）、`X-Source-Signature`（小写十六进制 HMAC-SHA256）。secret 至少 32 字节，仅后端读取。

规范签名字符串是以下 UTF-8 字段以 LF 连接，末尾没有 LF：协议版本、key_id、HTTP 方法大写、规范绝对路径（不含域名或查询）、时间戳、nonce、幂等键、请求体 SHA-256 小写十六进制。GET 收件查询的请求体为空，仍使用原事件 ID 作为幂等键。接口路径不接受查询、片段、重定向或 URL 用户信息。

正文在第一次网络调用之前持久冻结，重试不改变字节、event_id 或幂等键，只更新时间戳、nonce、签名。接收端须在 5 分钟窗口内校验时间戳并持久去重 nonce；以来源和 event_id 去重业务事件，同键不同正文返回 409；来源凭据绑定 namespace 和组织范围。

收件响应必须为 JSON:API `source-receipts`，data.id 为收件 UUID，attributes.status 为 received、applied、ignored_stale 或 rejected。客户端只在合法响应后保存收件 ID。响应丢失、网络或鉴权错误均保留事件，以 5 秒起指数退避、上限 1 小时重试；日志和列表仅保存错误代码，不存上游认证响应正文。

`GET /api/v1/source-receipts/{id}` 使用同样的来源签名，读取当前收件处理及发送摘要。received/applied 每 30 秒轮询，applied 只表示素材已更新；不显示为微信群已发送。ignored_stale/rejected 保留记录并停止自动查询。运营可在本校来源发件记录中重试原事件；重试仍受接收端幂等语义约束，不能创建新的发送意图。

来源事件以学校和帖子为范围分配严格递增 sequence。新建待审核内容不推送；通过审核且可见时推送完整正文；正文编辑、重新通过及重新展示产生新版本；隐藏、删除、撤销审核通过或停用学校产生撤回。浏览、点赞、收藏和计数更新不产生事件。导入历史不生成事件；导入后的业务变更按新的序号推进。

跨库传输有延迟。撤回只通知素材失效，无法撤回群内消息；接收端仍须单独执行学校授权、预览和人工确认。两边密钥轮换可保留有界重叠期；发件端更新 key_id/secret 后使用原冻结正文重试，不能为轮换改动历史事件。
