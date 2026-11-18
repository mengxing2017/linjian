import { init, use, type ComposeOption } from 'echarts/core'
import { BarChart, LineChart, type BarSeriesOption, type LineSeriesOption } from 'echarts/charts'
import {
  AriaComponent,
  DataZoomComponent,
  GridComponent,
  TooltipComponent,
  type AriaComponentOption,
  type DataZoomComponentOption,
  type GridComponentOption,
  type TooltipComponentOption,
} from 'echarts/components'
import { SVGRenderer } from 'echarts/renderers'
import { pointText, type ChartModel } from './chart-model'

use([
  LineChart,
  BarChart,
  GridComponent,
  TooltipComponent,
  DataZoomComponent,
  AriaComponent,
  SVGRenderer,
])
export { init }
export type ChartOption = ComposeOption<
  | LineSeriesOption
  | BarSeriesOption
  | GridComponentOption
  | TooltipComponentOption
  | DataZoomComponentOption
  | AriaComponentOption
>

export function chartOption(
  model: ChartModel,
  zoom?: { start?: number; end?: number },
): ChartOption {
  const numericAxis = {
    type: 'value' as const,
    min: model.percent ? (extent: { min: number }) => Math.min(0, extent.min) : 0,
    max: model.percent ? (extent: { max: number }) => Math.max(100, extent.max) : undefined,
    minInterval: model.percent ? undefined : 1,
    axisLabel: { formatter: model.percent ? '{value}%' : '{value}' },
    splitLine: { lineStyle: { color: '#f0f0f0' } },
  }
  const categoryAxis = {
    type: 'category' as const,
    data: model.categories.map((name, i) =>
      model.horizontal && model.series.every((s) => s.data[i] == null)
        ? `${name}（暂无数据）`
        : name,
    ),
    inverse: !!model.horizontal,
    boundaryGap: !!model.bars,
    axisLabel: model.horizontal
      ? { interval: 0, width: 100, overflow: 'truncate' as const }
      : {
          hideOverlap: true,
          formatter: (value: string) => value.replace(/^\d{4}\//, '').replace(/（.*$/, ''),
        },
  }
  return {
    animation: false,
    aria: {
      enabled: true,
      label: {
        description: `${model.title}。${model.description}。可通过查看图表数据读取各项数值。`,
      },
    },
    grid: {
      left: model.horizontal ? 125 : 55,
      right: model.horizontal ? 85 : 24,
      top: 20,
      bottom: model.zoom ? 85 : 45,
    },
    tooltip: {
      trigger: 'axis',
      renderMode: 'richText',
      confine: true,
      axisPointer: { type: model.bars ? 'shadow' : 'line' },
      formatter: (params) => {
        const item = Array.isArray(params) ? params[0] : params
        const index = item?.dataIndex
        if (typeof index !== 'number') return ''
        return [
          model.categories[index],
          ...model.series.map((s) => `${s.name}：${pointText(s, index)}\n${s.notes[index] ?? ''}`),
        ].join('\n')
      },
    },
    xAxis: model.horizontal ? numericAxis : categoryAxis,
    yAxis: model.horizontal ? categoryAxis : numericAxis,
    dataZoom: model.zoom
      ? [
          {
            type: 'slider',
            xAxisIndex: 0,
            bottom: 8,
            start: zoom?.start ?? Math.max(0, 100 - 6000 / model.categories.length),
            end: zoom?.end ?? 100,
            filterMode: 'filter',
          },
          {
            type: 'inside',
            xAxisIndex: 0,
            filterMode: 'filter',
            zoomOnMouseWheel: false,
            moveOnMouseWheel: false,
          },
        ]
      : [],
    series: model.series.map((s) => ({
      id: s.id,
      name: s.name,
      type: model.bars ? 'bar' : 'line',
      data: s.data,
      itemStyle: {
        color: s.color,
        opacity: s.dashed ? 0.55 : 1,
        borderColor: s.color,
        borderWidth: s.dashed ? 1 : 0,
        borderType: s.dashed ? 'dashed' : 'solid',
      },
      lineStyle: { color: s.color, type: s.dashed ? 'dashed' : 'solid', width: 2 },
      connectNulls: false,
      smooth: false,
      showSymbol: model.categories.length <= 60,
      symbol: s.dashed ? 'emptyCircle' : 'circle',
      symbolSize: 6,
      barMaxWidth: 32,
      label: {
        show: !!model.horizontal,
        position: 'right',
        formatter: (p) => pointText(s, p.dataIndex),
      },
    })),
  }
}
