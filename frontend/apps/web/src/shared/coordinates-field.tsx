import { Form, InputNumber } from 'antd'

export function CoordinatesField() {
  return (
    <div className="coordinate-field">
      <Form.Item noStyle name="latitude">
        <InputNumber
          aria-label="纬度"
          placeholder="纬度"
          min={-90}
          max={90}
          step={0.000001}
          precision={6}
          style={{ width: '100%' }}
        />
      </Form.Item>
      <Form.Item noStyle name="longitude">
        <InputNumber
          aria-label="经度"
          placeholder="经度"
          min={-180}
          max={180}
          step={0.000001}
          precision={6}
          style={{ width: '100%' }}
        />
      </Form.Item>
    </div>
  )
}
