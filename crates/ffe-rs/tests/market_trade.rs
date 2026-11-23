//! Local-only: a station trade against the executed original (release-1).
//!
//! `tools/reference/verify-trade.py` runs `jjffe-smoke --trade 220` on the real
//! Lave docked game and writes its JSONL to `FFE_TRADE_TRACE`. At frame 200 the
//! original's F555 (buy) and F556 (sell) run on fixed market entries, cash, hold
//! and stock: ten cases covering the success path and all five refusals
//! (`0x98e1`..`0x98e5`). This test rebuilds the same market and hold and replays
//! every case through `ffe_core::market::Market::{buy,sell}`, requiring the
//! cash, cargo word, hold used, station stock and message code to match.
//!
//! ```text
//! FFE_TRADE_TRACE=trade.jsonl cargo test -p ffe-rs --test market_trade \
//!     -- --ignored --nocapture
//! ```
use ffe_core::market::{Entry, Hold, Market, Refusal, GOODS};
use serde_json::Value;
use std::collections::BTreeSet;

fn rows() -> Vec<Value> {
    let path = std::env::var("FFE_TRADE_TRACE").expect("FFE_TRADE_TRACE");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("a JSONL row"))
        .collect()
}

fn int(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("not an integer: {value}"))
}

fn code(refusal: Refusal) -> i64 {
    match refusal {
        Refusal::NoCash => 0x98e1,
        Refusal::CannotPayToSell => 0x98e2,
        Refusal::HoldFull => 0x98e3,
        Refusal::OutOfStock => 0x98e4,
        Refusal::NothingToSell => 0x98e5,
    }
}

#[test]
#[ignore = "Local-only: needs the reference trace in FFE_TRADE_TRACE"]
fn the_originals_buy_and_sell_match_the_port() {
    let rows = rows();
    let table = rows
        .iter()
        .find_map(|row| row.get("trade_table"))
        .expect("a trade_table row");
    let stock: Vec<i32> = table["stock"]
        .as_array()
        .expect("stock array")
        .iter()
        .map(|value| int(value) as i32)
        .collect();
    let price: Vec<i32> = table["price"]
        .as_array()
        .expect("price array")
        .iter()
        .map(|value| int(value) as i32)
        .collect();
    assert_eq!(stock.len(), GOODS);
    assert_eq!(price.len(), GOODS);

    let cases: Vec<&Value> = rows.iter().filter_map(|row| row.get("trade")).collect();
    assert_eq!(cases.len(), 10, "the ten scripted trades");

    let mut codes = BTreeSet::new();
    for case in &cases {
        let good = int(&case["good"]) as usize;
        let count = int(&case["count"]) as u32;
        let buying = case["op"].as_str().unwrap() == "buy";
        // F538's entries, then F554's selected good's price and stock.
        let mut entries = [Entry::default(); GOODS];
        for (index, entry) in entries.iter_mut().enumerate() {
            entry.stock = stock[index];
            entry.price = price[index];
        }
        entries[good].stock = int(&case["stock"][0]) as i32;
        entries[good].price = int(&case["price"]) as i32;
        let mut market = Market { entries };
        let capacity = int(&case["capacity"]) as i32;
        let mut hold = Hold::new(capacity);
        hold.used = int(&case["used"][0]) as i32;
        hold.cargo[good] = int(&case["cargo"][0]) as u16;
        let mut cash = int(&case["cash"][0]) as i32;
        // F555/F556 loop over the requested tonnes and stop at the first refusal.
        let mut refusal = None;
        for _ in 0..count {
            let result = if buying {
                market.buy(good, &mut cash, &mut hold)
            } else {
                market.sell(good, &mut cash, &mut hold)
            };
            if let Err(why) = result {
                refusal = Some(why);
                break;
            }
        }
        // F556 writes 0x98e5 into the row before its loop and leaves it there on
        // success, so a sale's 0x98e5 means either "sold" or "nothing to sell";
        // the state below tells them apart. F555 leaves 0 on success.
        let message = match (buying, refusal) {
            (true, None) => 0,
            (true, Some(why)) => code(why),
            (false, Some(Refusal::CannotPayToSell)) => 0x98e2,
            (false, _) => 0x98e5,
        };
        let case_number = int(&case["case"]);
        assert_eq!(
            message,
            int(&case["code"]),
            "case {case_number}: F557's message"
        );
        assert_eq!(
            cash,
            int(&case["cash"][1]) as i32,
            "case {case_number}: cash"
        );
        assert_eq!(
            hold.used,
            int(&case["used"][1]) as i32,
            "case {case_number}: hold used"
        );
        assert_eq!(
            hold.cargo[good],
            int(&case["cargo"][1]) as u16,
            "case {case_number}: the cargo word"
        );
        assert_eq!(
            market.entries[good].stock,
            int(&case["stock"][1]) as i32,
            "case {case_number}: the station's stock"
        );
        codes.insert(message);
    }
    // Every refusal and the success path are in the set, so the trace cannot be
    // trimmed to the cases that happen to agree.
    assert_eq!(
        codes,
        BTreeSet::from([0, 0x98e1, 0x98e2, 0x98e3, 0x98e4, 0x98e5]),
        "the five refusals and the success path"
    );
    eprintln!("10 trades: cash/hold/cargo/stock and all five refusals match");
}
