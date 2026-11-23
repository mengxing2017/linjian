# 学校消息尾部广告

每校最多一个未删除配置。学校 `admin` 持有相应业务权限才能维护草稿、提交、撤回和删除；平台只读取、审核和强制下架，不能仅凭平台身份代写学校文案。会话、角色与权限在事务内复查，沿用学校授权栅栏。

文案为规范化后 1–50 个 Unicode 标量的单段纯文本。草稿可编辑；提交后全文、指纹和提交证据不可变。新增或编辑必须经平台对明确修订审核，待审期间旧通过版本继续使用。删除或下架保留全部证据，重新创建必须重新审核。

管理事务和来源消息冻结都以学校行作为串行边界。`content_publication::PreparationStore::finish` 在成功冻结事务中选择当前通过修订，将 `原有完整 content + LF + 广告全文` 一次序列化。总长超过 200 标量时整段省略广告。采用快照、最终请求字节和哈希原子提交，已冻结事件不因广告变化重写或重复追加。

本领域不创建 Outbox 意图，不改帖子，不创建发送任务，不访问外部广告平台。WeKeeper v2 六字段、签名和回执保持不变。

结构变更在 `0012_school_message_advertisements.sql`，离线快照格式为 7，旧格式拒绝。共享规范化及追加样例为 `api-contract/fixtures/message-advertisement-rules.json`。集成测试注册于 `backend/tests/message_advertisements/`，只接受 `LINJIAN_TEST_DATABASE_URL`。
