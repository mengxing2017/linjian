import type { Metrics, Bucket } from './api'
export const metricNames: Record<keyof Metrics, string> = {
  daily_posts: '日均发帖',
  daily_comments: '日均评论',
  posts: '新增帖子',
  miniapp_posts: '小程序投稿', web_posts: '后台直接发布', seed_posts: '模拟投稿', unknown_posts: '未知来源投稿',
  comments: '新增评论',
  post_authors: '发帖人数',
  comment_authors: '评论人数',
  participants: '内容参与人数',
  registrations: '新增注册人数',
  cumulative_registrations: '累计注册人数',
  new_participants: '新注册内容参与人数',
  active_users: '活跃注册用户数',
  registered_participants: '参与内容互动的注册用户数',
  comment_post_ratio: '评论/发帖量比（条/帖）',
  new_participation_rate: '新注册内容参与率（%）',
  participation_rate: '注册用户内容参与率（%）',
}
const numberFormatter = new Intl.NumberFormat('zh-CN')
const timeFormatter = new Intl.DateTimeFormat('zh-CN', {
  timeZone: 'Asia/Shanghai',
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  hour12: false,
})
export const formatValue = (value: number | null | undefined) =>
  value == null ? '—' : numberFormatter.format(value)
export function shanghai(value: string) {
  return timeFormatter.format(new Date(value))
}

export function metricUnit(key: keyof Metrics) {
  if (key.endsWith('_rate')) return '%'
  if (key === 'posts' || key.endsWith('_posts')) return '帖'
  if (key === 'comments' || key === 'daily_comments') return '条'
  if (key === 'comment_post_ratio') return '条/帖'
  return '人'
}
export function metricNote(key: keyof Metrics, coverage: Bucket['coverage'], value: number | null) {
  if (value == null) return '暂无可用数据'
  if (
    ['posts', 'comments', 'post_authors', 'comment_authors', 'participants'].includes(key) &&
    !coverage.content
  )
    return '数据存在缺口，仅展示现有记录'
  if (key === 'registrations' && !coverage.registration) return '注册数据存在缺口或时间异常'
  return ''
}
