import { compareDecimal, feePercent, isDecimalMultiple, subtractDecimal } from "./decimal";
import type {
  BalanceView,
  DestinationView,
  ExcludedRouteView,
  ExclusionCode,
  PlannerRequest,
  PlannerSnapshot,
  RecommendationObjective,
  RiskTier,
  RouteView,
} from "./types";

const balances: BalanceView[] = [
  { assetSymbol: "USDC", assetName: "USD Coin", available: "1842.50", locked: "125.00", fiatValue: "1695.10", fiatCurrency: "EUR", availableNetworks: 3 },
  { assetSymbol: "BTC", assetName: "Bitcoin", available: "0.0348", locked: "0", fiatValue: "3286.41", fiatCurrency: "EUR", availableNetworks: 2 },
  { assetSymbol: "ETH", assetName: "Ethereum", available: "1.284", locked: "0.100", fiatValue: "3128.22", fiatCurrency: "EUR", availableNetworks: 1 },
  { assetSymbol: "XRP", assetName: "XRP", available: "9250", locked: "250", fiatValue: "4871.30", fiatCurrency: "EUR", availableNetworks: 1 },
];

const destinations: DestinationView[] = [
  {
    id: "ledger-arbitrum",
    name: "Hardware wallet A · Arbitrum",
    kind: "Hardware wallet",
    chainId: "eip155:42161",
    chainName: "Arbitrum One",
    address: "synthetic:wallet-a-arbitrum",
    addressLabel: "Synthetic fixture",
    confidence: "VERIFIED_BY_WALLET_CONNECTION",
    supportedAssets: ["USDC"],
    memo: null,
  },
  {
    id: "trezor-ethereum",
    name: "Hardware wallet B · Ethereum",
    kind: "Hardware wallet",
    chainId: "eip155:1",
    chainName: "Ethereum",
    address: "synthetic:wallet-b-ethereum",
    addressLabel: "Synthetic fixture",
    confidence: "VERIFIED_BY_SIGNED_OWNERSHIP_PROOF",
    supportedAssets: ["ETH", "USDC"],
    memo: null,
  },
  {
    id: "cold-bitcoin",
    name: "Cold vault · Bitcoin",
    kind: "Self-custody",
    chainId: "bip122:000000000019d6689c085ae165831e93",
    chainName: "Bitcoin",
    address: "synthetic:cold-vault-bitcoin",
    addressLabel: "Synthetic fixture",
    confidence: "VERIFIED_BY_SIGNED_OWNERSHIP_PROOF",
    supportedAssets: ["BTC"],
    memo: null,
  },
  {
    id: "kraken-xrp",
    name: "Simulated exchange · XRP deposit",
    kind: "Exchange / VASP",
    chainId: "xrpl:0",
    chainName: "XRP Ledger",
    address: "synthetic:exchange-xrp",
    addressLabel: "Synthetic fixture",
    confidence: "VERIFIED_BY_DESTINATION_API",
    supportedAssets: ["XRP"],
    memo: null,
  },
];

interface DemoNetwork {
  asset: string;
  exchangeCode: string;
  networkName: string;
  chainId: string;
  representationId: string;
  representationClass: string;
  fee: string;
  minimum: string;
  maximum: string;
  increment: string;
  minutes: number;
  risk: RiskTier;
  mappingApproved: boolean;
}

