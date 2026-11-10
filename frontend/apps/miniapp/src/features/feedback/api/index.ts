import { createOperationsClient } from '@linjian/api-contract/operations'
import { client } from '../../identity/api'
export const feedbackApi = createOperationsClient(client)
export type { FeedbackType } from '@linjian/api-contract/operations'
