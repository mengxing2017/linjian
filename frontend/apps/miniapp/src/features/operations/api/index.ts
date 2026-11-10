import { createOperationsClient } from '@linjian/api-contract/operations'
import { client } from '../../identity/api'
export const operationsApi = createOperationsClient(client)
export type { Display, OperationResource, FeedbackType } from '@linjian/api-contract/operations'
