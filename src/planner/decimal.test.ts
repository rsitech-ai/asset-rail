import { expect, test } from "vitest";
import { feePercent, subtractDecimal, withdrawalAmountError } from "./decimal";

test("uses exact integer-scaled arithmetic for demo financial values", () => {
  expect(subtractDecimal("1000.00", "0.15")).toBe("999.85");
  expect(subtractDecimal("0.02000000", "0.00005")).toBe("0.01995");
  expect(feePercent("1000.00", "0.15")).toBe("0.015");
});

test("validates a positive fixed-point withdrawal amount", () => {
  expect(withdrawalAmountError("0")).toBe("Withdrawal amount must be greater than zero.");
  expect(withdrawalAmountError("1e3")).toContain("positive decimal");
  expect(withdrawalAmountError("1.000000001")).toBeNull();
  expect(withdrawalAmountError("1.00000000000000000000000000001")).toContain("up to 28");
  expect(withdrawalAmountError("10000000000000000000000000000")).toContain("28 significant");
  expect(withdrawalAmountError("0.0000000000000000000000000001")).toBeNull();
});

test("rejects invalid or over-precise decimal values", () => {
  expect(() => subtractDecimal("1e3", "0.1")).toThrow("Invalid decimal amount");
  expect(() => subtractDecimal("1.00000000000000000000000000001", "0.1")).toThrow(
    "exceeds 28 decimal places",
  );
});
