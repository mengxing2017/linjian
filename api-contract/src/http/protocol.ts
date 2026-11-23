/** Shared JSON:API transport, resource decoding and error handling. */
export interface ApiFailure {
  errors: Array<{
    status?: string;
    code?: string;
    title?: string;
    detail?: string;
    meta?: Record<string, unknown>;
  }>;
}

export function failureDetails(value: unknown): {
  code: string;
  message: string;
  meta?: Record<string, unknown>;
} {
  if (
    typeof value === "object" &&
    value !== null &&
    "errors" in value &&
    Array.isArray(value.errors)
  ) {
    const error: unknown = value.errors[0];
    if (typeof error === "object" && error !== null) {
      return {
        meta:
          "meta" in error &&
          typeof error.meta === "object" &&
          error.meta !== null
            ? (error.meta as Record<string, unknown>)
            : undefined,
        code:
          "code" in error && typeof error.code === "string"
            ? error.code
            : "http_error",
        message:
          "detail" in error && typeof error.detail === "string"
            ? error.detail
            : "title" in error && typeof error.title === "string"
              ? error.title
              : "服务暂时不可用",
      };
    }
  }
  return { code: "http_error", message: "服务暂时不可用" };
}

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly code: string,
    message: string,
    public readonly meta?: Record<string, unknown>,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

