import type { ReactNode } from 'react'
import { Alert } from 'antd'

export type FormFeedbackState =
  | { type: 'success'; message: string }
  | { type: 'error'; message: string }
  | { type: 'warning'; message: string }
  | { type: 'info'; message: string }
  | null

export function FormFeedback({
  feedback,
  action,
}: {
  feedback: FormFeedbackState
  action?: ReactNode
}) {
  if (!feedback) return null
  return (
    <Alert
      showIcon
      type={feedback.type}
      role={feedback.type === 'error' ? 'alert' : 'status'}
      title={feedback.message}
      action={action}
    />
  )
}
