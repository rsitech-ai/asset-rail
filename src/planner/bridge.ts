import { invoke } from "@tauri-apps/api/core";
import { demoPlannerSnapshot } from "./demo";
import type { PlannerRequest, PlannerSnapshot } from "./types";

const objectives = new Set(["SAFEST", "CHEAPEST", "FASTEST", "BALANCED", "NATIVE_ONLY"]);
const riskTiers = new Set(["LOW", "MEDIUM", "HIGH", "BLOCKED"]);
const representationClasses = new Set([
  "NATIVE", "ISSUER_NATIVE", "CANONICAL_TOKEN", "BRIDGED_CANONICAL", "THIRD_PARTY_WRAPPED", "UNKNOWN",
]);
const confidences = new Set([
  "VERIFIED_BY_DESTINATION_API",
  "VERIFIED_BY_WALLET_CONNECTION",
  "VERIFIED_BY_SIGNED_OWNERSHIP_PROOF",
  "CURATED_BY_PLATFORM",
  "USER_CONFIRMED",
  "INFERRED",
  "UNKNOWN",
]);
const exclusionCodes = new Set([
  "SNAPSHOT_STALE",
  "INSUFFICIENT_BALANCE",
  "WITHDRAWAL_DISABLED",
  "NETWORK_BUSY",
  "BELOW_MINIMUM",
  "ABOVE_MAXIMUM",
  "INVALID_INCREMENT",
  "MAPPING_UNAPPROVED",
  "DESTINATION_CHAIN_UNSUPPORTED",
  "DESTINATION_REPRESENTATION_UNSUPPORTED",
  "ADDRESS_INVALID",
  "MEMO_REQUIRED",
  "MEMO_UNSUPPORTED",
  "REPRESENTATION_BLOCKED",
  "RISK_BLOCKED",
  "CONFIDENCE_INSUFFICIENT",
  "OBJECTIVE_MISMATCH",
  "NON_POSITIVE_WITHDRAWAL",
  "NON_POSITIVE_NET",
]);

type UnknownRecord = Record<string, unknown>;
type FixedDecimal = { coefficient: bigint; scale: number };

const chainIdPattern = /^[-a-z0-9]{3,8}:[-_a-zA-Z0-9]{1,32}$/;
const assetIdPattern = /^[-a-z0-9]{3,8}:[-_a-zA-Z0-9]{1,32}\/[-a-z0-9]{3,8}:[-_.%a-zA-Z0-9]{1,128}$/;

