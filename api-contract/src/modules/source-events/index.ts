/** Server-to-server integration types. Credentials and signing stay in the Rust service. */
export interface SourceEvent {
 event_id:string
 schema_version:1
 event_type:'content.upsert'|'content.withdrawn'
 external_organization_id:string
 external_content_id:string
 version:string
 sequence:number
 occurred_at:string
 title:string|null
 content:string|null
 detail_url:string
}
export interface SourceReceipt {
 status:'received'|'applied'|'ignored_stale'|'rejected'
 sending?:{pending:number;sent:number;failed:number;unknown:number}|null
}
