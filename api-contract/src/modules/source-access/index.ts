import {requestResource,type Cancellation} from '../../http'
export interface SourceConnection {id:string;name:string;origin:string;key_id:string;revision:number;checked_at:string;expires_at:string}
export interface SourceConnectionSettings {configured:boolean;origin:string|null;key_id:string|null;connection:SourceConnection|null}
export interface SourceBinding {id:string;route_id:string;active:boolean;revision:number;checked_at:string|null;updated_at:string}
export interface SchoolRouteSettings {binding:SourceBinding|null;backlog?:Record<string,number>}
export const sourceAccess={
 connections:()=>requestResource<SourceConnectionSettings>('/api/v2/source-connections'),
 verify:()=>requestResource<{id:string;revision:number}>('/api/v2/source-connections','POST',{}),
 bindings:(school:string,signal?:Cancellation)=>requestResource<SchoolRouteSettings>(`/api/v2/schools/${encodeURIComponent(school)}/source-bindings`,'GET',undefined,signal),
 bind:(school:string,v:{route_id:string;active:boolean},revision:number)=>requestResource<{id:string;revision:number}>(`/api/v2/schools/${encodeURIComponent(school)}/source-bindings`,'POST',v,undefined,{'If-Match':`"${revision}"`}),
}

export type {SourceEventV2,SourceReceiptV2,SourceCheckV2,SourceCapabilitiesV2,SourceDocument} from './protocol'
export type SourceDeliveryState='waiting_binding'|'waiting_configuration'|'paused'|'preparing_content'|'ready'|'in_flight'|'retry_wait'|'blocked'|'tracking_receipt'|'settled'|'superseded'
export interface SourceOutboxView {
 id:string;post_id:string;sequence:number;event_type:'content.upsert'|'content.withdrawn';attempts:number
 next_attempt_at:string;receipt_id:string|null;receipt_status:'received'|'applied'|'ignored_stale'|'rejected'|null
 receipt_summary:unknown|null;last_error:string|null;schema_version:number;route_id:string|null;blocked_reason:string|null
 accepted_at:string|null;created_at:string;frozen_at:string|null;binding_version:string|null;superseded_by:string|null
 recovery_reason:string|null;delivery_state:SourceDeliveryState;recoverable:boolean
 preparation?:SourcePreparationView
 publication?:{summary:string;summary_algorithm:string;approved_post_revision:number;link_type:string|null;link:string|null;expires_at:string|null}
 recoveries?:Array<{actor_id:string;reason:string;created_at:string}>
 related_events?:Array<{id:string;route_id:string|null;sequence:number;event_type:string;receipt_status:string|null}>
}

export interface SourcePreparationView {
 state:'pending'|'generating'|'retry_wait'|'blocked'|'completed'|'superseded';revision:number;attempts:number
 next_attempt_at:string;error_code:string|null;recoverable:boolean
}
export interface SourcePreview {
 id:string;post_revision:number;content_revision:number;summary:string;summary_algorithm:'plain-v1';preview_digest:string
 source_projection:'legacy_full_v2'|'miniapp_summary_v1'
}
