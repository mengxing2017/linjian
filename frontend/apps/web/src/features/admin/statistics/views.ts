import type { Metrics, StatisticsView, StatisticsReport } from './api'
export const titles: Record<StatisticsView, string> = {
  overview: '统计概览',
  content: '内容统计',
  registration: '注册统计',
  activity: '活跃统计',
  schools: '学校对比',
}
export const metricGroups: Record<StatisticsView, (keyof Metrics)[]> = {
  overview: ['posts', 'comments', 'registrations', 'active_users'],
  content: ['posts', 'comments', 'post_authors', 'comment_authors', 'participants'],
  registration: [
    'registrations',
    'cumulative_registrations',
    'new_participants',
    'new_participation_rate',
  ],
  activity: ['active_users', 'registered_participants', 'participation_rate'],
  schools: [],
}
export const labels: Record<string, string> = {
  pending: '待审',
  approved: '通过',
  rejected: '拒绝',
  visible: '可见',
  hidden: '隐藏',
  offline: '下架',
  deleted: '删除',
  future: '尚未发生',
  in_progress: '进行中',
  partial: '部分周期',
  normal: '正常',
}

export function coverageReasons(report: StatisticsReport, view: StatisticsView) {
  const coverage = report.coverage
  const reasons: string[] = []
  if (['overview', 'content', 'schools'].includes(view) && !coverage.content)
    reasons.push('内容历史覆盖未确认，数量为已记录数据。')
  if (['overview', 'registration', 'schools'].includes(view)) {
    if (!coverage.registration) reasons.push('注册时间历史覆盖未确认，新增为已验证时间的记录。')
    if (!coverage.registration_assignment || !coverage.registration_base)
      reasons.push('注册归属或累计基数尚未完整确认。')
  }
  if (['overview', 'activity', 'schools'].includes(view) && !coverage.activity)
    reasons.push('活跃采集未覆盖、延迟或存在缺口。')
  return reasons
}
