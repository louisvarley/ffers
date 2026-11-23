//! Every hull a starport offers has a View page. Local-only: `FFE_WORLD_CACHE=<data.bin> cargo test
//! -p ffe-rs --test ship_view_stock -- --ignored --nocapture`.
use ffe_rs::navigation::{Navigator, DEFAULT_SHIP};

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn every_offered_hull_has_a_view_page() {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    for start in [0u8, 1, 2, 3] {
        let mut nav = Navigator::new_game(&cache, start).unwrap();
        let Some(station) = nav.docked else {
            continue;
        };
        let hulls = nav
            .station_offers(&cache, station)
            .map(|offers| offers.rows(DEFAULT_SHIP, None))
            .unwrap_or_default();
        let missing: Vec<_> = hulls
            .iter()
            .filter(|hull| ffe_rs::ship_view::page(&cache, hull.model, hull.seed).is_none())
            .map(|hull| (hull.name, hull.model))
            .collect();
        eprintln!(
            "start {start}: {} hulls, no page for {missing:?}",
            hulls.len()
        );
        assert!(missing.is_empty(), "start {start}: {missing:?}");
    }
}
