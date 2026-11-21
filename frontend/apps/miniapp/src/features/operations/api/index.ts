import { createOperationsClient, createServiceOperationsClient } from '@linjian/api-contract/operations'
import { client } from '../../identity/api'
export const operationsApi = createOperationsClient(client)
export const serviceApi = createServiceOperationsClient(client)
export type { MarketingSlot } from '@linjian/api-contract/operations'
export type { EffectiveService, EffectiveChannel, ServiceCatalog, ActionOutcome, OperationEvent, OperationOutcome } from '@linjian/api-contract/operations'
export type { Display, OperationResource, FeedbackType } from '@linjian/api-contract/operations'
