import { afterEach, expect, it, vi } from "vitest";
import { readFileSync } from "node:fs";
import {
  workspace,
  type PostManagementView,
  type PostImportPreview,
} from "../src/modules/business";
import { setCsrf, writeDocument } from "../src/http";
function fixture(name: string) {
  return JSON.parse(
    readFileSync(new URL(`../fixtures/${name}.json`, import.meta.url), "utf8"),
  );
}
function response(body: unknown) {
  return Response.json(body, {
    headers: { "content-type": "application/vnd.api+json" },
  });
}
afterEach(() => {
  vi.unstubAllGlobals();
  setCsrf("");
});
it.each([
  "post-management",
  "post-management-hidden",
  "post-management-offline",
])(
  "decodes %s and preserves ordering metadata and independent filters",
  async (name) => {
    const item = fixture(name);
    const fetch = vi
      .fn()
      .mockResolvedValue(
        response({
          data: [item.data],
          meta: {
            page: { number: 2, size: 30, has_more: false },
            order_revision: 7,
          },
        }),
      );
    vi.stubGlobal("fetch", fetch);
    const result = await workspace.postManagement.list("school-1", {
      page: 2,
      audit_status: "approved",
      visibility: "hidden",
      is_top: false,
      tag: "校园",
    });
    expect(result.order_revision).toBe(7);
    expect(result.items[0]).toMatchObject(
      item.data.attributes as PostManagementView,
    );
    const url = fetch.mock.calls[0][0];
    expect(url).toContain("audit_status=approved");
    expect(url).toContain("visibility=hidden");
    expect(url).toContain("is_top=false");
    expect(url).not.toMatch(/[?&]status=/);
  },
);
it("serializes every action resource and requires the explicit deletion revision", async () => {
  const item = fixture("post-management");
  const fetch = vi.fn().mockImplementation(async () => response(item));
  vi.stubGlobal("fetch", fetch);
  setCsrf("csrf-management");
  await workspace.postManagement.pin("school", "id", 2, true);
  expect(JSON.parse(fetch.mock.calls[0][1].body)).toEqual({
    data: {
      type: "post-management",
      attributes: { revision: 2, is_top: true },
    },
  });
  await workspace.postManagement.status("school", "id", 2, "offline", "原因");
  expect(JSON.parse(fetch.mock.calls[1][1].body).data.attributes).toEqual({
    revision: 2,
    visibility: "offline",
    reason: "原因",
  });
  const order = fixture("post-order-input");
  expect(
    writeDocument(
      "/api/v1/schools/s/post-management/order-changes",
      order.data.attributes,
    ),
  ).toEqual(order);
  await workspace.postManagement.delete("school", "id", 9);
  expect(fetch.mock.calls[2][1]).toMatchObject({
    method: "DELETE",
    headers: { "If-Match": '"9"', "X-CSRF-Token": "csrf-management" },
  });
});
it("uploads multipart with CSRF and decodes preview and idempotent commit results", async () => {
  const fetch = vi
    .fn()
    .mockResolvedValueOnce(response(fixture("post-import-preview")))
    .mockResolvedValueOnce(response(fixture("post-import-result")));
  vi.stubGlobal("fetch", fetch);
  setCsrf("import-csrf");
  const preview: PostImportPreview = await workspace.postManagement.preview(
    "school",
    new File(["标题,正文\na,b"], "posts.csv"),
  );
  expect(preview.errors[0].row).toBe(3);
  expect(fetch.mock.calls[0][1].body).toBeInstanceOf(FormData);
  expect(fetch.mock.calls[0][1].headers["Content-Type"]).toBeUndefined();
  expect(fetch.mock.calls[0][1].headers["X-CSRF-Token"]).toBe("import-csrf");
  const input = {
    revision: preview.revision,
    digest: preview.digest,
    idempotency_key: "60000000-0000-0000-0000-000000000006",
  };
  const result = await workspace.postManagement.commit(
    "school",
    preview.id,
    input,
  );
  expect(result.state).toBe("committed");
  expect(result.result).toHaveLength(1);
  expect(JSON.parse(fetch.mock.calls[1][1].body)).toEqual({
    data: { type: "post-imports", attributes: input },
  });
});
it("surfaces sorting conflicts without inventing a successful ordering token", async () => {
  vi.stubGlobal(
    "fetch",
    vi
      .fn()
      .mockResolvedValue(
        Response.json(
          { errors: [{ code: "conflict", detail: "版本冲突" }] },
          { status: 409 },
        ),
      ),
  );
  await expect(
    workspace.postManagement.reorder(
      "school",
      fixture("post-order-input").data.attributes,
    ),
  ).rejects.toMatchObject({ status: 409 });
});