export interface ResourceObject {
  type: string;
  id: string;
  attributes: Record<string, unknown>;
}
export interface PageMeta {
  number: number;
  size: number;
  has_more: boolean;
}
export interface Collection<T> {
  items: T[];
  page?: PageMeta;
}
export interface ResourceDocument {
  data: ResourceObject | ResourceObject[] | null;
  meta?: { page?: PageMeta; [key: string]: unknown };
}
export function object(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
export function invalidDocument(): never {
  throw new ApiError(
    200,
    "invalid_document",
    "服务返回了不合法的 JSON:API 文档",
  );
}
export function decodeResource(value: unknown): Record<string, unknown> {
  if (
    !object(value) ||
    typeof value.type !== "string" ||
    !value.type ||
    typeof value.id !== "string" ||
    !value.id ||
    !object(value.attributes) ||
    "id" in value.attributes ||
    "type" in value.attributes
  )
    return invalidDocument();
  return { ...value.attributes, id: value.id };
}
/** Explicit aliases for action routes whose target resource differs from the URL leaf. */
export function resourceType(path: string): string {
  const changePath = path.split('?')[0]
  if (/^\/api\/v1\/(?:miniapp|platform)\/school-change-requests(?:\/|$)/.test(changePath)) {
    if (changePath.endsWith('/cancel')) return 'miniapp-school-change-cancellations'
    if (changePath.endsWith('/approve')) return 'miniapp-school-change-approvals'
    if (changePath.endsWith('/reject')) return 'miniapp-school-change-rejections'
    return 'miniapp-school-change-requests'
  }
  const parts = path.split("?")[0].split("/").filter(Boolean);
  if (parts[1] === 'v2' && parts[2] === 'source-connections') return 'source-connections';
  if (parts[1] === 'v2' && parts[2] === 'schools' && parts[4] === 'source-bindings') return 'source-bindings';
  if (parts[0] !== "api" || parts[1] !== "v1")
    throw new Error("Unknown resource path");
  if(parts[2]==='platform' && parts[3]==='site-settings')return 'site-settings';
  if(parts[2]==='platform' && parts[3]==='miniapp-registration-settings')return 'miniapp-registration-settings';
  if(parts[2]==='schools' && parts[4]==='source-outbox' && parts[6]==='terminations')return 'source-event-terminations';
  if(parts[2]==='schools' && parts[4]==='members' && parts[6]==='recommendation-capability')return 'post-recommendation-capabilities';
  if(parts[2]==='schools' && parts[4]==='members' && parts[6]==='authorization')return 'school-member-authorizations';
  if(parts[2]==='schools' && parts[4]==='members' && parts[6]==='authorization-actions')return 'school-member-authorization-actions';
  if(parts[2]==='schools' && parts[4]==='post-management' && parts[6]==='recommendations')return 'post-recommendations';
  if(parts[2]==='schools' && parts[4]==='source-outbox' && parts[6]==='preparation-retries')return 'source-preparation-retries';
  if(parts[2]==='schools' && parts[4]==='source-outbox' && parts[6]==='replays')return 'source-outbox-replays';
  if(parts[2]==='schools' && (parts[4]==='post-reviews' || (parts[4]==='posts' && parts[6]==='review')))return 'post-reviews';
  if(parts[2]==='schools' && parts[4]==='post-management')return parts[5]==='order-changes'?'post-order-changes':'post-management';
  if(parts[2]==='schools' && parts[4]==='post-seed-batches')return parts[6]==='items'?'post-seed-items':'post-seed-batches';
  if(parts[2]==='schools' && parts[4]==='post-seed-authors')return 'post-seed-authors';
  if(parts[2]==='schools' && parts[4]==='post-seed-avatars')return 'post-seed-avatars';
  if(parts[2]==='schools' && parts[4]==='post-imports')return 'post-imports';
  if (parts[2] === 'schools' && parts[4] === 'home-menu-tags' && parts[5] === 'order') return 'home-menu-order';
  const kind=parts[2]==='platform'?parts[3]:parts[2]==='schools'?parts[4]:undefined;
  if(kind==='operation-service-targets' && ['submit','approve','reject','withdraw'].includes(parts[5]))return 'operation-target-reviews';
  if(kind==='home-shortcut-overrides')return 'home-shortcut-overrides';
  if(kind==='home-shortcuts' && parts[parts[2]==='platform'?4:5]==='preview')return 'home-shortcut-previews';
  if(kind==='home-shortcuts' && ['order','batch-status'].includes(parts[5]))return 'shortcut-batches';
  if(kind==='operation-services')return parts[6]==='resolve' ? 'operation-service-resolutions' : 'operation-services';
  if(kind==='marketing'||kind==='marketing-settings')return 'school-marketing';
  if(kind==='marketing-events')return 'marketing-events';
  if(kind && ['group-qrcodes','home-banners','customer-config','feedback','home-shortcuts','operation-service-categories','operation-service-targets','operation-partners','operation-cooperations','marketing-slots','operation-events','operation-action-events','operation-reports'].includes(kind))return kind;
  if(kind==='blocked-words')return 'blocked-words';
  if(kind==='blocked-word-imports')return 'blocked-word-imports';
  if(kind && ['resource-previews','resource-publications','resource-qrcodes','source-outbox','home-menu-tags','home-menu-settings','home-menu-imports'].includes(kind))return kind;
  if (parts[2] === "schools" && parts[4] === "published-resources") return "resources";
  if(parts[2]==='miniapp') {
    if(parts[3]==='activity-events') return 'miniapp-activity-events';
    if(parts[3]==='profile' && parts[4]==='mobile') return 'miniapp-user-mobiles';
    if(parts[3]==='profile' && ['phone-binding','phone-bindings'].includes(parts[4])) return 'miniapp-phone-bindings';
    if(parts[3]==='profile') return 'miniapp-profiles';
    if(parts[3]==='avatar-assets') return 'miniapp-avatar-assets';
  }
  const userKind=parts[2]==='platform'?parts[3]:parts[2]==='schools'?parts[4]:undefined;
  if(userKind==='miniapp-users') {
    const action=parts[2]==='platform'?parts[5]:parts[6];
    if((parts[2]==='platform'?parts[4]:parts[5])==='searches')return 'miniapp-user-searches';
    if(action==='status')return 'miniapp-user-statuses';
    if(action==='mobile')return 'miniapp-user-mobiles';
    if(action==='school')return 'miniapp-user-schools';
    return 'miniapp-users';
  }
  if (parts[2] === "auth") return "sessions";

  if (parts[2] === "profile" || parts[2] === "accounts") return "accounts";
  if (parts[2] === "schools") {
    const kind = parts[4] ?? "schools";
    if (kind === "members") return "school-management-members";
    if (kind === "posts" && parts[6] === "comments") return "comments";
    if (
      [
        "schools",
        "posts",
        "comments",
        "cases",
      ].includes(kind)
    )
      return kind;
  }
  throw new Error("Unknown write resource path");
}
export function writeDocument(path: string, body: unknown) {
  if (!object(body)) throw new Error("Resource attributes must be an object");
  const { id, ...attributes } = body;
  if (
    "type" in attributes ||
    (id !== undefined && (typeof id !== "string" || !id))
  )
    throw new Error("Invalid resource identity");
  return {
    data: {
      type: resourceType(path),
      ...(id === undefined ? {} : { id }),
      attributes,
    },
  };
}

export function decodeData<T>(document: ResourceDocument): T {
  return (
    Array.isArray(document.data)
      ? document.data.map(decodeResource)
      : document.data === null
        ? null
        : decodeResource(document.data)
  ) as T;
}
