//! The mission slice's scripted run: an offer is accepted, the contract is
//! settled at its destination, a late one is failed, and the whole state
//! round-trips through the save format. The cache-gated test generates a real
//! BBS post and reads it back through `Navigator::mission_offer`.
use ffe_core::bulletin::{Destination, Post, Var, World};
use ffe_core::missions::{self, Kind, PageKind, Status};
use ffe_rs::flight::Flight;
use ffe_rs::navigation::Navigator;

fn world<'a>(destinations: &'a [Destination]) -> World<'a> {
    World {
        code: 1,
        name: "Ross 154",
        population: 9,
        danger: 0,
        allegiance: 2,
        station_name: "Sirocco Station",
        destinations,
        names: None,
    }
}

fn passenger_post(money: i64, destination: u32) -> Post {
    Post {
        code: 0x981a,
        fake: false,
        day: 0,
        date: 0,
        vars: [
            Var::Int(money),
            Var::Text("passage for two people on a fast ship".to_owned()),
            Var::System(destination),
            Var::Seed(77),
            // Two passengers, additive 3, no package bit.
            Var::Int((3 << 9) | 1),
        ],
        text: "WANTED: passage".to_owned(),
    }
}

#[test]
fn a_scripted_passenger_contract_pays_and_journals() {
    let destinations = [Destination {
        code: 0x1234_5678,
        name: "Sol".to_owned(),
        population: 12,
        danger: 1,
    }];
    let offer = missions::offer(&passenger_post(1500, 0x1234_5678), &world(&destinations))
        .expect("a contract offer");
    assert_eq!(offer.kind, Kind::Passenger);
    assert_eq!(offer.passengers, 2);

    let mut state = missions::State::default();
    let mut cash_tenths = 1000u32;
    let accepted = state.accept(&offer, 4, 1, "Ross 154").unwrap();
    assert_eq!(
        accepted.text,
        "Excellent. $1500 will be paid on arrival at the Sol system."
    );
    assert_eq!(state.journal.len(), 1);
    assert_eq!(state.pages.len(), 1);

    // Arrival at Sol three days later: F303 pays the whole-credit reward into
    // the tenths cash field and writes the "thanks for bringing us" page.
    for outcome in state.tick(7, 0x1234_5678) {
        cash_tenths = cash_tenths.saturating_add(outcome.reward.saturating_mul(10));
        assert_eq!(outcome.status, Status::Complete);
    }
    assert_eq!(cash_tenths, 1000 + 1500 * 10);
    assert_eq!(state.reputation, 1);
    assert_eq!(state.missions[0].status, Status::Complete);
    assert!(state
        .journal
        .iter()
        .any(|entry| entry.text.contains("Thanks for bringing us here")));
    assert_eq!(state.pages.len(), 2, "acceptance and completion pages");
    assert_eq!(state.pages[1].kind, PageKind::Message);
}

#[test]
fn a_thirtieth_credit_passage_refuses_payment_on_arrival() {
    let destinations = [Destination {
        code: 0x1234_5678,
        name: "Sol".to_owned(),
        population: 12,
        danger: 1,
    }];
    // F638's additive index 10 is the `$30` tier; F303's first branch pays
    // nothing for it.
    let mut post = passenger_post(1500, 0x1234_5678);
    post.vars[4] = Var::Int((10 << 9) | 1);
    let offer = missions::offer(&post, &world(&destinations)).unwrap();
    let mut state = missions::State::default();
    let id = state.accept(&offer, 4, 1, "Ross 154").unwrap().id;
    let outcome = state.tick(7, 0x1234_5678);
    assert_eq!(outcome.len(), 1);
    assert_eq!(outcome[0].id, id);
    assert_eq!(outcome[0].status, Status::Failed);
    assert_eq!(outcome[0].reward, 0);
    assert_eq!(state.reputation, 0);
    assert!(outcome[0]
        .journal
        .text
        .contains("I can't pay at the moment"));
}

