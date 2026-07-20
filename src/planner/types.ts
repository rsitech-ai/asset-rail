export type RecommendationObjective =
  | "SAFEST"
  | "CHEAPEST"
  | "FASTEST"
  | "BALANCED"
  | "NATIVE_ONLY";

export type RiskTier = "LOW" | "MEDIUM" | "HIGH" | "BLOCKED";
export type SupportConfidence =
  | "VERIFIED_BY_DESTINATION_API"
  | "VERIFIED_BY_WALLET_CONNECTION"
  | "VERIFIED_BY_SIGNED_OWNERSHIP_PROOF"
  | "CURATED_BY_PLATFORM"
  | "USER_CONFIRMED"
  | "INFERRED"
  | "UNKNOWN";

export type ExclusionCode =
  | "SNAPSHOT_STALE"
  | "INSUFFICIENT_BALANCE"
  | "WITHDRAWAL_DISABLED"
  | "NETWORK_BUSY"
  | "BELOW_MINIMUM"
  | "ABOVE_MAXIMUM"
  | "INVALID_INCREMENT"
  | "MAPPING_UNAPPROVED"
  | "DESTINATION_CHAIN_UNSUPPORTED"
  | "DESTINATION_REPRESENTATION_UNSUPPORTED"
  | "ADDRESS_INVALID"
  | "MEMO_REQUIRED"
  | "MEMO_UNSUPPORTED"
  | "REPRESENTATION_BLOCKED"
  | "RISK_BLOCKED"
  | "CONFIDENCE_INSUFFICIENT"
  | "OBJECTIVE_MISMATCH"
  | "NON_POSITIVE_WITHDRAWAL"
  | "NON_POSITIVE_NET";

export interface PlannerRequest {
  assetSymbol: string;
  destinationId: string;
  amount: string;
  objective: RecommendationObjective;
  snapshotAgeSeconds: number;
}

export interface BalanceView {
  assetSymbol: string;
  assetName: string;
  available: string;
  locked: string;
  fiatValue: string;
  fiatCurrency: string;
  availableNetworks: number;
}

export interface DestinationView {
  id: string;
  name: string;
  kind: string;
  chainId: string;
  chainName: string;
  address: string;
  addressLabel: string;
  confidence: SupportConfidence;
  supportedAssets: string[];
  memo: string | null;
}

export interface RouteView {
  id: string;
  exchangeCode: string;
  networkName: string;
  chainId: string;
  representationId: string;
  representationClass: string;
  grossAmount: string;
  fee: string;
  netReceived: string;
  effectiveFeePercent: string;
  estimatedMinutes: number;
  riskTier: RiskTier;
  confidence: SupportConfidence;
  recommended: boolean;
  explanation: string;
}

export interface ExcludedRouteView {
  exchangeCode: string;
  networkName: string;
  chainId: string;
  code: ExclusionCode;
  title: string;
  detail: string;
}

export interface PlannerSnapshot {
  mode: "OFFLINE_FIXTURE";
  provider: string;
  generatedAt: string;
  snapshotAgeSeconds: number;
  freshForSeconds: number;
  balances: BalanceView[];
  destinations: DestinationView[];
  selection: {
    assetSymbol: string;
    destinationId: string;
    amount: string;
    objective: RecommendationObjective;
  };
  eligible: RouteView[];
  excluded: ExcludedRouteView[];
}
