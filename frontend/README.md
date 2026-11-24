# Frontend 工作区

- `apps/web`：React + Ant Design 管理/运营与现有社区 Web，部署到 Caddy。
- `apps/miniapp`：uni-app + Vue 3 + TypeScript 微信小程序，产物上传微信平台。
- 根目录 `api-contract/`：唯一接口 DTO、JSON:API 协议与业务 client；不部署。

## 开发与验证

使用 Node.js 26 与 pnpm 11.22.0（可进入 `devbox shell`），从仓库根运行：

```bash
make install
make check-web
pnpm --dir frontend dev
pnpm --dir frontend dev:miniapp
```

`make check-web` 包含 contract 的浏览器及无 DOM 类型检查、契约测试、双客户端类型检查/lint/测试/生产构建与导入边界检查。单独校验小程序使用 `make check-miniapp`。

Web 界面测试使用 JSDOM 31，并通过测试专用 ConfigProvider 设置 `motion: false`，保留真实 Ant Design 组件与样式可见性检查。测试文件串行运行，单项超时 15 秒；工作区测试命令实时输出各项目进度，避免通过延长等待时间掩盖交互问题。命令行可单独运行并显示每项耗时：

```bash
devbox run -- pnpm --dir frontend/apps/web exec vitest run --reporter=verbose
```

Web 默认 `http://localhost:5173`，通过 Vite 代理请求 `LINJIAN_DEV_API_UPSTREAM`（默认 `http://127.0.0.1:8080`）。根 `frontend dev` 继续转发参数给 Web，例如 `pnpm --dir frontend dev --host 127.0.0.1 --port 5174`。

开发和构建通过 Vite 的精确入口别名读取仓库 `api-contract/` 源码，避免 pnpm `file:` 安装副本在接口更新后保留旧导出。修改契约源码可直接触发开发服务更新；新增 package exports 或修改 Vite 配置后重启开发服务。

## 微信小程序配置

1. 将 `apps/miniapp/.env.example` 复制为同目录 `.env.local`，配置 `VITE_LINJIAN_API_BASE_URL` 为实际 API 的 HTTPS origin，不带路径，不带凭据。这里使用与 Web 相同的 Rust API，不接 PHP 或 uniCloud。
2. 在 `apps/miniapp/src/manifest.json` 的 `mp-weixin.appid` 中设置自己的小程序 AppID（写入构建产物）；本地调试的 appid 只放在 `apps/miniapp/project.private.config.json`（不存在时新建，该文件已被 `.gitignore` 忽略）。`apps/miniapp/project.config.json` 允许提交，禁止写入 appid，`pnpm lint` 与 `make check-miniapp` 会校验。仓库不填示例真实身份，不包含 AppSecret。后端配置对应的 `LINJIAN_WX_APP_ID`、`LINJIAN_WX_SECRET`（准确配置名见根 env.example）。
3. 微信后台配置对应请求域名；图片通过 `uni.request` 认证读取原始字节后写入本地临时文件，私有图片不依赖浏览器 Cookie，不向第三方图片地址发送 Bearer。
4. 调试：先运行 `pnpm --dir frontend dev:miniapp`（watch 编译，产物在 `apps/miniapp/dist/dev/mp-weixin`），再用微信开发者工具导入 `apps/miniapp` 目录；其 `project.config.json` 的 `miniprogramRoot` 已指向 `dist/dev/mp-weixin/`。发布：运行 `pnpm --dir frontend build:miniapp`，用工具导入 `apps/miniapp/dist/build/mp-weixin`。
5. 用真实身份及隔离学校验收登录、绑定、图片选择、发布、后台审核、详情互动和弱网恢复。构建成功不表示真实微信身份或真机验收完成。

默认开启小程序域名检查，关闭 uniStatistics。生产发布与微信审核单独进行，不加入 Compose；Web Dockerfile 安装/构建 contract + web，；使用帮助由 Web public/docs 静态文件随 Web dist 交付。

## 接口与边界

帖子详情互动使用只读 `GET /api/v1/schools/{school}/posts/{id}/interaction-state` 同步计数和本人状态，点赞、收藏、关注与评论刷新不重新请求带浏览计数的详情。社区读取的点赞数、评论数为必填字段，缺失按协议错误处理，不兼容旧响应。评论包含回复，公开计数仅包含已通过、可见、未删除记录，待审提交不直接加数。

小程序仅开放用户主动好友/群分享，使用中性标题和本地静态封面，不外发私有图片或帖子标题；分享不授予学校权限。默认封面生成源为 `apps/miniapp/tools/build-share-cover.mjs`，修改后运行 `node frontend/apps/miniapp/tools/build-share-cover.mjs`（仓库根目录）更新 PNG。朋友圈、码/海报和复制链接不在本期开放，真机验证与发布验收仍须独立完成。

只有 feature 的 `api/index.ts`、`api/client.ts`、`api/contract.ts` 引用 `@linjian/api-contract/*`，禁止绕过 package exports 深层导入。所有 `index.ts` / `index.tsx` 只保留导入、导出及类型声明，不创建客户端、定义组件或初始化状态；组件实现在具名 `*-page.tsx` 中，feature 客户端装配在 `api/client.ts` 中。

两端从各自 `app/api` 取得共享 client。小程序只在 `app/api/client.ts` 调用一次 `createClient(createUniTransport(...))`；它通过 identity 的纯 `api/contract.ts` 获取平台协议工厂，不依赖 identity API 实例或其入口。Web 的应用级 client 复用 contract 已有浏览器单例，确保 Cookie、CSRF、multipart 和会话过期监听共享同一状态，不额外创建 transport 或 client。identity API 保留原有 `client` 导出，但其他 feature 的客户端不得借此获取共享 client。入口的 re-export 仍可能加载单例装配模块，不承诺运行时导入完全无初始化。