#[test]
fn a_late_contract_fails_at_its_destination_and_an_abandoned_one_loses_reputation() {
    let destinations = [Destination {
        code: 0x1234_5678,
        name: "Sol".to_owned(),
        population: 12,
        danger: 1,
    }];
    let mut state = missions::State::default();
    let offer = missions::offer(&passenger_post(1500, 0x1234_5678), &world(&destinations)).unwrap();
    state.accept(&offer, 0, 1, "Ross 154").unwrap();
    state.reputation = 3;
    // The deadline is accept day + 0x1c. F302 fails the overdue contract
    // before it checks the current system, so arriving at Sol too late is a
    // failure, not a 0x9c2c settlement.
    let outcomes = state.tick(missions::ACCEPT_DAYS + 1, 0x1234_5678);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0].status, Status::Failed);
    assert_eq!(outcomes[0].reward, 0);
    assert_eq!(state.reputation, 1, "F302's -2");
    assert!(outcomes[0]
        .journal
        .text
        .contains("reporting you to the authorities"));
    assert!(state
        .pages
        .iter()
        .any(|page| page.text.contains("waited a month")));

    let mut state = missions::State::default();
    state.accept(&offer, 0, 1, "Ross 154").unwrap();
    state.reputation = 4;
    let outcome = state.abandon(1, 1).unwrap();
    assert_eq!(outcome.status, Status::Abandoned);
    assert_eq!(state.reputation, 3);
    assert!(outcome
        .journal
        .text
        .contains("reporting you to the authorities"));
}

#[test]
fn the_flight_save_keeps_contracts_and_old_saves_default_them() {
    let destinations = [Destination {
        code: 0x1234_5678,
        name: "Sol".to_owned(),
        population: 12,
        danger: 1,
    }];
    let mut flight = Flight::default();
    let offer = missions::offer(&passenger_post(1500, 0x1234_5678), &world(&destinations)).unwrap();
    flight.missions.accept(&offer, 2, 1, "Ross 154").unwrap();
    let json = ffe_rs::saves::encode(&flight).unwrap();
    let loaded = ffe_rs::saves::decode(&json).unwrap();
    assert_eq!(loaded.missions, flight.missions);
    assert_eq!(loaded.missions.missions.len(), 1);

    // A save from before the mission slice has no `missions` member; the
    // `#[serde(default)]` on the field gives it an empty state.
    let old = serde_json::json!({
        "format": "ffe-rs-experimental",
        "version": 1,
        "state": {
            "mode": "Docked",
            "position": [0.0, 0.0, -60.0],
            "velocity": [0.0, 0.0, 0.0],
            "yaw": 0.0,
            "pitch": 0.0,
            "credits": 1000,
            "cargo": 0,
            "stock": 50,
            "ticks": 0,
            "docking_requested": false
        }
    });
    let old = serde_json::to_vec(&old).unwrap();
    let loaded = ffe_rs::saves::decode(&old).unwrap();
    assert!(loaded.missions.missions.is_empty());
    assert!(loaded.missions.journal.is_empty());
}

/// Local-only: the military package list is a source-derived offer, and its
/// completion produces F307's medal page (song 10) and F306's rank page
/// (song 7).
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_military_package_offer_is_reachable_and_completes() {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let nav = Navigator::new(&cache).unwrap();
    let station = nav.docked.expect("docked at the start");
    let offers = nav
        .military_offers(&cache, station, 0)
        .expect("F493's list");
    assert_eq!(offers.len(), 1, "only the first tier at rank 0");
    let offer = offers[0].clone();
    assert!(offer.text.contains("Deliver"));
    assert!(offer.reward >= 150);
    let mut state = missions::State::default();
    state
        .accept(
            &offer,
            nav.day(),
            nav.current_system_code(),
            nav.system_name(),
        )
        .unwrap();
    // Jump the clock past the deadline? No: settle at the destination in time.
    let outcome = state
        .settle(1, nav.day() + 1, offer.destination)
        .expect("the delivery completes");
    assert_eq!(outcome.status, Status::Complete);
    assert_eq!(outcome.reward, offer.reward);
    let rank = state
        .pages
        .iter()
        .find(|page| page.kind == PageKind::Rank)
        .expect("F306's rank page");
    assert!(rank.text.contains("promotion"));
    assert!(
        !state.pages.iter().any(|page| page.kind == PageKind::Medal),
        "a package delivery has no medal (DATA_005115[0] is 0xff)"
    );
}

