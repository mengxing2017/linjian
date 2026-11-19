import {
  codesFromNames,
  emptyRegion,
  namesFromCodes,
  type RegionValue,
} from '@linjian/china-regions'
import { RegionCascader } from './region-cascader'

export function RegionField({
  value,
  onChange,
  id,
  disabled,
}: {
  value?: RegionValue
  onChange?: (value: RegionValue) => void
  id?: string
  disabled?: boolean
}) {
  const region = value ?? emptyRegion
  const codes = codesFromNames(region) ?? []
  return (
    <RegionCascader
      id={id}
      value={codes}
      onChange={(next) => onChange?.(namesFromCodes(next) ?? { ...emptyRegion })}
      disabled={disabled}
    />
  )
}
