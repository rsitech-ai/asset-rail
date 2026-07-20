import { invoke } from "@tauri-apps/api/core";
import { afterEach, expect, test, vi } from "vitest";
import { loadPlanner } from "./bridge";
import { demoPlannerSnapshot } from "./demo";
import type { PlannerRequest } from "./types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const request: PlannerRequest = {
  assetSymbol: "USDC",
  destinationId: "ledger-arbitrum",
  amount: "1000.00",
  objective: "BALANCED",
  snapshotAgeSeconds: 14,
};

afterEach(() => {
  Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
  vi.mocked(invoke).mockReset();
});

test("accepts the public synthetic fixture contract returned by the desktop bridge", async () => {
  Object.defineProperty(window, "__TAURI_INTERNALS__", {
    configurable: true,
    value: {},
  });
  const snapshot = await demoPlannerSnapshot(request);
  vi.mocked(invoke).mockResolvedValue(snapshot);

  await expect(loadPlanner(request)).resolves.toEqual(snapshot);
});

test("fails closed when the desktop IPC response violates the planner contract", async () => {
  Object.defineProperty(window, "__TAURI_INTERNALS__", {
    configurable: true,
    value: {},
  });
  vi.mocked(invoke).mockResolvedValue({
    mode: "UNKNOWN_MODE",
    eligible: [{ riskTier: "TRUST_ME" }],
  });

  await expect(loadPlanner(request)).rejects.toThrow("Invalid planner snapshot");
});

test("fails closed when IPC fields are individually valid but internally contradictory", async () => {
  Object.defineProperty(window, "__TAURI_INTERNALS__", {
    configurable: true,
    value: {},
  });
  vi.mocked(invoke).mockResolvedValue({
    mode: "OFFLINE_FIXTURE",
    provider: "Fixture",
    generatedAt: "2026-07-17T08:00:00Z",
    snapshotAgeSeconds: 1,
    freshForSeconds: 120,
    balances: [],
    destinations: [],
    selection: {
      assetSymbol: "USDC",
      destinationId: "missing",
      amount: "1",
      objective: "BALANCED",
    },
    eligible: [],
    excluded: [],
  });

  await expect(loadPlanner(request)).rejects.toThrow("Invalid planner snapshot");
});

test.each([
  ["exponent amount", (snapshot: Awaited<ReturnType<typeof demoPlannerSnapshot>>) => {
    snapshot.eligible[0].grossAmount = "1e3";
  }],
  ["contradictory arithmetic", (snapshot: Awaited<ReturnType<typeof demoPlannerSnapshot>>) => {
    snapshot.eligible[0].netReceived = "999.84";
  }],
  ["cross-chain representation", (snapshot: Awaited<ReturnType<typeof demoPlannerSnapshot>>) => {
    snapshot.eligible[0].representationId = "eip155:1/erc20:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
  }],
  ["unknown destination address", (snapshot: Awaited<ReturnType<typeof demoPlannerSnapshot>>) => {
    snapshot.destinations[0].address = "0x0000000000000000000000000000000000000000";
  }],
  ["cross-chain synthetic destination token", (snapshot: Awaited<ReturnType<typeof demoPlannerSnapshot>>) => {
    snapshot.destinations[0].address = snapshot.destinations[1].address;
  }],
] as const)("fails closed on %s in desktop planner evidence", async (_label, mutate) => {
  Object.defineProperty(window, "__TAURI_INTERNALS__", {
    configurable: true,
    value: {},
  });
  const snapshot = structuredClone(await demoPlannerSnapshot(request));
  mutate(snapshot);
  vi.mocked(invoke).mockResolvedValue(snapshot);

  await expect(loadPlanner(request)).rejects.toThrow("Invalid planner snapshot");
});
