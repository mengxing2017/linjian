# 管理工作区

`index.tsx` 是公开入口，只接入会话并导出兼容接口。业务模块不从入口反向导入。

- `workspace-shell.tsx`：导航、URL 兼容、学校选择、访问权限及工作区缓存刷新。
- `school-workspace-page.tsx`：已授权学校页面的显式分发；新增业务页面在此接入，不将导航键强转为 API 资源名。
- `content-moderation.tsx`：审核模式和帖子、评论页签；具体审核分别由 `post-reviews.tsx`、`comment-reviews.tsx` 处理。
- `cases.tsx`、`school-resources.tsx`、`global-resources.tsx`、`audit-log.tsx`：举报处理、学校展示资源、平台默认资源及操作审计。
- `record-list.tsx`：共用查询、搜索、分页、详情容器及基础展示。通过列和详情插槽复用，不根据业务资源名切换审核、编辑或举报处理逻辑。
- `state-label.ts`：状态文案。所有业务请求和 contract 类型通过 `api/index.ts` 引入。

学校切换和页面切换保留组件重建边界，避免上一学校的分页、选中记录或编辑状态进入下一学校。评论的版本冲突处理归评论详情所有；平台默认资源只读判断归学校资源页面所有。

`workspace.test.tsx` 验证导航、权限、旧链接及跨页面业务流程；帖子审核、资源编辑、学校设置等保留各自现有测试。修改公共列表时，应同时验证评论列顺序、举报提交版本及平台资源只读行为。
