use crate::systems::economy::RouteId;
use crate::world::index::{deterministic_rumor, RumorKind};

#[test]
fn deterministic_results_repeat() {
    let seed = 99;
    let route = RouteId(3);
    let first = deterministic_rumor(seed, route);
    let second = deterministic_rumor(seed, route);
    assert_eq!(first, second);
    assert!(matches!(
        first.0,
        RumorKind::Wind | RumorKind::Fog | RumorKind::Patrol
    ));
    assert!(first.1 <= 100);
}