const networks: DemoNetwork[] = [
  { asset: "USDC", exchangeCode: "ARBITRUM", networkName: "Arbitrum One", chainId: "eip155:42161", representationId: "eip155:42161/erc20:0xaf88d065e77c8cc2239327c5edb3a432268e5831", representationClass: "CANONICAL_TOKEN", fee: "0.15", minimum: "1", maximum: "100000", increment: "0.01", minutes: 2, risk: "LOW", mappingApproved: true },
  { asset: "USDC", exchangeCode: "ETH", networkName: "Ethereum", chainId: "eip155:1", representationId: "eip155:1/erc20:0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48", representationClass: "ISSUER_NATIVE", fee: "3.20", minimum: "10", maximum: "100000", increment: "0.01", minutes: 8, risk: "LOW", mappingApproved: true },
  { asset: "USDC", exchangeCode: "BSC", networkName: "BNB Smart Chain", chainId: "eip155:56", representationId: "eip155:56/bep20:0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d", representationClass: "THIRD_PARTY_WRAPPED", fee: "0.20", minimum: "5", maximum: "100000", increment: "0.01", minutes: 3, risk: "HIGH", mappingApproved: false },
  { asset: "BTC", exchangeCode: "BTC", networkName: "Bitcoin", chainId: "bip122:000000000019d6689c085ae165831e93", representationId: "bip122:000000000019d6689c085ae165831e93/slip44:0", representationClass: "NATIVE", fee: "0.00005", minimum: "0.0001", maximum: "100", increment: "0.00000001", minutes: 35, risk: "LOW", mappingApproved: true },
  { asset: "BTC", exchangeCode: "LIGHTNING", networkName: "Bitcoin Lightning", chainId: "bip122:000000000019d6689c085ae165831e93", representationId: "bip122:000000000019d6689c085ae165831e93/slip44:0", representationClass: "NATIVE", fee: "0.000001", minimum: "0.00001", maximum: "0.1", increment: "0.00000001", minutes: 1, risk: "MEDIUM", mappingApproved: false },
  { asset: "ETH", exchangeCode: "ETH", networkName: "Ethereum", chainId: "eip155:1", representationId: "eip155:1/slip44:60", representationClass: "NATIVE", fee: "0.0012", minimum: "0.01", maximum: "1000", increment: "0.00000001", minutes: 8, risk: "LOW", mappingApproved: true },
  { asset: "XRP", exchangeCode: "XRP", networkName: "XRP Ledger", chainId: "xrpl:0", representationId: "xrpl:0/slip44:144", representationClass: "NATIVE", fee: "0.20", minimum: "20", maximum: "10000000", increment: "0.000001", minutes: 1, risk: "LOW", mappingApproved: true },
];

const exclusionText: Record<ExclusionCode, [string, string]> = {
  SNAPSHOT_STALE: ["Snapshot expired", "Refresh critical exchange data before using this route."],
  INSUFFICIENT_BALANCE: ["Insufficient available balance", "Reduce the amount or wait for locked funds to become available."],
  WITHDRAWAL_DISABLED: ["Withdrawals unavailable", "The exchange currently disables withdrawal on this network."],
  NETWORK_BUSY: ["Network busy", "Workspace policy blocks busy networks until service stabilizes."],
  BELOW_MINIMUM: ["Below exchange minimum", "Increase the amount to satisfy the network minimum."],
  ABOVE_MAXIMUM: ["Above exchange maximum", "Reduce the amount to the network maximum or lower."],
  INVALID_INCREMENT: ["Invalid amount increment", "Use an amount aligned to the exchange withdrawal increment."],
  MAPPING_UNAPPROVED: ["Canonical mapping not approved", "AssetRail cannot prove this exchange code maps to the intended chain and token."],
  DESTINATION_CHAIN_UNSUPPORTED: ["Destination chain mismatch", "The fixture destination evidence maps to a different canonical chain."],
  DESTINATION_REPRESENTATION_UNSUPPORTED: ["Token representation unsupported", "The destination does not explicitly support this exact CAIP-19 asset."],
  ADDRESS_INVALID: ["Address invalid for chain", "The address fails validation for the explicitly selected chain."],
  MEMO_REQUIRED: ["Memo required", "Add the destination memo or tag before planning this route."],
  MEMO_UNSUPPORTED: ["Memo not supported", "Remove the memo because this network does not accept one."],
  REPRESENTATION_BLOCKED: ["Representation blocked", "Unknown asset representations are not eligible for recommendation."],
  RISK_BLOCKED: ["Route blocked by risk policy", "This route is not eligible under the current risk policy."],
  CONFIDENCE_INSUFFICIENT: ["Destination evidence insufficient", "Verify destination support through a stronger evidence source."],
  OBJECTIVE_MISMATCH: ["Not a native representation", "Native-only mode excludes token and wrapped representations."],
  NON_POSITIVE_WITHDRAWAL: ["Withdrawal amount must be positive", "Enter an amount greater than zero before planning a route."],
  NON_POSITIVE_NET: ["Fee consumes the withdrawal", "Increase the amount so the exact net received remains positive."],
};

