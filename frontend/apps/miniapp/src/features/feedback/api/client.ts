import { createOperationsClient } from '@linjian/api-contract/operations'
import { client } from '../../../app/api'
export const feedbackApi = createOperationsClient(client)
