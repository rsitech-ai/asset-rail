import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { App } from "./App";
import { demoPlannerSnapshot } from "./planner/demo";
import type { PlannerRequest } from "./planner/types";

const { mockLoadPlanner } = vi.hoisted(() => ({ mockLoadPlanner: vi.fn() }));

vi.mock("./planner/bridge", () => ({ loadPlanner: mockLoadPlanner }));

beforeEach(() => {
  mockLoadPlanner.mockImplementation((request: PlannerRequest) => demoPlannerSnapshot(request));
});

afterEach(cleanup);

test("plans a route and exposes deterministic exclusion evidence", async () => {
  const user = userEvent.setup();
  render(<App />);

  expect(
    await screen.findByRole("heading", { name: "Route workspace" }),
  ).toBeVisible();
  expect(screen.getByText("Local authority")).toBeVisible();
  expect(screen.getByText("999.85 USDC")).toBeVisible();
  expect(
    screen.getByText(
      "eip155:42161/erc20:0xaf88d065e77c8cc2239327c5edb3a432268e5831",
    ),
  ).toBeVisible();

  await user.click(screen.getByRole("button", { name: "Cheapest" }));
  expect(
    await screen.findByText(/lowest exact withdrawal fee among eligible routes/i),
  ).toBeVisible();

  await user.click(
    screen.getByRole("button", { name: "Review 2 excluded routes" }),
  );
  expect(screen.getByText("Destination chain mismatch")).toBeVisible();
  expect(screen.getByText("Canonical mapping not approved")).toBeVisible();

  await user.click(screen.getByRole("button", { name: "Select BTC balance" }));
  expect(await screen.findByText("0.01995 BTC")).toBeVisible();
  expect(screen.getByRole("heading", { name: "Bitcoin" })).toBeVisible();

  await user.click(screen.getByRole("button", { name: "Select USDC balance" }));
  await user.click(screen.getByRole("button", { name: "Native only" }));
  expect(
    await screen.findByRole("heading", { name: "No route can be recommended" }),
  ).toBeVisible();
});

test("does not advertise unavailable workspace surfaces as enabled controls", async () => {
  render(<App />);

  await screen.findByRole("heading", { name: "Route workspace" });
  expect(screen.getByRole("button", { name: /Destinations/ })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Catalog" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Security" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Workspace profile" })).toBeDisabled();
  expect(screen.getByText("Browser preview · no desktop bridge")).toBeVisible();
});

test("labels deterministic fixture freshness without claiming a live current snapshot", async () => {
  render(<App />);

  await screen.findByRole("heading", { name: "Route workspace" });
  expect(screen.getByText("Fixture within simulated freshness window")).toBeVisible();
  expect(screen.getAllByText("Simulated age 14s")).toHaveLength(2);
  expect(screen.getByText(/Fixture timestamp 2026-07-14/)).toBeVisible();
  expect(screen.getByText("Simulated verification evidence")).toBeVisible();
  expect(screen.queryByText("Snapshot current")).not.toBeInTheDocument();
  expect(screen.queryAllByText(/verified destination/i)).toHaveLength(0);
});

test("does not present stale route evidence for an invalid draft amount", async () => {
  const user = userEvent.setup();
  render(<App />);
  expect(await screen.findByText("999.85 USDC")).toBeVisible();
  const amount = screen.getByRole("textbox", { name: "Withdrawal amount" });
  await user.clear(amount);
  await user.type(amount, "invalid");
  expect(amount).toHaveAttribute("aria-invalid", "true");
  expect(screen.getByText("Use a positive decimal with up to 28 fractional digits.")).toBeVisible();
  expect(screen.getByRole("button", { name: "Compare routes" })).toBeDisabled();
  expect(screen.queryByText("999.85 USDC")).not.toBeInTheDocument();
  expect(screen.getByText("Route recommendation hidden for this draft")).toBeVisible();
  expect(screen.getByText("Edit complete route inputs, then compare again.")).toBeVisible();
  expect(screen.getByRole("complementary", { name: "Destination evidence" })).toBeVisible();
});

test("discloses build origin, source status, and official-versus-community identity", async () => {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByRole("heading", { name: "Route workspace" });

  await user.click(screen.getByRole("button", { name: "Build and source information" }));
  const dialog = screen.getByRole("dialog", { name: "Build and source information" });
  expect(dialog).toBeVisible();
  expect(dialog).toHaveFocus();
  expect(within(dialog).getByText("Local source build")).toBeVisible();
  expect(within(dialog).getByText("Open-source license adopted")).toBeVisible();
  expect(within(dialog).getByText("Apache-2.0")).toBeVisible();
  expect(within(dialog).getByText("unrecorded")).toBeVisible();
  expect(within(dialog).getByText(/Official names, icons, and services are not granted/i)).toBeVisible();
  expect(within(dialog).getByText(/Report vulnerabilities privately through GitHub Security Advisories or info@rsitech.ai/i)).toBeVisible();

  await user.keyboard("{Escape}");
  expect(screen.queryByRole("dialog", { name: "Build and source information" })).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Build and source information" })).toHaveFocus();
});

test("shows a stopped error state instead of an indefinite loading state", async () => {
  mockLoadPlanner.mockRejectedValueOnce(new Error("fixture unavailable"));
  render(<App />);

  expect(await screen.findByText("Planner unavailable")).toBeVisible();
  expect(screen.getByText("No planner result is available")).toBeVisible();
  expect(screen.queryByText("Loading the local planner…")).not.toBeInTheDocument();
});
