use crate::systems::economy::{DetRng, EconomyDay, HubId};

#[test]
fn deterministic_seed_reproducible() {
    let hub = HubId(7);
    let day = EconomyDay(3);
    let mut rng_a = DetRng::from_seed(42, 1, hub, day, 99);
    let mut rng_b = DetRng::from_seed(42, 1, hub, day, 99);
    let mut rng_c = DetRng::from_seed(42, 1, hub, EconomyDay(4), 99);

    let seq_a: Vec<u32> = (0..4).map(|_| rng_a.u32()).collect();
    let seq_b: Vec<u32> = (0..4).map(|_| rng_b.u32()).collect();
    let seq_c: Vec<u32> = (0..4).map(|_| rng_c.u32()).collect();

    assert_eq!(seq_a, seq_b);
    assert_ne!(seq_a, seq_c);
}

#[test]
fn norm_samples_clamped_and_stable() {
    let mut rng = DetRng::from_seed(7, 2, HubId(3), EconomyDay(55), 0);
    let samples: Vec<i32> = (0..5)
        .map(|_| rng.norm_bounded_bp(50, 400, 900).0)
        .collect();
    assert_eq!(samples, vec![-83, -703, 4, 260, 6]);

    let mut rng = DetRng::from_seed(9, 1, HubId(1), EconomyDay(1), 1);
    for _ in 0..256 {
        let sample = rng.norm_bounded_bp(-25, 1200, 600);
        assert!(sample.0 >= -600 && sample.0 <= 600);
    }
}
