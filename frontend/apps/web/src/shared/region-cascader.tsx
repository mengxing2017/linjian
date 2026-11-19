import { Cascader } from 'antd'
import { regionOptions, type RegionNode } from '@linjian/china-regions'
import { useMemo } from 'react'

export interface CascaderOption {
  value: string
  label: string
  children?: CascaderOption[]
}

function toOptions(nodes: RegionNode[]): CascaderOption[] {
  return nodes.map((node) => ({
    value: node.code,
    label: node.name,
    children: node.children ? toOptions(node.children) : undefined,
  }))
}

export function RegionCascader({
  value,
  onChange,
  id,
  ariaLabel,
  placeholder = '请选择省 / 市 / 区',
  disabled,
}: {
  value?: string[]
  onChange?: (codes: string[]) => void
  id?: string
  ariaLabel?: string
  placeholder?: string
  disabled?: boolean
}) {
  const options = useMemo(() => toOptions(regionOptions), [])
  return (
    <Cascader
      id={id}
      aria-label={ariaLabel}
      options={options}
      value={value ?? []}
      onChange={(next) => onChange?.(((next ?? []) as (string | number)[]).map(String))}
      changeOnSelect
      showSearch
      allowClear
      placeholder={placeholder}
      disabled={disabled}
    />
  )
}
