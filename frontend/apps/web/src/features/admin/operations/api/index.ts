import { client } from '@linjian/api-contract/http'
import { createOperationsClient } from '@linjian/api-contract/operations'
export const operationsApi = createOperationsClient(client)
export type { OperationResource, OperationInput, CustomerConfig, CustomerInput, Feedback, Filters, Module } from '@linjian/api-contract/operations'
export { product, workspace } from '@linjian/api-contract/business'
export type { ResourceAction, ImageRef } from '@linjian/api-contract/business'
