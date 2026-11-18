import { useState } from 'react'
import { Button, Checkbox, Radio, Space } from 'antd'
import type { Bucket, Metrics } from './api'
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
export const formatValue = (value: number | null | undefined) =>
  value == null ? '—' : value.toLocaleString('zh-CN')
export function shanghai(value: string) {
  return new Intl.DateTimeFormat('zh-CN', {
    timeZone: 'Asia/Shanghai',
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  }).format(new Date(value))
}
const colors = ['#1668dc', '#d46b08', '#389e0d']
export function Trends({
  title,
  buckets,
  keys,
  previous = [],
  onSelect,
}: {
  title: string
  buckets: Bucket[]
  keys: (keyof Metrics)[]
  previous?: Bucket[]
  onSelect: (date: string) => void
}) {
  const [bars, setBars] = useState(false)
  const [hidden, setHidden] = useState<string[]>([])
  const [focus, setFocus] = useState<number>()
  const active = keys.filter((key) => !hidden.includes(key))
  const values = [...buckets, ...previous].flatMap((b) => active.map((key) => b.metrics[key] ?? 0))
  const max = Math.max(1, ...values)
  const x = (i: number) => 65 + ((i + 0.5) / Math.max(1, buckets.length)) * 900
  const y = (value: number) => 240 - (value / max) * 210
  const lines = (rows: Bucket[], key: keyof Metrics) => {
    const parts: string[] = []
    let open = false
    rows.slice(0, buckets.length).forEach((b, i) => {
      const n = b.metrics[key]
      if (n == null) {
        open = false
        return
      }
      parts.push(`${open ? 'L' : 'M'}${x(i)},${y(n)}`)
      open = true
    })
    return parts.join(' ')
  }
  const selected = focus === undefined ? undefined : buckets[focus]
  return (
    <section aria-label={title} style={{ marginTop: 24 }}>
      <h3>{title}</h3>
      <Space wrap>
        <Radio.Group
          value={bars}
          onChange={(e) => setBars(e.target.value)}
          options={[
            { label: '折线图', value: false },
            { label: '分组柱状图', value: true },
          ]}
        />
        {keys.map((key, i) => (
          <Checkbox
            key={key}
            checked={!hidden.includes(key)}
            onChange={(e) =>
              setHidden(e.target.checked ? hidden.filter((x) => x !== key) : [...hidden, key])
            }
          >
            <span style={{ color: colors[i] }}>{metricNames[key]}</span>
          </Checkbox>
        ))}
      </Space>
      <svg
        viewBox="0 0 1000 280"
        role="img"
        aria-label={`${title}，横轴为上海时间，纵轴为人数或数量；可通过下方明细表读取数据。`}
        style={{ width: '100%', minHeight: 200 }}
      >
        {[0, 1, 2, 3].map((i) => {
          const value = Math.ceil((max * i) / 3)
          return (
            <g key={i}>
              <line x1={65} x2={965} y1={y(value)} y2={y(value)} stroke="#e8e8e8" />
              <text x={58} y={y(value) + 4} textAnchor="end" fontSize={12}>
                {value}
              </text>
            </g>
          )
        })}
        {active.map((key) => {
          const color = colors[keys.indexOf(key)]
          return (
            <g key={key}>
              {bars ? (
                buckets.map((b, i) =>
                  b.metrics[key] == null ? null : (
                    <rect
                      key={i}
                      x={
                        x(i) -
                        900 / buckets.length / 3 +
                        (active.indexOf(key) * 900) / buckets.length / 3 / active.length
                      }
                      y={y(b.metrics[key]!)}
                      width={Math.max(0.5, 900 / buckets.length / 3 / active.length)}
                      height={240 - y(b.metrics[key]!)}
                      fill={color}
                    />
                  ),
                )
              ) : (
                <path d={lines(buckets, key)} stroke={color} strokeWidth={2} fill="none" />
              )}
              <path d={lines(previous, key)} stroke={color} strokeDasharray="5 4" fill="none" />
            </g>
          )
        })}
        {buckets.map((b, i) => (
          <g
            key={b.key}
            tabIndex={0}
            role="button"
            aria-label={`${shanghai(b.start_at)}，${active.map((key) => `${metricNames[key]} ${formatValue(b.metrics[key])}`).join('，')}，${b.state}`}
            onFocus={() => setFocus(i)}
            onMouseEnter={() => setFocus(i)}
            onClick={() => onSelect(b.start_at)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') onSelect(b.start_at)
            }}
          >
            <rect
              x={x(i) - 450 / buckets.length}
              y={20}
              width={900 / buckets.length}
              height={225}
              fill="transparent"
            />
            {active.map((key) =>
              b.metrics[key] == null ? null : (
                <circle
                  key={key}
                  cx={x(i)}
                  cy={y(b.metrics[key]!)}
                  r={buckets.length > 50 ? 1 : 3}
                  fill={colors[keys.indexOf(key)]}
                >
                  <title>
                    {metricNames[key]}：{formatValue(b.metrics[key])}
                  </title>
                </circle>
              ),
            )}
            {(i === 0 ||
              i === buckets.length - 1 ||
              i % Math.max(1, Math.ceil(buckets.length / 7)) === 0) && (
              <text x={x(i)} y={268} fontSize={11} textAnchor="middle">
                {shanghai(b.start_at).split(' ')[0]}
              </text>
            )}
          </g>
        ))}
      </svg>
      {selected && (
        <div role="status">
          {shanghai(selected.start_at)} 至 {shanghai(selected.end_at)} ·{' '}
          {selected.state === 'future'
            ? '尚未发生'
            : selected.state === 'in_progress'
              ? '进行中'
              : selected.state === 'partial'
                ? '部分周期'
                : '完整周期'}{' '}
          ·{' '}
          {active
            .map((key) => `${metricNames[key]}：${formatValue(selected.metrics[key])}`)
            .join('；')}{' '}
          {selected.coverage.reasons.join('；')}{' '}
          <Button size="small" onClick={() => onSelect(selected.start_at)}>
            查看该周期
          </Button>
        </div>
      )}
      {previous.length > 0 && (
        <p>虚线为所选对比期，按周期内序号对齐；空缺表示不可用或没有对应位置。</p>
      )}
    </section>
  )
}
