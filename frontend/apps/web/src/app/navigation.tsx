import type { ReactNode } from 'react'
import {
  AlertOutlined,
  ApartmentOutlined,
  ApiOutlined,
  AppstoreOutlined,
  AuditOutlined,
  BankOutlined,
  BarChartOutlined,
  BranchesOutlined,
  CommentOutlined,
  CustomerServiceOutlined,
  DashboardOutlined,
  FileSearchOutlined,
  GlobalOutlined,
  IdcardOutlined,
  InfoCircleOutlined,
  LineChartOutlined,
  PictureOutlined,
  QrcodeOutlined,
  SafetyCertificateOutlined,
  SendOutlined,
  SettingOutlined,
  SlidersOutlined,
  StopOutlined,
  TagsOutlined,
  TeamOutlined,
  UserAddOutlined,
  UsergroupAddOutlined,
} from '@ant-design/icons'

export type NavigationDomain = 'statistics' | 'forum' | 'operations' | 'school' | 'source' | 'administration'
export const navigationDomains = [
  { key: 'statistics', label: '数据统计' },
  { key: 'forum', label: '论坛管理' },
  { key: 'operations', label: '运营中心' },
  { key: 'school', label: '学校管理' },
  { key: 'source', label: '来源推送' },
  { key: 'administration', label: '系统管理' },
] as const
export interface NavigationItem {
  key: string
  label: string
  description: string
  domain: NavigationDomain
  scope: 'school' | 'global' | 'selectable'
  capability: 'school.operate' | 'school.read' | 'school.admin' | 'global.manage' | 'resources.read'
  icon: ReactNode
  hidden?: boolean
}
export const navigation: NavigationItem[] = [
  { key: 'statistics', label: '统计概览', description: '查看所选范围的核心指标。', domain: 'statistics', scope: 'selectable', capability: 'resources.read', icon: <DashboardOutlined /> },
  { key: 'statistics-content', label: '内容统计', description: '查看帖子、评论与内容参与趋势。', domain: 'statistics', scope: 'selectable', capability: 'resources.read', icon: <BarChartOutlined /> },
  { key: 'statistics-registration', label: '注册统计', description: '查看新增与累计注册趋势。', domain: 'statistics', scope: 'selectable', capability: 'resources.read', icon: <UserAddOutlined /> },
  { key: 'statistics-activity', label: '活跃统计', description: '查看注册人员访问与参与趋势。', domain: 'statistics', scope: 'selectable', capability: 'resources.read', icon: <LineChartOutlined /> },
  { key: 'statistics-schools', label: '学校对比', description: '按学校对比统计指标。', domain: 'statistics', scope: 'global', capability: 'global.manage', icon: <ApartmentOutlined /> },


  {
    key: 'posts',
    icon: <FileSearchOutlined />,
    label: '内容管理',
    description: '管理帖子与评论，维护校园内容展示。',
    domain: 'forum',
    scope: 'school',
    capability: 'school.read',
  },
  { key: 'home-menu', icon: <TagsOutlined />, label: '首页菜单/标签', description: '管理首页菜单与发布候选标签。', domain: 'forum', scope: 'school', capability: 'school.read' },
  {
    key: 'cases',
    icon: <AlertOutlined />,
    label: '举报管理',
    description: '处理用户举报，保留处置记录。',
    domain: 'forum',
    scope: 'school',
    capability: 'school.read',
  },
  {
    key: 'resources',
    icon: <PictureOutlined />,
    label: '展示资源',
    description: '管理校园展示资源与服务入口。',
    domain: 'forum',
    scope: 'school',
    capability: 'school.read',
  },
  {
    key: 'platform-resources',
    icon: <AppstoreOutlined />,
    label: '平台默认展示配置',
    description: '维护跨学校共用的默认展示资源。',
    domain: 'forum',
    scope: 'global',
    capability: 'global.manage',
  },
  { key: 'blocked-words', icon: <StopOutlined />, label: '学校屏蔽词管理', description: '维护本校屏蔽词，查看对本校生效的平台规则。', domain: 'forum', scope: 'school', capability: 'school.read' },
  { key: 'platform-blocked-words', icon: <GlobalOutlined />, label: '平台屏蔽词管理', description: '管理所有学校共用的屏蔽词。', domain: 'forum', scope: 'global', capability: 'global.manage' },
  { key: 'group-qrcodes', label: '群二维码', description: '管理群二维码及展示时机。', domain: 'operations', scope: 'selectable', capability: 'resources.read', icon: <QrcodeOutlined /> },
  { key: 'home-banners', label: '首页广告位', description: '管理首页广告及曝光点击统计。', domain: 'operations', scope: 'selectable', capability: 'resources.read', icon: <SlidersOutlined /> },
  { key: 'customer-config', label: '客户配置', description: '配置平台合作、加盟与客服二维码。', domain: 'operations', scope: 'global', capability: 'global.manage', icon: <CustomerServiceOutlined /> },
  { key: 'feedback', label: '投诉反馈', description: '查询与删除用户投诉反馈。', domain: 'operations', scope: 'school', capability: 'school.read', icon: <CommentOutlined /> },
  {
    key: 'schools',
    icon: <BankOutlined />,
    label: '学校管理',
    description: '查看学校列表，创建学校并进入学校配置或成员管理。',
    domain: 'school',
    scope: 'global',
    capability: 'global.manage',
  },
  {
    key: 'miniapp-users', icon: <TeamOutlined />, label: '用户管理',
    description: '管理微信小程序注册用户的资料、所属学校及正常或封禁状态。',
    domain: 'school', scope: 'school', capability: 'school.read',
  },
  {
    key: 'members',
    icon: <UsergroupAddOutlined />,
    label: '学校成员与权限',
    description: '查看本校成员及角色，学校授权由平台管理员维护。',
    domain: 'school',
    scope: 'school',
    capability: 'school.admin',
  },
  {
    key: 'settings',
    icon: <SettingOutlined />,
    label: '学校设置',
    description: '设置学校信息、审核方式与主题颜色。',
    domain: 'school',
    scope: 'school',
    capability: 'school.admin',
  },
  {
    key: 'source-connections',
    label: 'WeKeeper 连接',
    description: '配置来源凭据和协议连接。',
    domain: 'source',
    scope: 'global',
    capability: 'global.manage',
    icon: <ApiOutlined />,
  },
  {
    key: 'source-bindings',
    label: '学校消息路由',
    description: '维护本校唯一消息路由，统一使用全局 WeKeeper 配置。',
    domain: 'source',
    scope: 'school',
    capability: 'school.admin',
    icon: <BranchesOutlined />,
  },
  {
    key: 'source-outbox',
    icon: <SendOutlined />,
    label: '来源发件记录',
    description: '查看论坛推送与收件结果，重试失败事件。',
    domain: 'source',
    scope: 'school',
    capability: 'global.manage',
  },
  {
    key: 'administrators',
    icon: <SafetyCertificateOutlined />,
    label: '平台管理员',
    description: '查看平台管理员身份与状态，管理登录密码。',
    domain: 'administration',
    scope: 'global',
    capability: 'global.manage',
  },
  {
    key: 'accounts',
    icon: <IdcardOutlined />,
    label: '登录账号管理',
    description: '管理全平台普通账号及登录状态。',
    domain: 'administration',
    scope: 'global',
    capability: 'global.manage',
  },
  {
    key: 'site-settings',
    icon: <InfoCircleOutlined />,
    label: '站点设置',
    description: '维护全平台网站名称、备案与版权信息。',
    domain: 'administration',
    scope: 'global',
    capability: 'global.manage',
  },
  {
    key: 'audit',
    icon: <AuditOutlined />,
    label: '操作审计',
    description: '追踪学校内的关键操作与变更记录。',
    domain: 'administration',
    scope: 'school',
    capability: 'school.read',
  },
]

export function canNavigate(
  item: NavigationItem,
  platformAdmin: boolean,
  schoolRole?: string,
): boolean {
  switch (item.capability) {
    case 'school.operate':
      return schoolRole === 'admin' || schoolRole === 'operator'
    case 'global.manage':
      return platformAdmin
    case 'resources.read':
      return platformAdmin || schoolRole === 'admin' || schoolRole === 'operator'
    case 'school.admin':
      return platformAdmin || schoolRole === 'admin'
    case 'school.read':
      return platformAdmin || schoolRole === 'admin' || schoolRole === 'operator'
  }
}
export function visibleNavigationKeys(platformAdmin: boolean, schoolRole?: string) {
  return navigation
    .filter((item) => canNavigate(item, platformAdmin, schoolRole))
    .map((item) => item.key)
}