function explanation(objective: RecommendationObjective, network: string): string {
  if (objective === "CHEAPEST") return `${network} has the lowest exact withdrawal fee among eligible routes.`;
  if (objective === "FASTEST") return `${network} has the shortest exchange arrival estimate among eligible routes.`;
  if (objective === "SAFEST") return `${network} has the lowest eligible risk tier for this simulated destination evidence.`;
  if (objective === "NATIVE_ONLY") return `${network} uses the native or issuer-native asset representation.`;
  return `${network} leads on risk first, then fee and expected arrival time as deterministic tie-breakers.`;
}

function excluded(network: DemoNetwork, code: ExclusionCode): ExcludedRouteView {
  const [title, detail] = exclusionText[code];
  return { exchangeCode: network.exchangeCode, networkName: network.networkName, chainId: network.chainId, code, title, detail };
}

export async function demoPlannerSnapshot(request: PlannerRequest): Promise<PlannerSnapshot> {
  await Promise.resolve();
  const destination = destinations.find((item) => item.id === request.destinationId);
  if (!destination) throw new Error("Unknown destination profile");
  const balance = balances.find((item) => item.assetSymbol === request.assetSymbol);
  if (!balance) throw new Error("Unknown asset");
  const assetNetworks = networks.filter((network) => network.asset === request.assetSymbol);
  const eligible: RouteView[] = [];
  const excludedRoutes: ExcludedRouteView[] = [];

  for (const network of assetNetworks) {
    let code: ExclusionCode | null = null;
    if (request.snapshotAgeSeconds > 120) code = "SNAPSHOT_STALE";
    else if (compareDecimal(request.amount, "0") <= 0) code = "NON_POSITIVE_WITHDRAWAL";
    else if (compareDecimal(request.amount, balance.available) > 0) code = "INSUFFICIENT_BALANCE";
    else if (compareDecimal(request.amount, network.minimum) < 0) code = "BELOW_MINIMUM";
    else if (compareDecimal(request.amount, network.maximum) > 0) code = "ABOVE_MAXIMUM";
    else if (!isDecimalMultiple(request.amount, network.increment)) code = "INVALID_INCREMENT";
    else if (!network.mappingApproved) code = "MAPPING_UNAPPROVED";
    else if (network.risk === "BLOCKED") code = "RISK_BLOCKED";
    else if (network.chainId !== destination.chainId) code = "DESTINATION_CHAIN_UNSUPPORTED";
    else if (!destination.supportedAssets.includes(request.assetSymbol)) code = "DESTINATION_REPRESENTATION_UNSUPPORTED";
    else if (compareDecimal(request.amount, network.fee) <= 0) code = "NON_POSITIVE_NET";
    else if (request.objective === "NATIVE_ONLY" && !["NATIVE", "ISSUER_NATIVE"].includes(network.representationClass)) code = "OBJECTIVE_MISMATCH";
    if (code) {
      excludedRoutes.push(excluded(network, code));
      continue;
    }
    eligible.push({
      id: `${request.assetSymbol}:${network.exchangeCode}`,
      exchangeCode: network.exchangeCode,
      networkName: network.networkName,
      chainId: network.chainId,
      representationId: network.representationId,
      representationClass: network.representationClass,
      grossAmount: request.amount,
      fee: network.fee,
      netReceived: subtractDecimal(request.amount, network.fee),
      effectiveFeePercent: feePercent(request.amount, network.fee),
      estimatedMinutes: network.minutes,
      riskTier: network.risk,
      confidence: destination.confidence,
      recommended: true,
      explanation: explanation(request.objective, network.exchangeCode),
    });
  }

  return {
    mode: "OFFLINE_FIXTURE",
    provider: "Binance Spot · deterministic fixture",
    generatedAt: "2026-07-14T08:45:00Z",
    snapshotAgeSeconds: request.snapshotAgeSeconds,
    freshForSeconds: 120,
    balances,
    destinations,
    selection: { ...request },
    eligible,
    excluded: excludedRoutes,
  };
}
