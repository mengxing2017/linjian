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
| `0015_school_permission_catalog_descriptions.sql` | 同步帖子推荐、学校统计查看与导出三项权限的说明文案，保持 SQL 与应用权限目录一致；不改变授权规则 |
| `0016_school_coordinates_legacy_compatible.sql` | 允许历史学校保留空坐标，使用 NOT VALID 检查约束保证新增和更新后的学校坐标完整，不扫描或阻断历史数据 |

`0001` 至 `0010` 保持独立空库基线；新增功能通过后续 forward-only migration 演进，不改写已执行的基线文件。

表内校验、主键、唯一约束及索引随业务域定义；跨表外键和触发器统一在表定义之后建立。初始数据使用运行时生成的时间与 ID，不包含业务数据。运营报表覆盖起点取自 `application_metadata.created_at`，不依赖某个迁移版本号。

已删除旧增量迁移、重复函数定义、旧协议中间结构与历史回填，以及无运行时用途的 `blocked_word_legacy_snapshot`、`school_source_binding_archive` 归档表。快照工具仍使用的 ID 映射与导入记录保留。

曾运行旧迁移的本地临时库应重新创建为空库，不能仅删除 `_sqlx_migrations` 后在旧结构上重跑。不要对需要保留数据的数据库执行此操作。本次整理不提供现有数据库升级路径；基线部署后的结构变更继续追加 forward-only migration，不修改已部署文件。

## 验证

### 权限目录说明同步（0015）

`0015_school_permission_catalog_descriptions.sql` 在迁移时更新 `forum_permission_catalog()` 的三处说明，保留目录顺序、权限键、角色默认值、依赖及风险标记，不改变权限目录版本或已有成员授权。不改写已执行的基线或 `0012`，新库初始化与已执行至 `0014` 的库均按正常 SQLx 迁移流程应用。

隔离数据库集成测试覆盖旧目录升级、应用与 SQL 完整目录一致性、仅三项说明变化、函数不可变属性及重复执行迁移无变化。

### 学校坐标写入必填与历史兼容（0016，接替未执行的 0014）

`0014_school_coordinates_required.sql` 保留原文件和校验和，不再对未执行它的数据库运行。应用迁移入口在 SQLx 迁移锁内读取执行记录：已执行 0014 的库仍校验原校验和；未执行（包括因空坐标事务回滚）的库跳过 0014，不伪造执行记录，由 0016 接替。其余迁移仍执行 SQLx 的版本、校验和及失败状态检查。启动、创建管理员和快照工具统一使用 `database::migrate`，不要对待升级库直接调用原始 MIGRATOR 或外部 SQLx CLI。

0016 移除列级 NOT NULL，添加 `CHECK (latitude IS NOT NULL AND longitude IS NOT NULL) NOT VALID`：历史学校（包括隐藏学校）可保留空值并正常读取，不扫描资料完整性或阻断启动；新增和任何更新后的学校必须有完整坐标。编辑历史学校即使只改名称或主题，也须补齐坐标，否则返回业务校验错误。已有完整坐标的学校仍支持不提交坐标的局部更新，不允许清空坐标。读取接口保留 JSON null，推荐接口跳过缺少坐标的学校。补齐时核实真实 GCJ-02 坐标，禁止用零坐标或城市中心占位。不要删除迁移记录、清库或回退到将读取坐标视为非空的旧 API。

```sh
python3 tools/test_migrations.py --bin-dir /path/to/postgresql/bin
devbox run -- make check-backend
LINJIAN_TEST_DATABASE_URL=postgresql://... devbox run -- make test-integration
```

第一个命令使用临时 PostgreSQL 单用户模式，不连接现有数据库；集成测试只接受显式的隔离测试库地址。
