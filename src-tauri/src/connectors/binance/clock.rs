use std::collections::VecDeque;

const MAX_SAMPLES: usize = 9;
const MAX_ABSOLUTE_OFFSET_MILLIS: u64 = 60_000;

/// Bounded rolling estimate of Binance server-clock offset.
#[derive(Clone, Debug)]
pub struct ServerClock {
    max_uncertainty_millis: u64,
    samples: VecDeque<ClockSample>,
}

#[derive(Clone, Copy, Debug)]
struct ClockSample {
    offset_millis: i64,
    uncertainty_millis: u64,
}

impl ServerClock {
    #[must_use]
    pub fn new(max_uncertainty_millis: u64) -> Self {
        Self {
            max_uncertainty_millis,
            samples: VecDeque::with_capacity(MAX_SAMPLES),
        }
    }

    /// Records a server-time observation bounded by local send/receive times.
    ///
    /// # Errors
    /// Returns an error for reversed local times or an offset outside `i64`.
    pub fn observe(
        &mut self,
        local_sent_millis: u64,
        server_millis: u64,
        local_received_millis: u64,
    ) -> Result<(), ClockError> {
        let round_trip = local_received_millis
            .checked_sub(local_sent_millis)
            .ok_or(ClockError::ReversedObservation)?;
        let midpoint = u128::from(local_sent_millis) + u128::from(round_trip / 2);
        let offset = i128::from(server_millis)
            - i128::try_from(midpoint).map_err(|_| ClockError::OffsetOutOfRange)?;
        let offset_millis = i64::try_from(offset).map_err(|_| ClockError::OffsetOutOfRange)?;
        let uncertainty_millis = round_trip.saturating_add(1) / 2;

        if self.samples.len() == MAX_SAMPLES {
            self.samples.pop_front();
        }
        self.samples.push_back(ClockSample {
            offset_millis,
            uncertainty_millis,
        });
        Ok(())
    }

    #[must_use]
    pub fn offset_millis(&self) -> Option<i64> {
        median(self.samples.iter().map(|sample| sample.offset_millis))
    }

    #[must_use]
    pub fn uncertainty_millis(&self) -> Option<u64> {
        median(self.samples.iter().map(|sample| sample.uncertainty_millis))
    }

    /// Applies the rolling median offset when uncertainty remains within policy.
    ///
    /// # Errors
    /// Returns an error until sampled, when uncertainty exceeds policy, or when
    /// the corrected timestamp would leave the `u64` range.
    pub fn corrected_timestamp(&self, local_millis: u64) -> Result<u64, ClockError> {
        let offset = self.offset_millis().ok_or(ClockError::Unsynchronized)?;
        let uncertainty = self
            .uncertainty_millis()
            .ok_or(ClockError::Unsynchronized)?;
        if uncertainty > self.max_uncertainty_millis {
            return Err(ClockError::UncertaintyExceeded {
                observed_millis: uncertainty,
                maximum_millis: self.max_uncertainty_millis,
            });
        }
        if offset.unsigned_abs() > MAX_ABSOLUTE_OFFSET_MILLIS {
            return Err(ClockError::OffsetExceeded {
                observed_millis: offset.unsigned_abs(),
                maximum_millis: MAX_ABSOLUTE_OFFSET_MILLIS,
            });
        }

        let corrected = i128::from(local_millis) + i128::from(offset);
        u64::try_from(corrected).map_err(|_| ClockError::TimestampOutOfRange)
    }
}

fn median<Value>(values: impl Iterator<Item = Value>) -> Option<Value>
where
    Value: Copy + Ord,
{
    let mut values: Vec<_> = values.collect();
    values.sort_unstable();
    values.get(values.len() / 2).copied()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ClockError {
    #[error("server-clock observation received before it was sent")]
    ReversedObservation,
    #[error("server-clock offset is outside the supported range")]
    OffsetOutOfRange,
    #[error("server clock has not been synchronized")]
    Unsynchronized,
    #[error("server-clock uncertainty {observed_millis} ms exceeds the {maximum_millis} ms policy")]
    UncertaintyExceeded {
        observed_millis: u64,
        maximum_millis: u64,
    },
    #[error("server-clock offset {observed_millis} ms exceeds the {maximum_millis} ms policy")]
    OffsetExceeded {
        observed_millis: u64,
        maximum_millis: u64,
    },
    #[error("corrected server timestamp is outside the supported range")]
    TimestampOutOfRange,
}
