// The model against the published curve of the N55 (225 kW, 400 Nm) and the
// direction of every change. The stock boost map is back-calculated from that
// curve (see spec.rs), so the first tests check that the model stays
// consistent with it, not that it is true for a real car.

use caruspex_i18n::{Language, with_language};
use caruspex_tune::spec;
use caruspex_tune::{
    BandOffset, BoostCap, Conditions, Gearbox, Maps, POINTS, Severity, Simulation, WarningKind,
    curve, rpm_at, simulate,
};

fn all_bands(boost: f64, ignition: f64, lambda: f64) -> Maps {
    Maps::stock().with_bands(&[BandOffset {
        from_rpm: 1000.0,
        to_rpm: 7000.0,
        boost_bar: boost,
        ignition_deg: ignition,
        lambda,
    }])
}

fn has(sim: &Simulation, kind: WarningKind) -> bool {
    sim.warnings.iter().any(|w| w.kind == kind)
}

#[test]
fn stock_gives_400_nm_from_1250_to_5000_rpm_within_three_percent() {
    let points = curve(&Maps::stock(), &Conditions::default());
    // 1,250 is the first point of the 250 rpm grid at or above the
    // published 1,200 rpm.
    for p in points
        .iter()
        .filter(|p| p.rpm >= 1250.0 && p.rpm <= spec::RATED_TORQUE_TO_RPM)
    {
        let off = (p.torque_nm - spec::RATED_TORQUE_NM).abs() / spec::RATED_TORQUE_NM;
        assert!(off < 0.03, "{} rpm: {:.1} Nm", p.rpm, p.torque_nm);
    }
    // Below 1,200 the turbo has not spooled up yet.
    assert!(points[0].torque_nm < 0.85 * spec::RATED_TORQUE_NM);
}

#[test]
fn stock_gives_306_ps_between_5700_and_6000_rpm_within_three_percent() {
    let sim = simulate(&Maps::stock(), &Conditions::default());
    let (rpm, kw) = sim.peak_power;
    assert!(
        (spec::RATED_POWER_FROM_RPM..=spec::RATED_POWER_TO_RPM).contains(&rpm),
        "peak at {rpm} rpm"
    );
    assert!(
        (kw - spec::RATED_POWER_KW).abs() / spec::RATED_POWER_KW < 0.03,
        "{kw:.1} kW"
    );
    for p in sim
        .points
        .iter()
        .filter(|p| p.rpm >= 5750.0 && p.rpm <= 6000.0)
    {
        assert!(
            (p.power_ps - 306.0).abs() / 306.0 < 0.03,
            "{} rpm: {:.1} PS",
            p.rpm,
            p.power_ps
        );
    }
    // Peak torque stays at the published 400 Nm.
    assert!((sim.peak_torque.1 - 400.0).abs() < 8.0);
}

#[test]
fn stock_inner_values_are_in_a_plausible_range() {
    let points = curve(&Maps::stock(), &Conditions::default());
    for p in &points {
        assert!(
            (0.0..=1.0).contains(&p.boost_bar),
            "{} rpm boost {}",
            p.rpm,
            p.boost_bar
        );
        assert!(
            (20.0..=60.0).contains(&p.charge_air_c),
            "{} rpm charge air",
            p.rpm
        );
        assert!(p.egt_c < spec::EGT_LIMIT_C, "{} rpm egt", p.rpm);
        assert!(p.turbo_load < 0.9 && p.pump_load < 0.9 && p.injector_load < 0.9);
        // The stock map keeps its reserve at reference conditions.
        assert!((p.knock_margin_deg - spec::STOCK_KNOCK_RESERVE_DEG).abs() < 0.01);
        assert_eq!(p.boost_cap, BoostCap::None);
    }
    // Air at rated power: a petrol engine needs roughly 3.3 to 4.5 kg/h per kW.
    let p = &points[20];
    assert_eq!(p.rpm, 6000.0);
    let per_kw = p.air_mass_kg_h / p.power_kw;
    assert!((3.3..=4.5).contains(&per_kw), "{per_kw:.2} kg/h per kW");
}

#[test]
fn stock_at_reference_conditions_has_no_warning() {
    let sim = simulate(&Maps::stock(), &Conditions::default());
    assert!(sim.warnings.is_empty(), "{:?}", sim.warnings);
}

#[test]
fn more_boost_gives_more_torque_and_warns_about_pre_ignition_and_gearbox() {
    let stock = simulate(&Maps::stock(), &Conditions::default());
    let more = simulate(&all_bands(0.3, 0.0, -0.05), &Conditions::default());
    for (a, b) in stock.points.iter().zip(&more.points).skip(1) {
        assert!(b.torque_nm > a.torque_nm, "{} rpm", a.rpm);
        assert!(b.charge_air_c > a.charge_air_c);
    }
    assert!(more.peak_torque.1 > 450.0);
    assert!(has(&more, WarningKind::PreIgnition));
    assert!(has(&more, WarningKind::Gearbox));
}