/// Local-only: F508/F495/F496/F506 — selecting a military offer opens its
/// briefing (the description and suggested responses) and records nothing
/// until the accept row is answered.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_military_offer_opens_its_briefing_before_it_is_accepted() {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let nav = Navigator::new(&cache).unwrap();
    let station = nav.docked.expect("docked at the start");
    let day = nav.day();
    let offer = nav
        .military_offers(&cache, station, 0x600)
        .expect("F493's list")
        .into_iter()
        .next()
        .expect("an offer");
    assert_eq!(offer.code, 0x9854, "the package tier is first at rank 0");
    let briefing = nav
        .military_briefing(&cache, station, &offer, day)
        .expect("F495/F496/F508's page");
    assert_eq!(briefing.codes(), missions::briefing_codes(offer.code));
    assert_eq!(briefing.description, offer.text);
    assert!(!briefing.place.is_empty(), "F466's destination place");
    assert!(!briefing.registration.is_empty(), "F791 kind 0x12");

    let mut state = missions::State::default();
    let mut briefing = briefing;
    assert_eq!(briefing.respond(0x9859), missions::BriefingStep::Answer);
    assert_eq!(
        briefing.page,
        "It is our policy to offer a fixed payment on completion of the mission only."
    );
    // F513's "Show mission description".
    assert_eq!(
        briefing.respond(0x985a),
        missions::BriefingStep::Description
    );
    assert_eq!(briefing.page, offer.text);
    // Nothing has been accepted yet.
    assert!(state.missions.is_empty());
    // F509's accept row.
    assert_eq!(briefing.respond(0x9858), missions::BriefingStep::Accept);
    let accepted = state
        .accept(
            &briefing.offer,
            day,
            nav.current_system_code(),
            nav.system_name(),
        )
        .unwrap();
    assert!(accepted.text.contains("will be paid on arrival"));
    assert_eq!(state.missions.len(), 1);
    // F342/F348: only "Return to Mission List" remains.
    briefing.closed = true;
    assert_eq!(briefing.codes(), [0x9860]);
}

/// Local-only: F496's photograph/bombing briefing uses F503's destination
/// surface record (model 0x77..0x7a), not F466's starport, and F470's ship is
/// F658/F635's kind 0x10 model from the offer's seed.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_photograph_briefing_uses_f503s_surface_record() {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let nav = Navigator::new(&cache).unwrap();
    let station = nav.docked.expect("docked at the start");
    let day = nav.day();
    let offer = nav
        .military_offers(&cache, station, 0x600)
        .expect("F493's list")
        .into_iter()
        .find(|offer| matches!(offer.code, 0x9855 | 0x9856))
        .expect("a photograph or bombing offer");
    let surface = nav
        .surface_place(&cache, offer.destination, offer.seed)
        .expect("F503's destination surface");
    assert!(!surface.is_empty());
    let briefing = nav
        .military_briefing(&cache, station, &offer, day)
        .expect("F496's page");
    assert_eq!(briefing.place, surface);
    assert!(!briefing.ship.is_empty(), "F470's ship");
    // F470's answer prints the ship and the surface record.
    let mut briefing = briefing;
    assert_eq!(briefing.respond(0x9865), missions::BriefingStep::Answer);
    assert!(briefing.page.contains(&surface), "{}", briefing.page);
}

/// Local-only: F521's reply roll uses the shared stream and refuses a
/// negative reputation with F520's 0x991e page. The port draws the same
/// `F922(9)` when a passenger or parcel advertisement is selected.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_negative_reputation_refuses_the_passenger_reply() {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let mut nav = Navigator::new(&cache).unwrap();
    let station = nav.docked.expect("docked at the start");
    let posts = nav
        .station_offers(&cache, station)
        .expect("F622's BBS")
        .posts
        .clone();
    let (row, post) = posts
        .iter()
        .enumerate()
        .find(|(_, post)| matches!(post.code, 0x981a | 0x981b))
        .expect("a passenger or parcel offer on the board");
    assert!(!post.text.is_empty());
    let offer = nav
        .mission_offer(&cache, station, row)
        .expect("the offer reads")
        .expect("a contract");
    let draw = nav.offer_reply_draw();
    assert!(
        missions::f521_refuses(-9, draw),
        "F922(9) = {draw} refuses at reputation -9"
    );
    assert!(!missions::f521_refuses(0, draw));
    let text = missions::refusal_text(&offer, nav.current_system_code(), nav.system_name());
    assert!(
        text.contains("won't deal with someone of your reputation"),
        "{text}"
    );
}

