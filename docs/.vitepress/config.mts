import { defineConfig } from 'vitepress'

export default defineConfig({
  lang: 'zh-CN',
  title: 'Linjian 文档',
  description: '校园论坛的启动、配置与使用说明',
  base: '/docs/',
  themeConfig: {
    nav: [
      { text: '首页', link: '/' },
      { text: 'Docker 构建与启动', link: '/docker-images' },
    ],
    sidebar: [
      {
        text: '启动与配置',
        items: [
          { text: 'Docker 构建与启动', link: '/docker-images' },
          { text: '来源连接配置', link: '/wekeeper-configuration' },
        ],
      },
      {
        text: '功能说明',
        items: [
          { text: '学校信息与主题', link: '/school-profiles' },
          { text: '小程序用户与登录', link: '/miniapp-users' },
          { text: '内容全文审核', link: '/content-review' },
          { text: '小程序内容来源推送', link: '/miniapp-publication' },
          { text: '运营中心', link: '/operations' },
        ],
      },
      {
        text: '开发参考',
        items: [
          { text: '来源推送协议', link: '/source-protocol' },
          { text: '数据迁移', link: '/migration' },
          { text: '独立论坛架构', link: '/adr/0001-independent-forum' },
        ],
      },
    ],
    search: { provider: 'local' },
    outline: { label: '本页目录', level: [2, 3] },
    docFooter: { prev: '上一页', next: '下一页' },
  },
})
