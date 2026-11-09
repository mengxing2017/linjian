import {
  requestResource,
  requestDocument,
  requestMultipart,
  decodeResource,
  invalidDocument,
  type PageMeta,
} from "../../http";
import type { PostReview } from "./types";
export type PostVisibility = "visible" | "hidden" | "offline";
export interface PostManagementView extends PostReview {
  visibility: PostVisibility;
  is_top: boolean;
  cover_image: string | null;
}
export interface PostContentInput {
  title: string;
  content: string;
  images: string[];
  tags: string[];
  revision?: number;
}
export interface PinPostInput {
  revision: number;
  is_top: boolean;
}
export interface SetPostStatusInput {
  revision: number;
  visibility: PostVisibility;
  reason: string;
}
export type CreatePostInput = Omit<PostContentInput, "revision">;
export type UpdatePostInput = PostContentInput & { revision: number };
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
  created_from?: string;
  created_to?: string;
}
export interface PostManagementPage {
  items: PostManagementView[];
  page?: PageMeta;
  order_revision: number;
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
export const postManagement = {
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
