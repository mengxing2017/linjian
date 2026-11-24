import { Col, Row } from 'antd'
import type { StatisticsReport } from '../api'
import { metricNames, metricNote, shanghai } from '../metrics'
import { labels } from '../views'
import { ChartCard } from './chart-card'
import { categoryModel } from './chart-model'

export function OverviewCharts({ report: r }: { report: StatisticsReport }) {
  return (
    <Row gutter={16}>
      {(
        [
          ['本期内容量', ['posts', 'comments']],
          ['新增注册与访问活跃', ['registrations', 'active_users']],
        ] as const
      ).map(([title, keys]) => (
        <Col xs={24} lg={12} key={title}>
          <ChartCard
            model={categoryModel(
              title,
              keys.map((key) => ({
                id: key,
                name:
                  metricNames[key] +
                  (key === 'posts' ? '（帖）' : key === 'comments' ? '（条）' : '（人）'),
                value: r.summary[key],
                note: metricNote(key, r.coverage, r.summary[key]),
              })),
              '',
              `本期摘要，数据截至 ${shanghai(r.as_of)}。各项独立比较，不代表包含、构成或转化关系。详细变化请进入对应统计模块。`,
            )}
          />
        </Col>
      ))}
    </Row>
  )
}

export function ContentStateCharts({ report: r }: { report: StatisticsReport }) {
  return (
    <Row gutter={16}>
      {(['posts', 'comments'] as const).map((kind) => (
        <Col xs={24} lg={12} key={kind}>
          {(['audit', 'visibility'] as const).map((dimension) => (
            <ChartCard
              key={dimension}
              model={categoryModel(
                `${kind === 'posts' ? '帖子' : '评论'}当前${dimension === 'audit' ? '审核' : '展示'}分布`,
                Object.entries(r.current_state[kind]?.[dimension] ?? {}).map(([key, value]) => ({
                  id: key,
                  name: labels[key] ?? key,
                  value,
                })),
                kind === 'posts' ? '帖' : '条',
                `显示本期新增${kind === 'posts' ? '帖子' : '评论'}截至 ${shanghai(r.as_of)} 的审核和展示状态，不统计历史审核次数或展示次数。${!r.coverage.content ? '数据存在缺口，仅展示现有记录。' : ''}`,
              )}
            />
          ))}
          <p>
            当前公开：{r.current_state[kind]?.public ?? '—'}。该数量与上方审核、展示状态可能重复，请勿相加。
          </p>
        </Col>
      ))}
    </Row>
  )
}

export function RegistrationDistribution({ report: r }: { report: StatisticsReport }) {
  const names: Record<string, string> = {
    known_school: '已知学校',
    unassigned: '未归属学校',
    unknown: '学校归属数据缺失',
  }
  return (
    <ChartCard
      model={categoryModel(
        '本期注册归属分布',
        Object.entries(r.registration_groups).map(([key, value]) => ({
          id: key,
          name: names[key] ?? key,
          value,
        })),
        '人',
        `按注册时记录的学校归属统计；未绑定学校计入“未归属学校”，归属数据缺失单独列出，不分摊到学校。${!r.coverage.registration ? '注册数据存在缺口或时间异常，仅展示有效记录。' : ''}数据截至 ${shanghai(r.as_of)}。`,
      )}
    />
  )
}
