import { useEffect, useRef, useState } from 'react'
import { chartOption, init } from './echarts-runtime'
import type { ChartModel } from './chart-model'

export default function EChartsChart({
  model,
  onSelect,
}: {
  model: ChartModel
  onSelect?: (id: string) => void
}) {
  const element = useRef<HTMLDivElement>(null)
  const instance = useRef<ReturnType<typeof init> | undefined>(undefined)
  const lastRange = useRef('')
  const latest = useRef({ model, onSelect })
  latest.current = { model, onSelect }
  const [error, setError] = useState<Error>()
  useEffect(() => {
    const node = element.current
    if (!node) return
    let frame = 0
    const resize = () => {
      cancelAnimationFrame(frame)
      frame = requestAnimationFrame(() => {
        if (!node.clientWidth || !node.clientHeight) return
        try {
          if (!instance.current) {
            const chart = init(node, undefined, { renderer: 'svg' })
            instance.current = chart
            chart.on('click', (event) => {
              const index = event.dataIndex
              if (typeof index !== 'number') return
              const id = latest.current.model.categoryIds[index]
              // Comparison-only tail buckets have no current-period drill target.
              if (id) latest.current.onSelect?.(id)
            })
            chart.setOption(chartOption(latest.current.model), { notMerge: true })
            lastRange.current = JSON.stringify(latest.current.model.categoryIds)
          } else instance.current.resize()
        } catch (cause) {
          setError(cause instanceof Error ? cause : new Error('图表渲染失败'))
        }
      })
    }
    const observer = new ResizeObserver(resize)
    observer.observe(node)
    resize()
    return () => {
      cancelAnimationFrame(frame)
      observer.disconnect()
      instance.current?.dispose()
      instance.current = undefined
    }
  }, [])
  useEffect(() => {
    try {
      const chart = instance.current
      if (!chart) return
      const range = JSON.stringify(model.categoryIds)
      const zoom =
        model.zoom && lastRange.current === range
          ? (chart.getOption().dataZoom as { start?: number; end?: number }[] | undefined)?.[0]
          : undefined
      chart.setOption(chartOption(model, zoom), { notMerge: true })
      lastRange.current = range
    } catch (cause) {
      setError(cause instanceof Error ? cause : new Error('图表更新失败'))
    }
  }, [model])
  if (error) throw error
  return (
    <div
      ref={element}
      className="linjian-statistics-chart"
      style={model.height ? { height: model.height } : undefined}
    />
  )
}
