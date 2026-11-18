import { StrictMode } from 'react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { cleanup, render } from '@testing-library/react'
import EChartsChart from './echarts-chart'
import { categoryModel } from './chart-model'

const mocks = vi.hoisted(() => ({ init: vi.fn(), option: vi.fn(() => ({})) }))
vi.mock('./echarts-runtime', () => ({ init: mocks.init, chartOption: mocks.option }))
let width = 0
let resize: () => void
let frames: Map<number, FrameRequestCallback>
let frameId = 0
const disconnected = vi.fn()
beforeEach(() => {
  frames = new Map()
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockImplementation(() => width)
  vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockReturnValue(320)
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
    frames.set(++frameId, callback)
    return frameId
  })
  vi.stubGlobal('cancelAnimationFrame', (id: number) => frames.delete(id))
  vi.stubGlobal(
    'ResizeObserver',
    class {
      constructor(callback: () => void) {
        resize = callback
      }
      observe() {}
      disconnect() {
        disconnected()
      }
    },
  )
})
afterEach(() => {
  cleanup()
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
  vi.clearAllMocks()
})
function flush() {
  for (const [id, callback] of frames) {
    frames.delete(id)
    callback(0)
  }
}
function fakeChart() {
  return {
    on: vi.fn(),
    setOption: vi.fn(),
    resize: vi.fn(),
    dispose: vi.fn(),
    getOption: vi.fn(() => ({ dataZoom: [{ start: 50, end: 90 }] })),
  }
}

it('waits for positive dimensions, resizes once per frame and disposes observers and the instance', () => {
  width = 0
  const chart = fakeChart()
  mocks.init.mockReturnValue(chart)
  const { unmount } = render(
    <EChartsChart model={categoryModel('分类', [{ id: 'a', name: '甲校', value: 0 }], '人', '')} />,
  )
  flush()
  expect(mocks.init).not.toHaveBeenCalled()
  width = 800
  resize()
  resize()
  flush()
  expect(mocks.init).toHaveBeenCalledTimes(1)
  resize()
  resize()
  flush()
  expect(chart.resize).toHaveBeenCalledTimes(1)
  unmount()
  expect(chart.dispose).toHaveBeenCalledTimes(1)
  expect(disconnected).toHaveBeenCalled()
  expect(frames.size).toBe(0)
})

it('replaces old-school series and uses the latest click target under StrictMode', () => {
  width = 800
  const chart = fakeChart()
  mocks.init.mockReturnValue(chart)
  const onSelect = vi.fn()
  const model = (id: string) => categoryModel('学校', [{ id, name: id, value: 2 }], '人', '')
  const { rerender } = render(
    <StrictMode>
      <EChartsChart model={model('a')} onSelect={onSelect} />
    </StrictMode>,
  )
  flush()
  rerender(
    <StrictMode>
      <EChartsChart model={model('b')} onSelect={onSelect} />
    </StrictMode>,
  )
  expect(chart.setOption).toHaveBeenLastCalledWith({}, { notMerge: true })
  const click = chart.on.mock.calls[0][1] as (event: { dataIndex: number }) => void
  click({ dataIndex: 0 })
  expect(onSelect).toHaveBeenCalledWith('b')
  expect(onSelect).not.toHaveBeenCalledWith('a')
})

it('retains zoom when updating the same time range and resets it for a new range', () => {
  width = 800
  const chart = fakeChart()
  mocks.init.mockReturnValue(chart)
  const first = {
    ...categoryModel('趋势', [{ id: 'a', name: '本期', value: 2 }], '人', ''),
    zoom: true,
  }
  const { rerender } = render(<EChartsChart model={first} />)
  flush()
  const updated = { ...first, bars: false }
  rerender(<EChartsChart model={updated} />)
  expect(mocks.option).toHaveBeenLastCalledWith(updated, { start: 50, end: 90 })
  const nextRange = { ...updated, categoryIds: ['b'] }
  rerender(<EChartsChart model={nextRange} />)
  expect(mocks.option).toHaveBeenLastCalledWith(nextRange, undefined)
})