#[test]
fn far_too_much_boost_hits_the_turbo_and_gains_little_at_high_rpm() {
    let a = simulate(&all_bands(0.6, 0.0, -0.05), &Conditions::default());
    let b = simulate(&all_bands(1.0, 0.0, -0.05), &Conditions::default());
    assert!(has(&a, WarningKind::TurboLimit));
    let at = |s: &Simulation| s.points[20].power_kw;
    // The turbo cannot deliver more: the higher target changes nothing at 6,000 rpm.
    assert!((at(&b) - at(&a)).abs() < 1.0);
    assert!(matches!(
        b.points[20].boost_cap,
        BoostCap::Flow | BoostCap::PressureRatio
    ));
    assert!(
        b.points
            .iter()
            .all(|p| p.pressure_ratio <= spec::TURBO_MAX_PRESSURE_RATIO + 1e-6)
    );
}

#[test]
fn earlier_ignition_than_the_knock_limit_is_pulled_back_and_warned() {
    let sim = simulate(&all_bands(0.0, 4.0, 0.0), &Conditions::default());
    assert!(has(&sim, WarningKind::KnockPulled));
    for p in &sim.points {
        assert!(p.timing_deg <= p.timing_requested_deg);
        assert!((p.timing_requested_deg - p.timing_deg - 2.5).abs() < 0.01);
    }
    // The reserve of 1.5 degrees is used and gives a little torque.
    let stock = simulate(&Maps::stock(), &Conditions::default());
    assert!(sim.peak_torque.1 > stock.peak_torque.1);
    assert!(sim.peak_torque.1 < stock.peak_torque.1 * 1.04);
}

#[test]
fn later_ignition_costs_torque_and_heats_the_exhaust() {
    let stock = curve(&Maps::stock(), &Conditions::default());
    let late = curve(&all_bands(0.0, -3.0, 0.0), &Conditions::default());
    for (a, b) in stock.iter().zip(&late) {
        assert!(b.torque_nm < a.torque_nm);
        assert!(b.egt_c > a.egt_c + 30.0);
    }
}

#[test]
fn ron_95_loses_power_and_ron_102_alone_gains_nothing() {
    let maps = Maps::stock();
    let ron = |octane| {
        simulate(
            &maps,
            &Conditions {
                octane,
                ..Conditions::default()
            },
        )
    };
    let r95 = ron(95.0);
    let r98 = ron(98.0);
    let r102 = ron(102.0);
    assert!(r95.peak_power.1 < r98.peak_power.1 * 0.98);
    assert!(has(&r95, WarningKind::KnockPulled));
    // The stock ignition map is the ceiling: better fuel only adds reserve.
    assert!((r102.peak_power.1 - r98.peak_power.1).abs() < 1e-9);
    assert!(r102.points.iter().all(|p| p.knock_margin_deg > 5.0));
}

#[test]
fn heat_and_altitude_cost_power_and_load_the_turbo() {
    let hot_high = Conditions {
        ambient_c: 35.0,
        ambient_hpa: 900.0,
        ..Conditions::default()
    };
    let stock = curve(&Maps::stock(), &Conditions::default());
    let worse = curve(&Maps::stock(), &hot_high);
    for (a, b) in stock.iter().zip(&worse).skip(1) {
        assert!(b.pressure_ratio > a.pressure_ratio);
        assert!(b.charge_air_c > a.charge_air_c);
        assert!(b.torque_nm < a.torque_nm);
    }
}

#[test]
fn bands_only_change_their_own_rpm_range() {
    let maps = Maps::stock().with_bands(&[BandOffset {
        from_rpm: 2000.0,
        to_rpm: 3500.0,
        boost_bar: 0.1,
        ignition_deg: 1.0,
        lambda: -0.02,
    }]);
    let stock = Maps::stock();
    for i in 0..POINTS {
        let rpm = rpm_at(i);
        let inside = (2000.0..3500.0).contains(&rpm);
        let d = maps.boost_bar[i] - stock.boost_bar[i];
        assert_eq!(d.abs() > 1e-9, inside, "{rpm}");
        assert_eq!(maps.ignition_offset_deg[i] != 0.0, inside);
    }
    // The last band includes the redline.
    let top = Maps::stock().with_bands(&[BandOffset {
        from_rpm: 6000.0,
        to_rpm: 7000.0,
        boost_bar: 0.1,
        ignition_deg: 0.0,
        lambda: 0.0,
    }]);
    assert!(top.boost_bar[POINTS - 1] > stock.boost_bar[POINTS - 1]);
}

