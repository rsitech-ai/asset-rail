const SCALE = 28;
const SCALE_FACTOR = 10n ** BigInt(SCALE);

export function withdrawalAmountError(value: string): string | null {
  const match = /^(\d+)(?:\.(\d{1,28}))?$/.exec(value.trim());
  if (!match) return "Use a positive decimal with up to 28 fractional digits.";
  const digits = `${match[1]}${match[2] ?? ""}`;
  const significantDigits = digits.replace(/^0+/, "").length;
  if (significantDigits > 28) return "Use no more than 28 significant digits.";
  return significantDigits > 0 ? null : "Withdrawal amount must be greater than zero.";
}

function toUnits(value: string): bigint {
  const match = /^(\d+)(?:\.(\d+))?$/.exec(value.trim());
  if (!match) throw new Error(`Invalid decimal amount: ${value}`);
  const fraction = (match[2] ?? "").padEnd(SCALE, "0");
  if (fraction.length > SCALE) throw new Error(`Amount exceeds ${SCALE} decimal places`);
  if (`${match[1]}${match[2] ?? ""}`.replace(/^0+/, "").length > 28) {
    throw new Error("Amount exceeds 28 significant digits");
  }
  return BigInt(match[1]) * SCALE_FACTOR + BigInt(fraction || "0");
}

function fromUnits(units: bigint): string {
  const whole = units / SCALE_FACTOR;
  const fraction = (units % SCALE_FACTOR).toString().padStart(SCALE, "0").replace(/0+$/, "");
  return fraction ? `${whole}.${fraction}` : whole.toString();
}

export function subtractDecimal(gross: string, fee: string): string {
  const result = toUnits(gross) - toUnits(fee);
  if (result < 0n) throw new Error("Withdrawal fee exceeds gross amount");
  return fromUnits(result);
}

export function compareDecimal(left: string, right: string): number {
  const leftUnits = toUnits(left);
  const rightUnits = toUnits(right);
  return leftUnits < rightUnits ? -1 : leftUnits > rightUnits ? 1 : 0;
}

export function isDecimalMultiple(value: string, increment: string): boolean {
  const incrementUnits = toUnits(increment);
  return incrementUnits !== 0n && toUnits(value) % incrementUnits === 0n;
}

export function feePercent(gross: string, fee: string): string {
  const grossUnits = toUnits(gross);
  if (grossUnits === 0n) return "0";
  const scaledPercent = (toUnits(fee) * 100n * 10_000n) / grossUnits;
  const whole = scaledPercent / 10_000n;
  const fraction = (scaledPercent % 10_000n)
    .toString()
    .padStart(4, "0")
    .replace(/0+$/, "");
  return fraction ? `${whole}.${fraction}` : whole.toString();
}