/// Local-only: the generated board must contain a contract offer that reads
/// back with its destination/reward/deadline fields from the real tables.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_generated_bbs_post_becomes_a_contract() {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let mut nav = Navigator::new(&cache).unwrap();
    let station = nav.docked.expect("docked at the start");
    let posts = nav
        .station_offers(&cache, station)
        .expect("F622's BBS")
        .posts
        .clone();
    let (row, _) = posts
        .iter()
        .enumerate()
        .find(|(_, post)| matches!(post.code, 0x981a | 0x981b | 0x981f..=0x9822))
        .expect("a contract offer on the board");
    let offer = nav
        .mission_offer(&cache, station, row)
        .expect("the offer reads")
        .expect("a contract");
    assert!(offer.reward > 0, "{offer:?}");
    assert_ne!(offer.destination_name, "");
    let mut state = missions::State::default();
    let accepted = state
        .accept(
            &offer,
            nav.day(),
            nav.current_system_code(),
            nav.system_name(),
        )
        .unwrap();
    assert!(accepted.text.contains(&offer.destination_name));
    // F535 clears the advertisement; the board shrinks and the row is gone.
    let before = posts.len();
    nav.clear_post(&cache, station, row).unwrap();
    let after = nav.station_offers(&cache, station).unwrap().posts.len();
    assert_eq!(after, before - 1);
}

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_generated_passenger_post_runs_f521s_reply_page() {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let mut nav = Navigator::new(&cache).unwrap();
    let station = nav.docked.expect("docked at the start");
    let posts = nav
        .station_offers(&cache, station)
        .expect("F622's BBS")
        .posts
        .clone();
    let (row, _) = posts
        .iter()
        .enumerate()
        .find(|(_, post)| matches!(post.code, 0x981a | 0x981b))
        .expect("a passenger/parcel offer on the board");
    let offer = nav
        .mission_offer(&cache, station, row)
        .expect("the offer reads")
        .expect("a contract");
    let system = nav.system_name().to_owned();
    let code = nav.current_system_code();
    let day = nav.day();

    // F521: the intro page and F522's response list, on the real post's words.
    let mut reply = missions::Negotiation::open(&offer, row, false);
    let intro = reply.intro(code, &system);
    assert!(intro.contains(&offer.destination_name), "{intro}");
    assert!(intro.contains(&offer.client), "{intro}");
    let expected = if offer.kind == Kind::Parcel { 12 } else { 13 };
    assert_eq!(reply.codes().len(), expected, "{:?}", reply.codes());
    let rows = reply.rows(code, &system);
    assert_eq!(rows[0].text, "\"OK - agreed.\"");
    assert_eq!(rows.last().unwrap().text, "Hang up.");

    // F530's ten per cent and F531's half-now terms, then F534.
    let mut state = missions::State {
        reputation: missions::MAX_REPUTATION,
        ..missions::State::default()
    };
    match reply.respond(0x983a, state.reputation, code, &system) {
        missions::ReplyStep::Answer(text) => assert!(text.contains("extra 10%"), "{text}"),
        other => panic!("{other:?}"),
    }
    assert_eq!(reply.money, offer.reward + offer.reward / 10);
    match reply.respond(0x983b, 1, code, &system) {
        missions::ReplyStep::Answer(text) => {
            assert!(text.contains("half once you agree"), "{text}")
        }
        other => panic!("{other:?}"),
    }
    let (negotiated, advance, _) = reply.acceptance();
    assert_eq!(advance, reply.money / 2);
    assert_eq!(negotiated.reward, reply.money / 2);
    let page = reply.acceptance_text(code, &system);
    // The half-now page (0x992b) names the halved amount twice.
    assert!(page.contains(&format!("${advance}")), "{page}");
    // F534's cabin check: give the ship exactly the cabins the contract needs.
    let cabins = u32::from(negotiated.passengers) + 1;
    assert_eq!(state.cabin_refusal(&negotiated, cabins), None);
    state
        .accept_page(&negotiated, day, code, &system, page.clone())
        .unwrap();
    assert!(state.active().any(|m| m.reward == negotiated.reward));
    // F535 removes the advert from the real board.
    let before = posts.len();
    nav.clear_post(&cache, station, row).unwrap();
    assert_eq!(
        nav.station_offers(&cache, station).unwrap().posts.len(),
        before - 1
    );
}