Web 与小程序禁止互相引用，社区不得依赖管理工作区，shared 不定义 DTO/client。`tools/check-boundaries.mjs` 检查静态导入、类型导入、re-export、动态导入、Vue script、跨 feature 的 `api/client` 深层导入和运行时循环依赖；纯类型依赖不计入运行时循环。`tools/public-entries.test.mjs` 对照拆分前的公开符号及类型/值角色，防止入口重构丢失导出。

平台中立 client 通过 transport 传输，统一编码 JSON:API、验证资源/分页并保留服务器错误。Web 保留 Cookie + CSRF 兼容 facade；小程序使用独立 Bearer 和可信 HTTPS origin。写入不自动重试，响应丢失时先核对“我的发布”。

小程序学校缓存、页面请求代次、私有图片与草稿按账号/学校隔离；切校取消旧请求，撤权清理页面，退出清 token 与私有图片。草稿保留于本机，只能在相同账号与学校恢复，编辑器隐藏时自动保存非空草稿，发布成功删除对应草稿。敏感终端可在丢弃草稿后退出。

## 迁移范围

### 小程序根导航与发布

小程序使用“首页、动态、消息、我的”四个原生根 Tab；`apps/miniapp/src/custom-tab-bar/` 是原生微信自定义底栏，中间加号通过统一导航打开独立发布页，不占第五个根 Tab。`routes.json` 是根页映射来源，构建后会校验配置与原生组件产物；不需要原项目服务或 UI 框架。

应用通过 `app/tabbar.ts` 将当前路由、学校主题和未读状态桥接到原生组件。根页跳转使用 `app/navigation.ts`，正常切换不清空页面实例；动态页固定展示关注内容。详情、搜索、资料、选校及编辑器不显示根底栏。安全区和底部占位只加在根页容器。

发布编辑器的“取消”支持继续编辑、保存草稿退出和放弃修改退出；系统返回与后台隐藏采用本地自动保存，不承诺微信系统手势会弹退出确认。放弃或成功提交后不在 hide 中复活草稿；账号/学校变化不写入另一作用域。成功后替换编辑页进入详情并保留来源根页；直接进入编辑器时补建首页返回路径。登录续接仅接受固定帖子详情或发布意图，有效期 30 分钟，已知账号不跨账号续接。

消息角标读取 `GET /api/v1/schools/{school}/notification-summary`，按账号和学校共享权威未读总数；0 隐藏，超过 99 显示 `99+`。前台恢复及进入消息页按 30 秒新鲜度刷新，成功已读强制重查，旧请求不得覆盖已读后的结果。首次失败不显示虚构数量；旧 API 暂无该接口时不阻断导航或通知列表。消息页支持下拉刷新，普通切回保留列表与分页，汇总变化时重载列表。

真机视觉、安全区、页面栈和系统返回仍需使用微信开发者工具及 iOS/Android 实机验收，自动构建检查不代替这些结论。

保留 18 个旧注册路径，新增非 tabBar 的动态编辑页以支持带参数跳转，使用薄页面 + feature 组件组织；页面生命周期由注册页显式转发给 feature。功能包括发现/关注/收藏/搜索/我的发布、详情评论回复与举报、图片发布/编辑/删除、学校绑定/切换、昵称、通知已读、资源与反馈图片。

旧独立分类改为校园服务入口，标签页展示配置标签，不冒充服务端精确标签筛选；旧粉丝及点赞人员明细明确下线，我的关注与互动消息保留。位置、视频、手机号/头像修改、短信登录不在首期恢复。旧整数详情链接不自动猜测 UUID，需使用已验证的迁移映射后再做兼容发布。

复用旧默认主题色、双列内容卡片/图片预览等交互及默认头像素材。未整包复制旧 UI 库和混合 PHP 调用的巨型页面；Vue 2 全局注入已由 Vue 3 feature 与显式 client 代替。

## 公开入口约定

`index.ts`、`index.tsx` 是公开入口：保留稳定的默认导出、命名导出和类型导出；页面入口只装配会话与具名组件。查询、状态、事件处理和业务表单放在具名模块，模块内部不反向依赖自身的入口。

- Web 社区入口接入会话，`community-content.tsx` 协调学校和页面数据，发布、评论、帖子操作、消息、个人信息及反馈各有独立组件。
- 身份入口导出 `session-gate.tsx`，登录表单在 `sign-in.tsx`；学校上下文的查询在 `queries.ts`。
- 后台各 feature 入口导出具名业务页面；工作区的业务拆分见其目录下的 README。
- `app/shell/index.tsx` 只导出布局、角色文案和导航；管理端页面入口保持默认导出兼容。
- API 的 `index.ts` 是 contract 适配边界，保留 client 装配与类型导出。只有这些文件可以直接引用 `@linjian/api-contract`，其他业务模块通过所属 feature 的 API 入口访问。
- 小程序的 `pages/**/index.vue` 保留页面注册、主题和 `useScreen` 生命周期接线；业务实现位于 feature 组件。Web 启动页和文档的 `index.html` 保留静态页面入口职责。

`make check-web` 包含公开入口边界检查，防止状态、查询和业务函数重新进入 TypeScript 的 index 文件。小程序与 Web 仍独立构建，禁止互相引用。
