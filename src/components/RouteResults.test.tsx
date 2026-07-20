import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";
import { RouteResults } from "./RouteResults";
import type { RouteView } from "../planner/types";

function route(id: string, recommended: boolean): RouteView {
  return {
    id,
    exchangeCode: id,
    networkName: id,
    chainId: `chain:${id}`,
    representationId: `chain:${id}/asset:${id}`,
    representationClass: "NATIVE",
    grossAmount: "10",
    fee: "1",
    netReceived: "9",
    effectiveFeePercent: "10",
    estimatedMinutes: 1,
    riskTier: "LOW",
    confidence: "USER_CONFIRMED",
    recommended,
    explanation: `${id} explanation`,
  };
}

test("labels only the route selected by the authority as recommended", () => {
  render(
    <RouteResults
      routes={[route("FIRST", true), route("SECOND", false)]}
      excluded={[]}
      asset="TEST"
    />,
  );

  expect(screen.getAllByText("Recommended")).toHaveLength(1);
});