function isRecord(value: unknown): value is UnknownRecord {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

function isNonNegativeInteger(value: unknown): value is number {
  return Number.isInteger(value) && typeof value === "number" && value >= 0;
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every(isString);
}

function parseFixedDecimal(value: unknown): FixedDecimal | null {
  if (!isString(value)) return null;
  const match = /^(\d+)(?:\.(\d{1,28}))?$/.exec(value);
  if (!match) return null;
  const digits = `${match[1]}${match[2] ?? ""}`;
  if (digits.replace(/^0+/, "").length > 28) return null;
  return { coefficient: BigInt(digits), scale: (match[2] ?? "").length };
}

function compareFixed(left: FixedDecimal, right: FixedDecimal): number {
  const scale = Math.max(left.scale, right.scale);
  const leftValue = left.coefficient * 10n ** BigInt(scale - left.scale);
  const rightValue = right.coefficient * 10n ** BigInt(scale - right.scale);
  return leftValue < rightValue ? -1 : leftValue > rightValue ? 1 : 0;
}

function exactDifference(gross: FixedDecimal, fee: FixedDecimal, net: FixedDecimal): boolean {
  const scale = Math.max(gross.scale, fee.scale, net.scale);
  const units = (value: FixedDecimal) => value.coefficient * 10n ** BigInt(scale - value.scale);
  return units(gross) - units(fee) === units(net);
}

function isCanonicalChainId(value: unknown): value is string {
  return isString(value) && chainIdPattern.test(value);
}

function isAssetRepresentationId(value: unknown): value is string {
  return isString(value) && assetIdPattern.test(value);
}

function representationChain(value: string): string {
  return value.slice(0, value.indexOf("/"));
}

function isOfflineFixtureAddress(chainId: string, address: string): boolean {
  const addresses: Record<string, string> = {
    "eip155:42161": "synthetic:wallet-a-arbitrum",
    "eip155:1": "synthetic:wallet-b-ethereum",
    "bip122:000000000019d6689c085ae165831e93": "synthetic:cold-vault-bitcoin",
    "xrpl:0": "synthetic:exchange-xrp",
  };
  return addresses[chainId] === address;
}

function isBalance(value: unknown): boolean {
  if (!isRecord(value)) return false;
  return ["assetSymbol", "assetName", "fiatCurrency"].every((key) => isString(value[key]))
    && ["available", "locked", "fiatValue"].every((key) => parseFixedDecimal(value[key]) !== null)
    && isNonNegativeInteger(value.availableNetworks);
}

function isDestination(value: unknown): boolean {
  if (!isRecord(value)) return false;
  const stringsValid = ["id", "name", "kind", "chainName", "address", "addressLabel"].every(
    (key) => isString(value[key]),
  );
  return stringsValid
    && isCanonicalChainId(value.chainId)
    && isOfflineFixtureAddress(value.chainId, value.address as string)
    && confidences.has(value.confidence as string)
    && isStringArray(value.supportedAssets)
    && (value.memo === null || isString(value.memo));
}

function isRoute(value: unknown): boolean {
  if (!isRecord(value)) return false;
  return ["id", "exchangeCode", "networkName", "explanation"].every((key) => isString(value[key]))
    && isCanonicalChainId(value.chainId)
    && isAssetRepresentationId(value.representationId)
    && representationChain(value.representationId) === value.chainId
    && representationClasses.has(value.representationClass as string)
    && ["grossAmount", "fee", "netReceived", "effectiveFeePercent"].every(
      (key) => parseFixedDecimal(value[key]) !== null,
    )
    && isNonNegativeInteger(value.estimatedMinutes)
    && riskTiers.has(value.riskTier as string)
    && confidences.has(value.confidence as string)
    && typeof value.recommended === "boolean";
}

function isExcludedRoute(value: unknown): boolean {
  if (!isRecord(value)) return false;
  return ["exchangeCode", "networkName", "title", "detail"].every(
    (key) => isString(value[key]),
  ) && isCanonicalChainId(value.chainId) && exclusionCodes.has(value.code as string);
}

function assertPlannerSnapshot(value: unknown): asserts value is PlannerSnapshot {
  const selection = isRecord(value) && isRecord(value.selection) ? value.selection : null;
  const valid = isRecord(value)
    && value.mode === "OFFLINE_FIXTURE"
    && isString(value.provider)
    && isString(value.generatedAt)
    && !Number.isNaN(Date.parse(value.generatedAt))
    && isNonNegativeInteger(value.snapshotAgeSeconds)
    && isNonNegativeInteger(value.freshForSeconds)
    && Array.isArray(value.balances)
    && value.balances.every(isBalance)
    && Array.isArray(value.destinations)
    && value.destinations.every(isDestination)
    && selection !== null
    && isString(selection.assetSymbol)
    && isString(selection.destinationId)
    && parseFixedDecimal(selection.amount) !== null
    && objectives.has(selection.objective as string)
    && Array.isArray(value.eligible)
    && value.eligible.every(isRoute)
    && Array.isArray(value.excluded)
    && value.excluded.every(isExcludedRoute);

  if (!valid) throw new Error("Invalid planner snapshot returned by the desktop bridge");

  const snapshot = value as unknown as PlannerSnapshot;
  const destination = snapshot.destinations.find(
    (item) => item.id === snapshot.selection.destinationId,
  );
  const selectedBalance = snapshot.balances.some(
    (item) => item.assetSymbol === snapshot.selection.assetSymbol,
  );
  const selectedBalanceRecord = snapshot.balances.find(
    (item) => item.assetSymbol === snapshot.selection.assetSymbol,
  );
  const selectedAmount = parseFixedDecimal(snapshot.selection.amount);
  const recommendationCount = snapshot.eligible.filter((route) => route.recommended).length;
  const routeIds = new Set(snapshot.eligible.map((route) => route.id));
  const crossFieldsValid = destination !== undefined
    && selectedBalance
    && selectedBalanceRecord !== undefined
    && selectedAmount !== null
    && destination.supportedAssets.includes(snapshot.selection.assetSymbol)
    && compareFixed(selectedAmount, parseFixedDecimal(selectedBalanceRecord.available)!) <= 0
    && snapshot.eligible.every((route) => route.chainId === destination.chainId)
    && snapshot.eligible.every((route) => route.confidence === destination.confidence)
    && snapshot.eligible.every((route) => {
      const gross = parseFixedDecimal(route.grossAmount)!;
      const fee = parseFixedDecimal(route.fee)!;
      const net = parseFixedDecimal(route.netReceived)!;
      return compareFixed(gross, selectedAmount) === 0
        && compareFixed(net, { coefficient: 0n, scale: 0 }) > 0
        && compareFixed(fee, gross) < 0
        && exactDifference(gross, fee, net);
    })
    && routeIds.size === snapshot.eligible.length
    && recommendationCount === (snapshot.eligible.length > 0 ? 1 : 0);
  if (!crossFieldsValid) {
    throw new Error("Invalid planner snapshot returned by the desktop bridge");
  }
}

function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

export async function loadPlanner(request: PlannerRequest): Promise<PlannerSnapshot> {
  if (!isTauriRuntime()) return demoPlannerSnapshot(request);
  const snapshot: unknown = await invoke("planner_snapshot", { request });
  assertPlannerSnapshot(snapshot);
  return snapshot;
}
