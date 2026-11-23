import { afterEach, expect, it } from 'vitest'
import { cleanup, render, screen } from '@testing-library/react'
import { Input, Select, Space, Switch, type SwitchProps } from 'antd'
import { readFileSync } from 'node:fs'
import { URL as NodeURL } from 'node:url'

const styles = readFileSync(new NodeURL('./styles.css', import.meta.url), 'utf8')

let stylesheet: HTMLStyleElement | undefined

afterEach(() => {
  cleanup()
  stylesheet?.remove()
  stylesheet = undefined
})

function applyStyles() {
  stylesheet = document.createElement('style')
  stylesheet.textContent = styles
  document.head.append(stylesheet)
}

const variants: [string, SwitchProps][] = [
  ['off', { checked: false }],
  ['on', { checked: true }],
  ['text off', { checked: false, checkedChildren: '正常', unCheckedChildren: '隐藏' }],
  ['text on', { checked: true, checkedChildren: '显示', unCheckedChildren: '隐藏' }],
  ['small', { size: 'small' }],
  ['disabled', { disabled: true }],
  ['loading', { loading: true }],
]

it.each(['form-grid', 'form-pair'])('keeps %s switches out of cross-axis stretching without changing text fields', layout => {
  render(<div className={layout}>
    {variants.map(([name, props]) => <label key={name}>{name}<Switch {...props} aria-label={name} /></label>)}
    <label>标题<Input aria-label="标题" /></label>
    <label>分类<Select aria-label="分类" options={[]} /></label>
  </div>)
  applyStyles()

  for (const [name] of variants) {
    const control = screen.getByRole('switch', { name })
    expect(getComputedStyle(control).alignSelf).toBe('flex-start')
  }
  expect(getComputedStyle(screen.getByRole('textbox', { name: '标题' })).alignSelf).not.toBe('flex-start')
  const select = screen.getByRole('combobox', { name: '分类' }).closest('.ant-select')!
  expect(getComputedStyle(select).width).toBe('100%')
  expect(getComputedStyle(select).alignSelf).not.toBe('flex-start')
})

it('leaves table, horizontal Space and ordinary inline switches outside the form override', () => {
  render(<>
    <table><tbody><tr><td><Switch aria-label="本校启用" /></td></tr></tbody></table>
    <Space wrap><Switch aria-label="显示最新菜单" /></Space>
    <label>显示／隐藏<Switch aria-label="客户入口" checkedChildren="显示" unCheckedChildren="隐藏" /></label>
  </>)
  applyStyles()

  for (const control of screen.getAllByRole('switch')) {
    expect(getComputedStyle(control).alignSelf).not.toBe('flex-start')
  }
})
