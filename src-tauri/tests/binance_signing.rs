use assetrail_lib::connectors::binance::ServerClock;

#[test]
fn server_clock_uses_median_offset_and_blocks_excess_uncertainty() {
    let mut clock = ServerClock::new(50);
    clock
        .observe(1_000, 1_510, 1_020)
        .expect("first bounded sample is accepted");
    clock
        .observe(2_000, 2_525, 2_050)
        .expect("second bounded sample is accepted");
    clock
        .observe(3_000, 3_530, 3_060)
        .expect("third bounded sample is accepted");

    assert_eq!(clock.offset_millis(), Some(500));
    assert_eq!(clock.uncertainty_millis(), Some(25));
    assert_eq!(clock.corrected_timestamp(4_000).unwrap(), 4_500);

    let mut uncertain = ServerClock::new(10);
    uncertain
        .observe(1_000, 1_510, 1_040)
        .expect("the sample is retained for diagnostics");
    assert!(uncertain.corrected_timestamp(2_000).is_err());
}

#[test]
fn server_clock_rejects_a_low_uncertainty_but_unsafe_absolute_offset() {
    let mut clock = ServerClock::new(10);
    clock
        .observe(1_000, 121_000, 1_000)
        .expect("the sample remains available for diagnostics");

    assert!(matches!(
        clock.corrected_timestamp(2_000),
        Err(assetrail_lib::connectors::binance::ClockError::OffsetExceeded { .. })
    ));
}
