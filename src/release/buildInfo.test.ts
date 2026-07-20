import { describe, expect, test } from "vitest";
import { createBuildInfo } from "./buildInfo";

describe("createBuildInfo", () => {
  test("defaults to a non-release local source build under the adopted source license", () => {
    expect(createBuildInfo({})).toEqual({
      distributionKind: "development",
      distributionLabel: "Local source build",
      productName: "AssetRail",
      sourceLicenseId: "Apache-2.0",
      sourceLicenseStatus: "Open-source license adopted",
      sourceRevision: "unrecorded",
      sourceUrl: null,
      version: "0.1.1",
      buildNumber: "development",
    });
  });

  test("labels community builds as unofficial and preserves exact source metadata", () => {
    expect(createBuildInfo({
      VITE_DISTRIBUTION_KIND: "community",
      VITE_PRODUCT_NAME: "Rail Planner Community",
      VITE_SOURCE_LICENSE_ID: "Apache-2.0",
      VITE_SOURCE_LICENSE_STATUS: "License adopted",
      VITE_SOURCE_REVISION: "0123456789abcdef0123456789abcdef01234567",
      VITE_SOURCE_URL: "https://example.test/source/0123456789abcdef0123456789abcdef01234567",
      VITE_APP_VERSION: "1.2.3",
      VITE_BUILD_NUMBER: "45",
    })).toEqual({
      distributionKind: "community",
      distributionLabel: "Unofficial community build",
      productName: "Rail Planner Community",
      sourceLicenseId: "Apache-2.0",
      sourceLicenseStatus: "License adopted",
      sourceRevision: "0123456789abcdef0123456789abcdef01234567",
      sourceUrl: "https://example.test/source/0123456789abcdef0123456789abcdef01234567",
      version: "1.2.3",
      buildNumber: "45",
    });
  });

  test("rejects misleading or malformed build metadata", () => {
    expect(() => createBuildInfo({ VITE_DISTRIBUTION_KIND: "community", VITE_PRODUCT_NAME: "AssetRail" })).toThrow(/distinct product name/i);
    expect(() => createBuildInfo({ VITE_DISTRIBUTION_KIND: "official", VITE_SOURCE_REVISION: "main" })).toThrow(/full lowercase Git commit/i);
    expect(() => createBuildInfo({ VITE_SOURCE_URL: "http://example.test/source" })).toThrow(/HTTPS source URL/i);
  });
});
