import { Form, InputNumber } from 'antd'

export function CoordinatesField() {
  return (
    <div className="coordinate-field">
      <Form.Item noStyle name="latitude" rules={[{ required: true, message: '请填写纬度' }]}>
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
      <Form.Item noStyle name="longitude" rules={[{ required: true, message: '请填写经度' }]}>
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
