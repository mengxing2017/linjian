# 空库基线

当前项目没有历史业务数据，迁移已从按开发日期累积的增量脚本收敛为按业务域划分的独立空库基线。只支持从空库初始化，不提供旧迁移记录兼容、旧协议转换或历史数据回填。

SQLx 按文件编号顺序执行，所有 SQL 文件保持在本目录，不移动到子目录：

| 文件 | 内容 |
| --- | --- |
| `0001_identity_schools_settings.sql` | 应用元数据、账号、学校资料、成员权限、会话、审计、站点配置及快照工具元数据 |
| `0002_media.sql` | 文件资产、引用与租约、运营媒体及访问令牌 |
| `0003_community.sql` | 帖子、评论、互动、审核快照、案件、屏蔽词、内容导入及模拟帖子 |
| `0004_miniapp.sql` | 小程序用户与身份、注册配置、菜单、发布链接及换校申请与材料 |
| `0005_operations.sql` | 运营资源、营销配置、服务目标与审核版本、事件指标及报告 |
| `0006_source_outbox.sql` | 来源连接与版本、学校绑定、完整消息 Outbox、发布准备及推荐证据 |
| `0007_statistics.sql` | 注册事实、活跃事件、统计覆盖范围与快照 |
| `0008_foreign_keys.sql` | 跨表外键，统一在全部表创建后建立，解决循环引用 |
| `0009_functions_triggers.sql` | 当前有效函数及触发器，函数按依赖顺序创建 |
| `0010_initial_data.sql` | 安装元数据、站点与注册默认配置、来源配置及空库统计覆盖范围 |
| `0011_miniapp_phone_bindings.sql` | 后续 forward-only 迁移：微信已核验手机号的加密绑定状态及一次性授权请求去重记录；不将历史手填号码升级为认证 |
| `0012_school_message_advertisements.sql` | 学校消息尾部广告配置、封存修订、平台审核证据、冻结采用快照、管理员专属权限和新版消息拼接校验；离线快照格式升级至 7 |

`0001` 至 `0010` 保持独立空库基线；新增功能通过后续 forward-only migration 演进，不改写已执行的基线文件。

表内校验、主键、唯一约束及索引随业务域定义；跨表外键和触发器统一在表定义之后建立。初始数据使用运行时生成的时间与 ID，不包含业务数据。运营报表覆盖起点取自 `application_metadata.created_at`，不依赖某个迁移版本号。

已删除旧增量迁移、重复函数定义、旧协议中间结构与历史回填，以及无运行时用途的 `blocked_word_legacy_snapshot`、`school_source_binding_archive` 归档表。快照工具仍使用的 ID 映射与导入记录保留。

曾运行旧迁移的本地临时库应重新创建为空库，不能仅删除 `_sqlx_migrations` 后在旧结构上重跑。不要对需要保留数据的数据库执行此操作。本次整理不提供现有数据库升级路径；基线部署后的结构变更继续追加 forward-only migration，不修改已部署文件。

## 验证

### 学校坐标必填（0014）

`0014_school_coordinates_required.sql` 要求全部学校（包括隐藏学校）已有非空经纬度后才收紧约束。部署前通过现有受权学校维护入口补齐并核实 GCJ-02 坐标，明确多校区代表点；禁止以零坐标、城市中心或其他假值占位。安排短暂学校创建维护窗口，先补齐，再部署会自动执行迁移的新 API 和匹配的 Web。旧管理端无坐标创建将被拒绝。

迁移失败时修复真实资料后重试，不能删除 `_sqlx_migrations`、改已执行基线或对保留数据的库清库。推荐接口不持久化用户位置，不新增业务启动扫描或验收门禁。恢复优先向前修复，未经兼容核验不得回退到能创建空坐标的旧 API。

```sh
python3 tools/test_migrations.py --bin-dir /path/to/postgresql/bin
devbox run -- make check-backend
LINJIAN_TEST_DATABASE_URL=postgresql://... devbox run -- make test-integration
```

第一个命令使用临时 PostgreSQL 单用户模式，不连接现有数据库；集成测试只接受显式的隔离测试库地址。
