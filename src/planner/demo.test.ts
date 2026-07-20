import { expect, test } from "vitest";
import { demoPlannerSnapshot } from "./demo";
import type { PlannerRequest } from "./types";

const request: PlannerRequest = {
  assetSymbol: "USDC",
  destinationId: "ledger-arbitrum",
  amount: "1000.00",
  objective: "BALANCED",
  snapshotAgeSeconds: 14,
};

test("stale browser fixture data fails closed for every network", async () => {
  const snapshot = await demoPlannerSnapshot({ ...request, snapshotAgeSeconds: 121 });
  expect(snapshot.eligible).toHaveLength(0);
  expect(snapshot.excluded).toHaveLength(3);
  expect(snapshot.excluded.every((route) => route.code === "SNAPSHOT_STALE")).toBe(true);
});

test("native-only mode does not treat canonical tokens as native", async () => {
  const snapshot = await demoPlannerSnapshot({ ...request, objective: "NATIVE_ONLY" });
  expect(snapshot.eligible).toHaveLength(0);
  expect(snapshot.excluded.some((route) => route.code === "OBJECTIVE_MISMATCH")).toBe(true);
});

test("browser preview rejects a withdrawal above the available balance", async () => {
  const snapshot = await demoPlannerSnapshot({ ...request, amount: "2000" });
  expect(snapshot.eligible).toHaveLength(0);
  expect(snapshot.excluded.every((route) => route.code === "INSUFFICIENT_BALANCE")).toBe(true);
});

test("browser preview applies minimums and exact increments before destination ranking", async () => {
  const belowMinimum = await demoPlannerSnapshot({ ...request, amount: "0.001" });
  expect(belowMinimum.eligible).toHaveLength(0);
  expect(belowMinimum.excluded.every((route) => route.code === "BELOW_MINIMUM")).toBe(true);

  const invalidIncrement = await demoPlannerSnapshot({ ...request, amount: "1.005" });
  expect(invalidIncrement.eligible).toHaveLength(0);
  expect(
    invalidIncrement.excluded.some(
      (route) => route.exchangeCode === "ARBITRUM" && route.code === "INVALID_INCREMENT",
    ),
  ).toBe(true);
});

test("browser preview uses the Rust planner's non-positive withdrawal exclusion", async () => {
  const snapshot = await demoPlannerSnapshot({ ...request, amount: "0" });
  expect(snapshot.eligible).toHaveLength(0);
  expect(snapshot.excluded.every((route) => route.code === "NON_POSITIVE_WITHDRAWAL")).toBe(true);
});

test("public fixtures expose synthetic destination tokens instead of address or memo data", async () => {
  const snapshot = await demoPlannerSnapshot(request);

  expect(snapshot.destinations).toHaveLength(4);
  for (const destination of snapshot.destinations) {
    expect(destination.address).toMatch(/^synthetic:[a-z0-9-]+$/);
    expect(destination.addressLabel).toBe("Synthetic fixture");
    expect(destination.memo).toBeNull();
  }
});
