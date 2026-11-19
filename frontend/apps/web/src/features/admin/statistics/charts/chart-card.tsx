import { Component, lazy, Suspense, useMemo, useState, type ReactNode } from 'react'
import { Alert, Button, Card, Empty, Skeleton, Table } from 'antd'
import { pointText, type ChartModel } from './chart-model'
import './charts.css'

class ChartBoundary extends Component<
  { children: ReactNode; onRetry: () => void },
  { failed: boolean }
> {
  state = { failed: false }
  static getDerivedStateFromError() {
    return { failed: true }
  }
  render() {
    return this.state.failed ? (
      <Alert
        type="warning"
        title="图表加载失败，仍可查看图表数据或周期明细。"
        action={<Button onClick={this.props.onRetry}>重新加载图表</Button>}
      />
    ) : (
      this.props.children
    )
  }
}

export function ChartCard({
  model,
  onSelect,
  controls,
}: {
  model: ChartModel
  onSelect?: (id: string) => void
  controls?: ReactNode
}) {
  const [attempt, setAttempt] = useState(0)
  const [details, setDetails] = useState(false)
  // Recreate lazy on retry so a rejected module promise is not retained by React.
  const Canvas = useMemo(() => {
    void attempt
    return lazy(() => import('./echarts-chart'))
  }, [attempt])
  const known = model.series.flatMap((s) => s.data).filter((v) => v != null)
  return (
    <Card size="small" className="linjian-statistics-card" title={<h3>{model.title}</h3>}>
      {controls}
      <p className="linjian-statistics-caption">{model.description}</p>
      <figure aria-label={model.title} className="linjian-statistics-figure">
        {known.length ? (
          <>
            <ChartBoundary key={attempt} onRetry={() => setAttempt((n) => n + 1)}>
              <Suspense
                fallback={
                  <div
                    className="linjian-statistics-chart"
                    style={model.height ? { height: model.height } : undefined}
                  >
                    <Skeleton.Node active style={{ width: '100%', height: '100%' }} />
                  </div>
                }
              >
                <Canvas model={model} onSelect={onSelect} />
              </Suspense>
            </ChartBoundary>
            {known.every((v) => v === 0) && (
              <figcaption>所选期间可用指标均为 0。</figcaption>
            )}
          </>
        ) : (
          <div className="linjian-statistics-chart-empty">
            <Empty description={model.emptyMessage ?? '请选择至少一个指标。'} />
          </div>
        )}
      </figure>
      <Button type="link" onClick={() => setDetails(!details)}>
        {details ? '收起图表数据' : '查看图表数据'}
      </Button>
      {details && (
        <Table
          size="small"
          rowKey="index"
          scroll={{ x: true }}
          pagination={{ pageSize: 20, showSizeChanger: false }}
          dataSource={model.categories.map((name, index) => ({ name, index }))}
          columns={[
            {
              title: '时间或分类',
              key: 'name',
              render: (_, row) =>
                onSelect && model.categoryIds[row.index] ? (
                  <Button type="link" onClick={() => onSelect(model.categoryIds[row.index])}>
                    {row.name}
                  </Button>
                ) : (
                  row.name
                ),
            },
            ...model.series.map((s) => ({
              title: s.name,
              key: s.id,
              render: (_: unknown, row: { index: number }) => (
                <span>
                  {pointText(s, row.index)}
                  {s.notes[row.index] && `；${s.notes[row.index]}`}
                </span>
              ),
            })),
          ]}
        />
      )}
    </Card>
  )
}
