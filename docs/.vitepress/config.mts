import { defineConfig } from 'vitepress'

export default defineConfig({
  lang: 'zh-CN',
  title: 'Linjian 文档',
  description: '校园论坛的启动、配置与使用说明',
  base: '/docs/',
  themeConfig: {
    nav: [
      { text: '首页', link: '/' },
      { text: 'Docker 构建与启动', link: '/runbooks/docker-images' },
    ],
    sidebar: [
      {
        text: '部署与运维',
        items: [
          { text: 'Docker 构建与启动', link: '/runbooks/docker-images' },
          { text: '来源连接配置', link: '/runbooks/wekeeper-configuration' },
          { text: '数据迁移', link: '/runbooks/migration' },
        ],
      },
      {
        text: '功能说明',
        items: [
          { text: '学校信息与主题', link: '/guide/school-profiles' },
          { text: '学校成员与业务权限', link: '/guide/member-permissions' },
          { text: '小程序用户与登录', link: '/guide/miniapp-users' },
          { text: '换校与学生证明审核', link: '/guide/miniapp-school-changes' },
          { text: '内容全文审核', link: '/guide/content-review' },
          { text: '仿真发帖', link: '/guide/simulated-posts' },
          { text: '小程序内容来源推送', link: '/guide/miniapp-publication' },
          { text: '运营中心', link: '/guide/operations' },
          { text: '服务入口、合作与官方营销', link: '/guide/operation-services' },
          { text: '统计概览', link: '/guide/business-statistics' },
          { text: '站点品牌与备案版权', link: '/guide/site-settings' },
        ],
      },
      {
        text: '开发参考',
        items: [
          { text: '来源推送协议', link: '/references/source-protocol' },
        ],
      },
      {
        text: '架构决策',
        items: [
          { text: '独立论坛架构', link: '/adr/0001-independent-forum' },
        ],
      },
    ],
    search: { provider: 'local' },
    outline: { label: '本页目录', level: [2, 3] },
    docFooter: { prev: '上一页', next: '下一页' },
  },
})
