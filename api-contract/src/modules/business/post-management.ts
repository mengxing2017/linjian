import { requestDocument, requestResource } from "../../http/browser-client";
import { requestMultipart } from "../../http/multipart";
import { decodeResource, invalidDocument, type PageMeta } from "../../http/protocol";
import type { PostReview, Row } from "./types";
export type PostVisibility = "visible" | "hidden" | "offline";
export interface PostManagementView extends PostReview {
  created_via: "miniapp" | "web" | "seed" | "legacy";
  author_kind: "ordinary" | "simulated";
  seed_batch_id: string | null;
  created_by: string | null;
  display_published_at: string;
  source_summary: Pick<
    RecommendationView,
    "id" | "state" | "created_at"
  > | null;
  visibility: PostVisibility;
  is_top: boolean;
  cover_image: string | null;
}
export type PostContentInput = import("../community/post-content").PostContent & { revision?: number };
export interface PinPostInput {
  revision: number;
  is_top: boolean;
}
export interface SetPostStatusInput {
  revision: number;
  visibility: PostVisibility;
  reason: string;
}
export type CreatePostInput = import("../community/post-content").CreatePostInput;
export type UpdatePostInput = import("../community/post-content").UpdatePostInput;
export type PostManagementFilters = PostManagementFilter;
export type ReorderPostsInput = PostOrderInput;
export type PostImportResult = PostImportPreview;
export interface PostManagementFilter {
  page?: number;
  page_size?: number;
  search?: string;
  audit_status?: "" | PostReview["audit_status"];
  visibility?: "" | PostVisibility;
  is_top?: boolean;
  tag?: string;
  created_via?: string; author_kind?: string; seed_batch_id?: string; display_from?: string; display_to?: string;
  created_from?: string;
  created_to?: string;
}
export interface PostManagementPage {
  items: PostManagementView[];
  page?: PageMeta;
  order_revision: number;
  can_recommend?: boolean;
}
export interface PostOrderInput {
  order_revision: number;
  is_top: boolean;
  page: number;
  page_size: number;
  original: { id: string; revision: number }[];
  ordered_ids: string[];
}
export interface PostImportPreview {
  id: string;
  school_id: string;
  actor_id: string;
  revision: number;
  digest: string;
  rows: { row: number; post: PostContentInput }[];
  row_labels: string[];
  errors: { row: number; message: string }[];
  state: "preview" | "committed" | "expired";
  result: string[];
  expires_at: string;
}
export interface CommitPostImport {
  revision: number;
  digest: string;
  idempotency_key: string;
}
const part = encodeURIComponent;
const path = (school: string, tail = "") =>
  `/api/v1/schools/${part(school)}/post-management${tail}`;
const imports = (school: string, tail = "") =>
  `/api/v1/schools/${part(school)}/post-imports${tail}`;
