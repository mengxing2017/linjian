import type { Metrics, Bucket } from './api'
export const metricNames: Record<keyof Metrics, string> = {
  daily_posts: '日均发帖',
  daily_comments: '日均评论',
  posts: '新增帖子',
  comments: '新增评论',
  post_authors: '发帖人数',
  comment_authors: '评论人数',
  participants: '内容参与人数',
  registrations: '新增注册人数',
  cumulative_registrations: '累计注册人数',
  new_participants: '新注册内容参与人数',
  active_users: '注册人员访问活跃人数',
  registered_participants: '注册人员内容参与人数',
  comment_post_ratio: '评论/发帖量比（条/帖）',
  new_participation_rate: '新注册内容参与率（%）',
  participation_rate: '注册人员内容参与率（%）',
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
  if (key === 'posts' || key === 'daily_posts') return '帖'
  if (key === 'comments' || key === 'daily_comments') return '条'
  if (key === 'comment_post_ratio') return '条/帖'
  return '人'
}
export function metricNote(key: keyof Metrics, coverage: Bucket['coverage'], value: number | null) {
  if (value == null) return '暂无可确认数据'
  if (
    ['posts', 'comments', 'post_authors', 'comment_authors', 'participants'].includes(key) &&
    !coverage.content
  )
    return '已记录，历史覆盖未确认'
  if (key === 'registrations' && !coverage.registration) return '已记录，注册时间历史覆盖未确认'
  return ''
}