/// Local-only: F460's removal conversation opens its six-row page, refuses a
/// commander whose Elite score is below the source's gate, and prints F467's
/// 0x99ca acceptance page. The destination place and the F470 ship come from
/// the real tables.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_removal_advert_opens_f460s_conversation() {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let mut nav = Navigator::new(&cache).unwrap();
    let station = nav.docked.expect("docked at the start");
    let offer = missions::Offer {
        kind: Kind::Assassination,
        code: 0x981f,
        destination: nav.current_system_code(),
        destination_name: nav.system_name().to_owned(),
        reward: 5000,
        passengers: 0,
        cargo: "double crossing spy".to_owned(),
        seed: 0x1234,
        basic_data: 0,
        deadline_days: 7,
        text: String::new(),
        client: String::new(),
        side: 0,
    };
    let day = nav.day();

    // F460/F459: a fresh commander (Elite 0) is refused the contract.
    nav.career.elite = 0;
    let refused = nav
        .removal_briefing(&cache, station, &offer, day, day + 7, 0)
        .expect("the refusal page");
    assert_eq!(refused.codes(), [0x983f]);
    assert!(refused.page.contains("combat rating"), "{}", refused.page);
    assert_eq!(refused.row, 0);

    // After some combat kills the same advert opens the conversation.
    nav.career.elite = 0x7fff_ffff;
    let mut briefing = nav
        .removal_briefing(&cache, station, &offer, day, day + 7, 0)
        .expect("the conversation");
    assert_eq!(missions::REMOVAL_CODES.len(), 6);
    assert_eq!(briefing.codes(), missions::REMOVAL_CODES);
    assert!(briefing.is_removal());
    assert_eq!(briefing.screen_title(), "Video Link");
    assert_ne!(briefing.place, "", "F466's destination place");
    assert_ne!(briefing.ship, "", "F470's F635(0x10) ship");
    assert_ne!(briefing.registration, "", "F791 kind 0x12 registration");
    let rows = briefing.rows();
    assert!(
        rows[1].text.contains("double crossing spy"),
        "{}",
        rows[1].text
    );
    // F469's answer names F466's place in the destination system.
    assert_eq!(briefing.respond(0x9862), missions::BriefingStep::Answer);
    assert!(briefing.page.contains(&briefing.place), "{}", briefing.page);
    // F470's answer names the ship and registration.
    assert_eq!(briefing.respond(0x9863), missions::BriefingStep::Answer);
    assert!(briefing.page.contains(&briefing.ship), "{}", briefing.page);
    assert!(
        briefing.page.contains(&briefing.registration),
        "{}",
        briefing.page
    );
    // F467: accept, record the port's contract and print 0x99ca.
    assert_eq!(briefing.respond(0x9861), missions::BriefingStep::Accept);
    let page =
        missions::removal_accepted_text(&offer, nav.current_system_code(), nav.system_name());
    assert!(page.contains("succesful return"), "{page}");
    let mut state = missions::State::default();
    state
        .accept_page(
            &offer,
            day,
            nav.current_system_code(),
            nav.system_name(),
            page.clone(),
        )
        .expect("the removal contract");
    assert!(state
        .active()
        .any(|mission| mission.kind == Kind::Assassination));
    // F535 clears the advertisement.
    let before = nav.station_offers(&cache, station).unwrap().posts.len();
    nav.clear_post(&cache, station, 0).unwrap();
    assert_eq!(
        nav.station_offers(&cache, station).unwrap().posts.len(),
        before - 1
    );
}