#[test]
fn nonsense_input_is_clamped_and_never_gives_nan() {
    let mut maps = Maps::stock();
    maps.boost_bar[3] = f64::NAN;
    maps.boost_bar[4] = 50.0;
    maps.lambda[5] = 0.1;
    maps.ignition_offset_deg[6] = f64::INFINITY;
    let cond = Conditions {
        ambient_c: f64::NAN,
        ambient_hpa: -5.0,
        octane: 200.0,
        gearbox: Gearbox::Manual6,
    };
    let sim = simulate(&maps, &cond);
    for p in &sim.points {
        for v in [
            p.torque_nm,
            p.power_kw,
            p.boost_bar,
            p.charge_air_c,
            p.egt_c,
            p.knock_margin_deg,
        ] {
            assert!(v.is_finite(), "{} rpm", p.rpm);
        }
        assert!(p.torque_nm >= 0.0);
    }
}

#[test]
fn lean_and_very_rich_mixtures_are_warned() {
    let lean = simulate(&all_bands(0.0, 0.0, 0.08), &Conditions::default());
    assert!(has(&lean, WarningKind::Lean));
    let rich = simulate(&all_bands(0.0, 0.0, -0.12), &Conditions::default());
    assert!(has(&rich, WarningKind::Rich));
}

#[test]
fn manual_gearbox_has_its_own_reason() {
    let maps = all_bands(0.3, 0.0, 0.0);
    let manual = simulate(
        &maps,
        &Conditions {
            gearbox: Gearbox::Manual6,
            ..Conditions::default()
        },
    );
    let w = manual
        .warnings
        .iter()
        .find(|w| w.kind == WarningKind::Gearbox)
        .expect("gearbox warning");
    assert_eq!(w.severity, Severity::Warning);
    with_language(Language::English, || {
        assert!(w.reason().contains("clutch"), "{}", w.reason());
    });
}

#[test]
fn reasons_are_filled_in_both_languages() {
    let sim = simulate(&all_bands(0.8, 4.0, 0.06), &Conditions::default());
    assert!(sim.warnings.len() >= 5);
    for language in [Language::German, Language::English] {
        with_language(language, || {
            for w in &sim.warnings {
                let reason = w.reason();
                // No placeholder left over (a percent sign only as a unit).
                for n in 1..=3 {
                    assert!(!reason.contains(&format!("%{n}")), "{reason}");
                }
                assert!(!w.title().is_empty());
                assert!(w.from_rpm <= w.to_rpm);
            }
        });
    }
    // Most serious first.
    assert!(
        sim.warnings
            .windows(2)
            .all(|p| p[0].severity >= p[1].severity)
    );
}

#[test]
fn a_whole_curve_takes_well_under_a_millisecond() {
    let maps = all_bands(0.2, 1.0, -0.02);
    let cond = Conditions::default();
    // Warm up, then measure many runs. A debug build is far slower than the
    // release build of the app and still has to pass.
    let _ = simulate(&maps, &cond);
    let runs = 200;
    let start = std::time::Instant::now();
    for _ in 0..runs {
        std::hint::black_box(simulate(std::hint::black_box(&maps), &cond));
    }
    let each = start.elapsed() / runs;
    // Shown with --nocapture; docs/site quotes the release number.
    println!("simulate: {each:?} per curve");
    assert!(each.as_micros() < 1000, "{each:?} per curve");
}

#[test]
fn a_value_hovering_at_a_limit_gives_one_warning_per_stretch() {
    // About 450 Nm from 2,000 to 5,000 rpm: right at the 8HP45 rating.
    let sim = simulate(&all_bands(0.2, 0.0, 0.0), &Conditions::default());
    let gearbox: Vec<_> = sim
        .warnings
        .iter()
        .filter(|w| matches!(w.kind, WarningKind::Gearbox | WarningKind::GearboxNear))
        .collect();
    assert_eq!(gearbox.len(), 1, "{gearbox:?}");
    let knock: Vec<_> = sim
        .warnings
        .iter()
        .filter(|w| matches!(w.kind, WarningKind::KnockPulled | WarningKind::KnockReserve))
        .collect();
    assert_eq!(knock.len(), 1, "{knock:?}");
    assert_eq!(knock[0].kind, WarningKind::KnockPulled);
    with_language(Language::English, || {
        // The degrees past the limit, as a positive number.
        assert!(
            knock[0].reason().contains("up to 0."),
            "{}",
            knock[0].reason()
        );
    });
}
