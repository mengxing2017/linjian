import { ImagePicker } from '../../../shared/image-picker'
import { product } from './api'
export function MediaPicker({ school, value, onChange, label, onBusyChange, context = 'resource.school.write' }: { context?: string; school?: string; value: string | null; onChange: (id: string | null) => void; label: string; onBusyChange?: (busy: boolean) => void }) {
  return <ImagePicker key={school ?? 'platform'} scope={school ?? 'platform'} value={value} label={label} onChange={onChange} onBusyChange={onBusyChange}
    load={page => product.resources.artwork(school, page, context)}
    upload={async file => product.resources.upload(school, await file.arrayBuffer(), context)}
    source={id => `${school ? `/api/v1/schools/${school}` : '/api/v1/platform'}/resource-artwork/${id}${school ? '?context='+encodeURIComponent(context) : ''}`} />
}
