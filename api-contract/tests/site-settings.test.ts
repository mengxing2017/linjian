import { afterEach, expect, it, vi } from "vitest";
import { readFileSync } from "node:fs";
import {
  siteSettings,
  type SiteSettingsInput,
} from "../src/modules/business";
import { resourceType, setCsrf } from "../src/http";

const fixture = (name: string) =>
  JSON.parse(
    readFileSync(new URL(`../fixtures/${name}.json`, import.meta.url), "utf8"),
  );

afterEach(() => {
  vi.unstubAllGlobals();
  setCsrf("");
});

it("reads public site settings without a protected write document", async () => {
  const output = fixture("site-settings-public");
  const fetch = vi
    .fn()
    .mockImplementation(async () =>
      Response.json(output, {
        headers: { "content-type": "application/vnd.api+json" },
      }),
    );
  vi.stubGlobal("fetch", fetch);
  const result = await siteSettings.public();
  expect(result.site_name).toBe("示例校园");
  expect(fetch).toHaveBeenCalledTimes(1);
  const [url, options] = fetch.mock.calls[0];
  expect(url).toBe("/api/v1/site-settings");
  expect(options.method).toBe("GET");
  expect(options.body).toBeUndefined();
  expect(options.headers["X-CSRF-Token"]).toBeUndefined();
});

it("sends every admin field explicitly and uses the site-settings resource type", async () => {
  const input = fixture("site-settings-input");
  const output = fixture("site-settings-admin");
  const fetch = vi
    .fn()
    .mockImplementation(async () =>
      Response.json(output, {
        headers: { "content-type": "application/vnd.api+json" },
      }),
    );
  vi.stubGlobal("fetch", fetch);
  setCsrf("csrf-site-settings");
  await siteSettings.update(input.data.attributes as SiteSettingsInput);
  const [url, options] = fetch.mock.calls[0];
  expect(url).toBe("/api/v1/platform/site-settings");
  expect(options.method).toBe("PUT");
  expect(options.headers["X-CSRF-Token"]).toBe("csrf-site-settings");
  expect(JSON.parse(options.body)).toEqual(input);
  expect(resourceType("/api/v1/platform/site-settings")).toBe("site-settings");
});

it("keeps public and admin fixtures on the same public fields", () => {
  const publicFixture = fixture("site-settings-public");
  const adminFixture = fixture("site-settings-admin");
  for (const key of Object.keys(publicFixture.data.attributes)) {
    expect(adminFixture.data.attributes[key]).toEqual(
      publicFixture.data.attributes[key],
    );
  }
  expect(fixture("site-settings-public-empty").data.attributes.site_name).toBe(
    null,
  );
});
