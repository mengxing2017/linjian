import {
  createOperationsClient,
  createServiceOperationsClient,
} from '@linjian/api-contract/operations'
import { client } from '../../../app/api'
export const operationsApi = createOperationsClient(client)
export const serviceApi = createServiceOperationsClient(client)
