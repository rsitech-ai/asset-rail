use reqwest::{StatusCode, header::HeaderMap};
#[cfg(all(feature = "test-transport", any(debug_assertions, test)))]
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Instant;

const IP_WEIGHT_HEADERS: [&str; 3] = [
    "x-mbx-used-weight",
    "x-mbx-used-weight-1m",
    "x-sapi-used-ip-weight-1m",
];
const UID_WEIGHT_HEADERS: [&str; 1] = ["x-sapi-used-uid-weight-1m"];
const WEIGHT_WINDOW_MILLIS: u64 = 60_000;
const FALLBACK_RETRY_429_MILLIS: u64 = 60_000;
const FALLBACK_RETRY_418_MILLIS: u64 = 3 * 24 * 60 * 60 * 1_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeightScope {
    Ip,
    Uid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RefreshPriority {
    Background,
    Operator,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct RequestWeight {
    ip: u64,
    uid: u64,
}

impl RequestWeight {
    #[must_use]
    pub(super) const fn new(ip: u64, uid: u64) -> Self {
        Self { ip, uid }
    }

    const fn for_scope(self, scope: WeightScope) -> u64 {
        match scope {
            WeightScope::Ip => self.ip,
            WeightScope::Uid => self.uid,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RateBudget {
    ip_limit: u64,
    uid_limit: u64,
    reserved_operator: u64,
    used_ip: u64,
    used_uid: u64,
    last_weight_activity_millis: u64,
    blocked_until_millis: Option<u64>,
    clock: RateClock,
}

#[derive(Clone, Debug)]
enum RateClock {
    Monotonic(Instant),
    #[cfg(all(feature = "test-transport", any(debug_assertions, test)))]
    Deterministic(DeterministicRateClock),
}

impl RateClock {
    fn now_millis(&self) -> u64 {
        match self {
            Self::Monotonic(started_at) => {
                u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
            }
            #[cfg(all(feature = "test-transport", any(debug_assertions, test)))]
            Self::Deterministic(clock) => clock.now_millis(),
        }
    }
}

/// A deterministic monotonic clock available only in debug builds.
#[cfg(all(feature = "test-transport", any(debug_assertions, test)))]
#[derive(Clone, Debug)]
pub(super) struct DeterministicRateClock {
    now_millis: Arc<AtomicU64>,
}

#[cfg(all(feature = "test-transport", any(debug_assertions, test)))]
impl DeterministicRateClock {
    #[must_use]
    pub(super) fn new(now_millis: u64) -> Self {
        Self {
            now_millis: Arc::new(AtomicU64::new(now_millis)),
        }
    }

    /// Moves the debug-only test clock to an explicit monotonic instant.
    pub(super) fn set_now_millis(&self, now_millis: u64) {
        self.now_millis.store(now_millis, Ordering::Relaxed);
    }

    fn now_millis(&self) -> u64 {
        self.now_millis.load(Ordering::Relaxed)
    }
}

impl RateBudget {
    #[must_use]
    pub fn new(ip_limit: u64, uid_limit: u64, reserved_operator: u64) -> Self {
        Self {
            ip_limit,
            uid_limit,
            reserved_operator,
            used_ip: 0,
            used_uid: 0,
            last_weight_activity_millis: 0,
            blocked_until_millis: None,
            clock: RateClock::Monotonic(Instant::now()),
        }
    }

    /// Creates a rate budget with deterministic time for debug-only tests.
    #[cfg(all(test, feature = "test-transport"))]
    #[must_use]
    pub(super) fn with_test_clock(
        ip_limit: u64,
        uid_limit: u64,
        reserved_operator: u64,
        clock: DeterministicRateClock,
    ) -> Self {
        let last_weight_activity_millis = clock.now_millis();
        Self {
            ip_limit,
            uid_limit,
            reserved_operator,
            used_ip: 0,
            used_uid: 0,
            last_weight_activity_millis,
            blocked_until_millis: None,
            clock: RateClock::Deterministic(clock),
        }
    }

    pub(crate) fn install_production_clock(&mut self) {
        self.clock = RateClock::Monotonic(Instant::now());
        self.reset_window();
    }

    #[cfg(all(feature = "test-transport", any(debug_assertions, test)))]
    pub(super) fn install_test_clock(&mut self, clock: DeterministicRateClock) {
        self.clock = RateClock::Deterministic(clock);
        self.reset_window();
    }

    #[must_use]
    pub const fn used(&self, scope: WeightScope) -> u64 {
        match scope {
            WeightScope::Ip => self.used_ip,
            WeightScope::Uid => self.used_uid,
        }
    }

    #[must_use]
    pub const fn blocked_until_millis(&self) -> Option<u64> {
        self.blocked_until_millis
    }

    /// Updates authoritative provider usage snapshots from Binance headers.
    ///
    /// # Errors
    /// Returns an error when a recognized header is not an unsigned integer.
    pub(super) fn observe_headers(&mut self, headers: &HeaderMap) -> Result<(), BudgetError> {
        let now_millis = self.clock.now_millis();
        self.roll_window_if_idle(now_millis);
        if let Some(used) = parse_max_header(headers, &IP_WEIGHT_HEADERS)? {
            self.used_ip = self.used_ip.max(used);
        }
        if let Some(used) = parse_max_header(headers, &UID_WEIGHT_HEADERS)? {
            self.used_uid = self.used_uid.max(used);
        }
        self.last_weight_activity_millis = now_millis;
        Ok(())
    }

    /// Records response weight and opens the circuit for Binance 429/418
    /// responses for the complete valid `Retry-After` duration, or for a
    /// conservative status-specific fallback when the header is unusable.
    ///
    /// # Errors
    /// Returns an error for missing, malformed, or overflowing `Retry-After`
    /// values and for malformed recognized weight headers.
    pub(super) fn observe_response(
        &mut self,
        status: StatusCode,
        headers: &HeaderMap,
    ) -> Result<(), BudgetError> {
        let mut retry_error = None;
        if matches!(status.as_u16(), 418 | 429) {
            let parsed_retry_millis = headers
                .get("retry-after")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .and_then(|seconds| seconds.checked_mul(1_000));
            let retry_millis = parsed_retry_millis.unwrap_or_else(|| {
                retry_error = Some(BudgetError::MissingOrMalformedRetryAfter);
                if status.as_u16() == 418 {
                    FALLBACK_RETRY_418_MILLIS
                } else {
                    FALLBACK_RETRY_429_MILLIS
                }
            });
            let blocked_until_millis = self.clock.now_millis().saturating_add(retry_millis);
            self.blocked_until_millis = Some(
                self.blocked_until_millis
                    .map_or(blocked_until_millis, |current| {
                        current.max(blocked_until_millis)
                    }),
            );
        }
        let header_result = self.observe_headers(headers);
        if let Some(error) = retry_error {
            return Err(error);
        }
        header_result
    }

    /// Atomically reserves predicted IP and UID weight before a request.
    ///
    /// # Errors
    /// Fails while retry-blocked, before background work can consume the
    /// operator reserve, on exhaustion, or on arithmetic overflow.
    pub(super) fn reserve(
        &mut self,
        weight: RequestWeight,
        priority: RefreshPriority,
    ) -> Result<(), BudgetError> {
        let now_millis = self.clock.now_millis();
        self.roll_window_if_idle(now_millis);
        if let Some(blocked_until_millis) = self.blocked_until_millis {
            if now_millis < blocked_until_millis {
                return Err(BudgetError::CircuitOpen {
                    blocked_until_millis,
                });
            }
            self.blocked_until_millis = None;
        }

        let next_ip = self
            .used_ip
            .checked_add(weight.for_scope(WeightScope::Ip))
            .ok_or(BudgetError::Overflow)?;
        let next_uid = self
            .used_uid
            .checked_add(weight.for_scope(WeightScope::Uid))
            .ok_or(BudgetError::Overflow)?;
        self.check_scope(WeightScope::Ip, next_ip, priority)?;
        self.check_scope(WeightScope::Uid, next_uid, priority)?;

        self.used_ip = next_ip;
        self.used_uid = next_uid;
        self.last_weight_activity_millis = now_millis;
        Ok(())
    }

    fn reset_window(&mut self) {
        self.used_ip = 0;
        self.used_uid = 0;
        self.last_weight_activity_millis = self.clock.now_millis();
    }

    fn roll_window_if_idle(&mut self, now_millis: u64) {
        if now_millis.saturating_sub(self.last_weight_activity_millis) >= WEIGHT_WINDOW_MILLIS {
            self.used_ip = 0;
            self.used_uid = 0;
            self.last_weight_activity_millis = now_millis;
        }
    }

    fn check_scope(
        &self,
        scope: WeightScope,
        next: u64,
        priority: RefreshPriority,
    ) -> Result<(), BudgetError> {
        let limit = match scope {
            WeightScope::Ip => self.ip_limit,
            WeightScope::Uid => self.uid_limit,
        };
        if next > limit {
            return Err(BudgetError::Exhausted { scope });
        }
        if priority == RefreshPriority::Background
            && next > limit.saturating_sub(self.reserved_operator)
        {
            return Err(BudgetError::ReservedOperatorCapacity { scope });
        }
        Ok(())
    }
}

fn parse_max_header(headers: &HeaderMap, names: &[&str]) -> Result<Option<u64>, BudgetError> {
    let mut maximum: Option<u64> = None;
    for name in names {
        if let Some(value) = headers.get(*name) {
            let value = value
                .to_str()
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .ok_or(BudgetError::MalformedWeightHeader)?;
            maximum = Some(maximum.map_or(value, |current| current.max(value)));
        }
    }
    Ok(maximum)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BudgetError {
    #[error("Binance weight header is malformed")]
    MalformedWeightHeader,
    #[error("Binance 429/418 response is missing a valid Retry-After value")]
    MissingOrMalformedRetryAfter,
    #[error("Binance rate-limit circuit is open until {blocked_until_millis} ms")]
    CircuitOpen { blocked_until_millis: u64 },
    #[error("{scope:?} rate budget is reserved for an operator refresh")]
    ReservedOperatorCapacity { scope: WeightScope },
    #[error("{scope:?} rate budget is exhausted")]
    Exhausted { scope: WeightScope },
    #[error("Binance rate-budget arithmetic overflowed")]
    Overflow,
}

#[cfg(all(test, feature = "test-transport"))]
mod tests {
    use super::*;

    #[test]
    fn usage_recovers_only_after_a_full_idle_weight_window() {
        let clock = DeterministicRateClock::new(0);
        let mut budget = RateBudget::with_test_clock(10, 10, 2, clock.clone());

        budget
            .reserve(RequestWeight::new(8, 8), RefreshPriority::Background)
            .expect("background work may use non-reserved capacity");
        assert!(matches!(
            budget.reserve(RequestWeight::new(1, 1), RefreshPriority::Background),
            Err(BudgetError::ReservedOperatorCapacity { .. })
        ));

        clock.set_now_millis(59_000);
        let mut headers = HeaderMap::new();
        headers.insert("X-MBX-USED-WEIGHT-1M", "8".parse().expect("valid header"));
        headers.insert(
            "X-SAPI-USED-UID-WEIGHT-1M",
            "8".parse().expect("valid header"),
        );
        budget
            .observe_headers(&headers)
            .expect("authoritative usage refreshes the activity clock");

        clock.set_now_millis(60_000);
        assert!(matches!(
            budget.reserve(RequestWeight::new(1, 1), RefreshPriority::Background),
            Err(BudgetError::ReservedOperatorCapacity { .. })
        ));

        clock.set_now_millis(119_000);
        budget
            .reserve(RequestWeight::new(1, 1), RefreshPriority::Background)
            .expect("a full minute without weight activity safely expires local usage");
        assert_eq!(budget.used(WeightScope::Ip), 1);
        assert_eq!(budget.used(WeightScope::Uid), 1);
    }

    #[test]
    fn out_of_order_provider_usage_observations_never_regress_an_active_window() {
        let clock = DeterministicRateClock::new(1_000);
        let mut budget = RateBudget::with_test_clock(100, 100, 0, clock);
        let mut newer = HeaderMap::new();
        newer.insert("X-MBX-USED-WEIGHT-1M", "20".parse().expect("valid header"));
        newer.insert(
            "X-SAPI-USED-UID-WEIGHT-1M",
            "20".parse().expect("valid header"),
        );
        let mut delayed_older = HeaderMap::new();
        delayed_older.insert("X-MBX-USED-WEIGHT-1M", "10".parse().expect("valid header"));
        delayed_older.insert(
            "X-SAPI-USED-UID-WEIGHT-1M",
            "10".parse().expect("valid header"),
        );

        budget
            .observe_headers(&newer)
            .expect("newer response is valid");
        budget
            .observe_headers(&delayed_older)
            .expect("delayed response is valid");

        assert_eq!(budget.used(WeightScope::Ip), 20);
        assert_eq!(budget.used(WeightScope::Uid), 20);
    }

    #[test]
    fn malformed_throttle_metadata_still_opens_a_conservative_circuit() {
        let clock = DeterministicRateClock::new(1_000);
        let mut budget = RateBudget::with_test_clock(100, 100, 10, clock);

        assert_eq!(
            budget.observe_response(StatusCode::TOO_MANY_REQUESTS, &HeaderMap::new()),
            Err(BudgetError::MissingOrMalformedRetryAfter)
        );
        assert_eq!(budget.blocked_until_millis(), Some(61_000));
        assert!(matches!(
            budget.reserve(RequestWeight::new(1, 1), RefreshPriority::Operator),
            Err(BudgetError::CircuitOpen {
                blocked_until_millis: 61_000
            })
        ));
    }

    #[test]
    fn malformed_ban_metadata_uses_the_longer_conservative_fallback() {
        let clock = DeterministicRateClock::new(2_000);
        let mut budget = RateBudget::with_test_clock(100, 100, 10, clock.clone());

        assert_eq!(
            budget.observe_response(StatusCode::IM_A_TEAPOT, &HeaderMap::new()),
            Err(BudgetError::MissingOrMalformedRetryAfter)
        );
        assert_eq!(budget.blocked_until_millis(), Some(259_202_000));
        assert!(matches!(
            budget.reserve(RequestWeight::new(1, 1), RefreshPriority::Operator),
            Err(BudgetError::CircuitOpen {
                blocked_until_millis: 259_202_000
            })
        ));

        clock.set_now_millis(302_000);
        assert!(matches!(
            budget.reserve(RequestWeight::new(1, 1), RefreshPriority::Operator),
            Err(BudgetError::CircuitOpen {
                blocked_until_millis: 259_202_000
            })
        ));
    }
}
