import {
  createOperationsClient,
  createServiceOperationsClient,
} from '@linjian/api-contract/operations'
import { client } from '../../../../app/api'
export const serviceApi = createServiceOperationsClient(client)
export const operationsApi = createOperationsClient(client)
