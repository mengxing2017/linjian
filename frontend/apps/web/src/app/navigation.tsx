import type { ReactNode } from 'react'
import {
  AlertOutlined,
  AppstoreOutlined,
  AuditOutlined,
  BankOutlined,
  FileSearchOutlined,
  FilterOutlined,
  SafetyCertificateOutlined,
  SettingOutlined,
  UserOutlined,
  UsergroupAddOutlined,
} from '@ant-design/icons'

export type NavigationDomain = 'forum' | 'school' | 'source' | 'administration'
export const navigationDomains = [
  { key: 'forum', label: '论坛功能' },
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
  {
    key: 'posts',
    icon: <FileSearchOutlined />,
    label: '内容管理',
    description: '管理帖子与评论，维护校园内容展示。',
    domain: 'forum',
    scope: 'school',
    capability: 'school.read',
  },
  { key: 'home-menu', icon: <FilterOutlined />, label: '首页菜单/标签', description: '管理首页菜单与发布候选标签。', domain: 'forum', scope: 'school', capability: 'school.read' },
  {
    key: 'cases',
    icon: <AlertOutlined />,
    label: '举报与反馈',
    description: '处理用户举报与反馈，保留处置记录。',
    domain: 'forum',
    scope: 'school',
    capability: 'school.read',
  },
  {
    key: 'resources',
    icon: <FilterOutlined />,
    label: '展示与屏蔽词',
    description: '管理校园展示资源、服务入口与内容规则。',
    domain: 'forum',
    scope: 'school',
    capability: 'school.read',
  },
  {
    key: 'platform-resources',
    icon: <AppstoreOutlined />,
    label: '平台默认展示配置',
    description: '维护跨学校共用的默认资源与屏蔽词。',
    domain: 'forum',
    scope: 'global',
    capability: 'global.manage',
  },
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
    key: 'miniapp-users', icon: <UserOutlined />, label: '用户管理',
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
    icon: <SettingOutlined />,
  },
  {
    key: 'source-bindings',
    label: '学校消息路由',
    description: '维护本校唯一消息路由，统一使用全局 WeKeeper 配置。',
    domain: 'source',
    scope: 'school',
    capability: 'school.admin',
    icon: <SettingOutlined />,
  },
  {
    key: 'source-outbox',
    icon: <FileSearchOutlined />,
    label: '来源发件记录',
    description: '查看论坛推送与收件结果，重试失败事件。',
    domain: 'source',
    scope: 'school',
    capability: 'school.read',
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
    icon: <UserOutlined />,
    label: '登录账号管理',
    description: '管理全平台普通账号、登录状态与企业微信身份映射。',
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
