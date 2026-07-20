import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, test, vi } from "vitest";
import { App } from "./App";
import { loadPlanner } from "./planner/bridge";
import { demoPlannerSnapshot } from "./planner/demo";
import type { PlannerRequest, PlannerSnapshot } from "./planner/types";

vi.mock("./planner/bridge", () => ({ loadPlanner: vi.fn() }));

const initialRequest: PlannerRequest = {
  assetSymbol: "USDC",
  destinationId: "ledger-arbitrum",
  amount: "1000.00",
  objective: "BALANCED",
  snapshotAgeSeconds: 14,
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

beforeEach(() => {
  vi.mocked(loadPlanner).mockReset();
  vi.mocked(loadPlanner).mockImplementation(demoPlannerSnapshot);
});

test("hides evaluated route results while edited inputs are pending", async () => {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByText("999.85 USDC");

  await user.selectOptions(
    screen.getByRole("combobox", { name: "To fixture destination" }),
    "trezor-ethereum",
  );

  expect(screen.getByText("Route recommendation hidden for this draft")).toBeVisible();
  expect(screen.queryByRole("article")).not.toBeInTheDocument();
  expect(within(screen.getByLabelText("Destination evidence")).getByText("eip155:42161")).toBeVisible();
});

test("only the latest evaluation may replace the visible snapshot", async () => {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByText("999.85 USDC");

  const btcResult = deferred<PlannerSnapshot>();
  const ethResult = deferred<PlannerSnapshot>();
  vi.mocked(loadPlanner)
    .mockImplementationOnce(() => btcResult.promise)
    .mockImplementationOnce(() => ethResult.promise);

  await user.click(screen.getByRole("button", { name: "Select BTC balance" }));
  await user.click(screen.getByRole("button", { name: "Select ETH balance" }));

  ethResult.resolve(
    await demoPlannerSnapshot({
      ...initialRequest,
      assetSymbol: "ETH",
      destinationId: "trezor-ethereum",
      amount: "1.00",
    }),
  );
  expect(await screen.findByRole("heading", { name: "Ethereum" })).toBeVisible();

  const btcSnapshot = await demoPlannerSnapshot({
      ...initialRequest,
      assetSymbol: "BTC",
      destinationId: "cold-bitcoin",
      amount: "0.02",
    });
  await act(async () => {
    btcResult.resolve(btcSnapshot);
    await btcResult.promise;
  });

  expect(screen.getByRole("heading", { name: "Ethereum" })).toBeVisible();
  expect(screen.queryByRole("heading", { name: "Bitcoin" })).not.toBeInTheDocument();
});

test("a failed latest evaluation does not leave prior results represented as current", async () => {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByText("999.85 USDC");

  vi.mocked(loadPlanner).mockRejectedValueOnce(new Error("invalid decimal amount"));
  const amount = screen.getByRole("textbox", { name: /Withdrawal amount/ });
  await user.clear(amount);
  await user.type(amount, "900");
  await user.click(screen.getByRole("button", { name: "Compare routes" }));

  expect(await screen.findByRole("alert")).toHaveTextContent("invalid decimal amount");
  expect(screen.queryByText("999.85 USDC")).not.toBeInTheDocument();
});

test("rejects malformed amounts before invoking the planner bridge", async () => {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByText("999.85 USDC");
  vi.mocked(loadPlanner).mockClear();

  const amount = screen.getByRole("textbox", { name: /Withdrawal amount/ });
  await user.clear(amount);
  await user.type(amount, "1e3");

  expect(amount).toHaveAttribute("aria-invalid", "true");
  expect(screen.getByText("Use a positive decimal with up to 28 fractional digits.")).toBeVisible();
  expect(screen.getByRole("button", { name: "Compare routes" })).toBeDisabled();
  expect(loadPlanner).not.toHaveBeenCalled();
});
