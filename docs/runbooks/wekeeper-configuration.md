# WeKeeper 连接部署配置

Linjian 使用后端部署配置中的 WeKeeper 服务地址、调用方凭据 ID 和签名密钥。一个下游对应一条 WeKeeper 调用方凭据；secret 直接用于 HMAC 签名，不在 Linjian 数据库保存，不需要加解密或 `LINJIAN_SOURCE_MASTER_KEY_V1`。

## Docker 配置

复制 `env.example` 为 `.env`，填写同一条 WeKeeper 凭据：

```dotenv
LINJIAN_WEKEEPER_ORIGIN=https://keeper.example.com
LINJIAN_WEKEEPER_KEY_ID=<WeKeeper 签发的 key_id>
LINJIAN_WEKEEPER_SECRET=<同一条凭据的 secret>
```

地址必须是规范 HTTPS origin，例如 `https://keeper.example.com`，允许显式端口；不要填写接口路径、末尾斜杠、query 或 fragment。不要将 `LINJIAN_PUBLIC_ORIGIN` 填成 WeKeeper 域名：它仍是 Linjian 自己的公开论坛地址，用于帖子详情链接。无需填写 source_id，内部归属由认证响应自动取得。

三项全部留空时不启用连接；部分填写或格式错误时 API 启动会返回明确的配置错误码，不输出配置值。Compose 将三项传给 API 运行时，不传给 Web 或小程序构建。修改 `.env` 后执行：

```sh
docker compose --env-file .env config --quiet
docker compose --env-file .env up -d --build api
```

单纯重启旧容器不会重新读取 `.env`。所有 API 实例使用一致的配置；`.env` 不提交版本库。独立启动二进制或 `make api-dev` 时，需由启动环境导出相同的三项变量；Make 不会自动加载 `.env`。

## 后台使用

“WeKeeper 连接”仅显示当前配置的地址、key_id 和已配置状态，不提供凭据录入或下载。点击“验证连接”检查真实凭据并保存连接元数据，学校消息路由直接使用此唯一全局 WeKeeper 配置，管理员只填写 WeKeeper 为本校分配的 route_id。每所学校只维护一条路由，一个 Linjian 实例只对应一个 WeKeeper，各校使用独立 route_id，不选择或填写连接 ID。修改并保存会更新本校原配置，停用后仍占用本校唯一配置，不新增第二条记录。验证需要现有管理员会话和 CSRF，失败不会保存成功状态。

连接 ID、内部归属、路由绑定、连接版本和消息回执继续保存在数据库。只有与配置中 origin、key_id 相符的连接才能取得签名 secret；更换到另一调用方凭据或另一 WeKeeper 实例不会给历史事件套用新凭据。相同凭据修复配置后可重新验证并恢复可重试队列，事件正文和 receipt_id 保持不变。WeKeeper 临时不可用不会阻止论坛正常写入。

## 已有部署升级

升级前将原连接的 origin、key_id 和原始 secret 填入上述三项。secret 可由 WeKeeper 调用方凭据详情重复查看；不需要从 Linjian 数据库解密导出。新增前向迁移删除旧 encrypted_secret 与 encryption_version 列，保留原连接、学校绑定、历史 Outbox 和回执标识。同一 origin、key_id 验证时复用已有连接。

移除旧 `LINJIAN_SOURCE_MASTER_KEY_V1`。旧 `LINJIAN_SOURCE_ENDPOINT`、`LINJIAN_SOURCE_KEY_ID`、`LINJIAN_SOURCE_SECRET` 不作为 v2 配置使用，统一改用本页三项名称。WeKeeper 自身的存储主密钥配置不受影响，也不应复制到 Linjian。

## 学校路由关系升级

数据库使用 school_id 唯一约束，学校消息路由为单条配置表单；上游 WeKeeper 由全局部署配置唯一确定，route_id 是本校消息的分发目标。学校保存接口只接受 route_id 和启停状态，后端自动解析唯一当前连接。停用既有路由不依赖上游在线或配置可用。更换本校路由只影响后续消息，历史 Outbox 继续使用冻结的原路由和连接版本。

如果旧版本已为同一学校配置多条路由，升级优先保留启用记录，并在同状态记录中保留 ID 最大的一条（旧版本创建使用 UUIDv7）。其余原配置快照保存到 school_source_binding_archive，便于核对，不再参与后续推送。只有一条配置的学校及其他学校不受合并影响；历史连接版本、消息及回执不删除。

## 动态配置、暂停与历史撤回

学校路由保存使用 `If-Match: "<revision>"`；未配置学校使用 `"0"`。请求 JSON:API 属性仍只接受 route_id 和 active。修订冲突返回 409，缺少修订返回 428。保存后立即生效，不重启服务；全局环境变更仍需重建 API 容器，所有实例配置必须一致。

无路由时，新内容以 v2 待绑定事件留存。全局配置与学校旧连接不匹配时进入待配置；不会借用旧连接。配置完成后后台一次性绑定原事件，正文与事件身份冻结。历史 v1 保留 legacy_version_blocked，不自动转换。

暂停学校消息推送保留未处理事件和原路由，不影响论坛发布或其他学校。恢复时继续使用事件原快照。路由 A 改为 B 后，后续正文更新会产生旧 A 的撤回和新 B 的独立事件；已请求但未取得回执的旧路由也计入历史。停用学校则建立持久撤回任务，覆盖所有历史可能有效的路由。撤回不受普通推送暂停限制，但仍受上游凭据与路由授权限制。重新启用学校不会重放历史导入。

来源事件通过短事务租约领取，网络调用期间不持有数据库事务。租约到期可重试，旧 worker 的过期结果不能覆盖新进展。来源发件页区分本地调度状态、上游回执状态与发送摘要；支持查看原路由、冻结时间、关联事件和恢复记录。恢复必须填写原因，仅重新调度原事件；已知回执优先查询。终态拒绝、旧协议、原文损坏和事件冲突不能通过通用按钮解除。

## 本次队列结构升级

新增迁移 `202611090001_dynamic_school_outbox.sql`。升级前停止旧 worker，执行迁移后发布匹配 API 和管理端，再恢复来源投递；禁止新旧 worker 混跑。历史 v2 正文、摘要、事件 ID 和回执保持不变；旧绑定修订无法还原的部分标记为迁移基线，不伪造校验时间。停用学校的既有路由通过持久任务补充撤回。

出现问题可暂停来源 worker 并以兼容新结构的版本修复；数据库只追加修复迁移，不逆向删除历史记录。论坛事务仍须可靠写入 Outbox，不能关闭原子性约束来维持写入。监控可按学校统计 Outbox 状态和最老积压，检查 source_reconciliation_jobs 未完成任务及其 processed 进度，不记录密钥、签名或完整正文。