export interface RecommendPostInput {
  revision: number;
  content_revision: number;
  publication_snapshot_id: string;
  reason: string;
  idempotency_key: string;
}
export interface RecommendationView extends Row {
  id: string;
  school_id: string;
  post_id: string;
  recommendation_id: string | null;
  trigger_type: "review_publication" | "manual_recommendation";
  state: "preparing" | "processing" | "accepted" | "blocked" | "cancelled";
  created_at: string;
  accepted_at: string | null;
  actor_name: string | null;
  reason: string | null;
  terminated_reason: string | null;
}
export interface RecommendationPreview {
  id: string;
  school_id: string;
  school_name: string;
  title: string;
  display_label: string;
  summary: string | null;
  revision: number;
  content_revision: number;
  publication_snapshot_id: string | null;
  authorized: boolean;
  can_recommend: boolean;
  unavailable_reason: string | null;
  next_eligible_at: string | null;
  latest: RecommendationView | null;
}
export const postManagement = {
  recommendationPreview: (school: string, id: string, signal?: AbortSignal) =>
    requestResource<RecommendationPreview>(
      path(school, `/${part(id)}/recommendation-preview`),
      "GET",
      undefined,
      signal,
    ),
  recommend: (school: string, id: string, input: RecommendPostInput) =>
    requestResource<RecommendationView>(
      path(school, `/${part(id)}/recommendations`),
      "POST",
      input,
    ),
  recommendations: async (
    school: string,
    id: string,
    page = 1,
    signal?: AbortSignal,
  ) => {
    const doc = await requestDocument(
      path(
        school,
        `/${part(id)}/recommendations?page[number]=${page}&page[size]=10`,
      ),
      "GET",
      undefined,
      signal,
    );
    if (!Array.isArray(doc.data)) return invalidDocument();
    return {
      items: doc.data.map(decodeResource) as RecommendationView[],
      page: doc.meta?.page,
    };
  },
  recommendation: (
    school: string,
    id: string,
    record: string,
    signal?: AbortSignal,
  ) =>
    requestResource<RecommendationView>(
      path(school, `/${part(id)}/recommendations/${part(record)}`),
      "GET",
      undefined,
      signal,
    ),
  async list(
    school: string,
    filter: PostManagementFilter = {},
    signal?: AbortSignal,
  ): Promise<PostManagementPage> {
    const params = new URLSearchParams({
      "page[number]": String(filter.page ?? 1),
      "page[size]": String(filter.page_size ?? 30),
    });
    for (const key of [
      "search",
      "audit_status",
      "visibility",
      "is_top",
      "tag",
      "created_via", "author_kind", "seed_batch_id", "display_from", "display_to",
      "created_from",
      "created_to",
    ] as const) {
      if (filter[key] !== undefined && filter[key] !== "")
        params.set(key, String(filter[key]));
    }
    const doc = await requestDocument(
      `${path(school)}?${params}`,
      "GET",
      undefined,
      signal,
    );
    if (
      !Array.isArray(doc.data) ||
      !Number.isSafeInteger(doc.meta?.order_revision) ||
      Number(doc.meta?.order_revision) < 1
    )
      return invalidDocument();
    return {
      items: doc.data.map(decodeResource) as PostManagementView[],
      page: doc.meta?.page,
      order_revision: Number(doc.meta?.order_revision),
      can_recommend: doc.meta?.can_recommend === true,
    };
  },
  detail: (school: string, id: string, signal?: AbortSignal) =>
    requestResource<PostManagementView>(
      path(school, `/${part(id)}`),
      "GET",
      undefined,
      signal,
    ),
  create: (school: string, input: CreatePostInput) =>
    requestResource<PostManagementView>(path(school), "POST", input),
  update: (school: string, id: string, input: UpdatePostInput) =>
    requestResource<PostManagementView>(
      path(school, `/${part(id)}`),
      "PUT",
      input,
    ),
  pin: (school: string, id: string, revision: number, is_top: boolean) =>
    requestResource<PostManagementView>(
      path(school, `/${part(id)}/pin`),
      "PUT",
      { revision, is_top },
    ),
  status: (
    school: string,
    id: string,
    revision: number,
    visibility: PostVisibility,
    reason = "",
  ) =>
    requestResource<PostManagementView>(
      path(school, `/${part(id)}/status`),
      "PUT",
      { revision, visibility, reason },
    ),
  delete: (school: string, id: string, revision: number) =>
    requestResource<null>(
      path(school, `/${part(id)}`),
      "DELETE",
      undefined,
      undefined,
      { "If-Match": `"${revision}"` },
    ),
  reorder: (school: string, input: PostOrderInput) =>
    requestResource<{ id: string; order_revision: number }>(
      path(school, "/order-changes"),
      "POST",
      input,
    ),
  preview: (school: string, file: File, signal?: AbortSignal) =>
    requestMultipart<PostImportPreview>(imports(school), file, signal),
  importDetail: (school: string, id: string) =>
    requestResource<PostImportPreview>(imports(school, `/${part(id)}`)),
  commit: (school: string, id: string, input: CommitPostImport) =>
    requestResource<PostImportPreview>(
      imports(school, `/${part(id)}/commit`),
      "POST",
      input,
    ),
};
