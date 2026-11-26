//! Buying and selling at a station in the playable: the original's markets
//! (`ffe_core::market`) against the player's cash (`Flight::credits`, the
//! original's units: 1000 is 100.0 credits) and hold (`Flight::hold`).
use crate::flight::Flight;
use crate::navigation::Navigator;
use ffe_content::VerifiedCache;
use ffe_core::market::{Hold, GOODS, ILLEGAL, NAMES};
use ffe_core::npc::{self, OffenceReport};

/// One line of the market screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    pub good: usize,
    pub name: String,
    pub price: i32,
    pub stock: i32,
    pub held: u16,
}

/// `enhancement-19`: commodity `good`'s galactic average price, the price F619
/// gives with no supply variance in a system of danger 0 (whose multiplier,
/// one half, cancels F619's doubling): the commodity's base price. Local supply,
/// legality and danger move a market's price off it.
pub fn galactic_average(tables: &ffe_core::market::Tables<'_>, good: usize) -> i32 {
    tables.good(good).base_price
}

/// `enhancement-19`: how a price compares with the galactic average.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    /// More than [`TREND_BAND`] below it: cheap here.
    Low,
    /// Within the band.
    Average,
    /// More than the band above it: dear here.
    High,
}

/// The share either side of the average that still counts as average.
pub const TREND_BAND: f64 = 0.10;

/// Where `price` lies against `average` (both in tenths of a credit).
pub fn trend(price: i32, average: i32) -> Trend {
    if average <= 0 {
        return Trend::Average;
    }
    let ratio = f64::from(price) / f64::from(average);
    if ratio < 1.0 - TREND_BAND {
        Trend::Low
    } else if ratio > 1.0 + TREND_BAND {
        Trend::High
    } else {
        Trend::Average
    }
}

/// `price` against `average` as a signed whole percentage ("+12%", "-30%", "0%").
pub fn percent_text(price: i32, average: i32) -> String {
    if average <= 0 {
        return String::new();
    }
    let percent = ((f64::from(price) / f64::from(average) - 1.0) * 100.0).round() as i64;
    if percent > 0 {
        format!("+{percent}%")
    } else {
        format!("{percent}%")
    }
}

/// A price or cash as the screen shows it: tenths of a credit.
pub fn credits(value: i32) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let value = i64::from(value).abs();
    format!("{sign}{}.{}", value / 10, value % 10)
}

/// F390/F391's hold capacity: the ship's delivered capacity less the mass of
/// the equipment fitted after delivery. A new game's `Navigator::hold` is
/// F998's 7 t with the delivered loadout already priced in (`F998` does not run
/// F390 for its own equipment); a hull bought later uses the cargo statistic and
/// all of the fitted mass.
pub fn capacity(nav: &Navigator, cache: &VerifiedCache, state: &Flight) -> i32 {
    let base = nav.hold(cache, state.ship);
    let delivered = if let Some(start) = nav.new_game.filter(|state| !state.bought) {
        crate::boot::StartSpec::get(start.start_type)
            .expect("validated source start")
            .loadout()
    } else {
        // A bought hull's integral drive is already out of the cargo statistic;
        // a removable one is charged to the hold like any fitted drive.
        let (class, integral) = nav.hull_drive(cache, state.ship);
        ffe_core::equipment::Loadout {
            drive: if integral { class } else { 0 },
            ..ffe_core::equipment::Loadout::default()
        }
    };
    state
        .loadout
        .capacity(base, &delivered, &state.combat.lasers)
        .max(0)
}

/// The player's hold as the market code sees it.
pub fn hold_of(state: &Flight, capacity: i32) -> Hold {
    let mut hold = Hold::new(capacity);
    for (slot, tonnes) in hold.cargo.iter_mut().zip(&state.hold) {
        *slot = *tonnes;
    }
    hold.used = state.hold.iter().map(|t| i32::from(*t)).sum();
    hold
}

/// The market screen's lines for `station`: the legal goods, in the original's
/// order. The black market (F454's `0x80` mask) uses [`listing_with`] instead.
pub fn listing(
    nav: &mut Navigator,
    cache: &VerifiedCache,
    station: usize,
    state: &Flight,
) -> Result<Vec<Listing>, String> {
    listing_with(nav, cache, station, state, false)
}

/// [`listing`] with F538's illegal-goods mask: `show_illegal` is the `0x80`
/// bit F454 passes to F537, so the fake/real smuggler advertisement's screen
/// lists the bit-7 commodities as well.
pub fn listing_with(
    nav: &mut Navigator,
    cache: &VerifiedCache,
    station: usize,
    state: &Flight,
    show_illegal: bool,
) -> Result<Vec<Listing>, String> {
    let flags = nav.stock_flags(cache)?;
    let names: Vec<String> = {
        let tables = nav.market_tables().ok_or("no market tables")?;
        (0..GOODS)
            .map(|good| tables.name(good).unwrap_or("?").to_owned())
            .collect()
    };
    let market = nav.market(cache, station)?;
    Ok((0..GOODS)
        .filter(|good| show_illegal || flags[*good] & ILLEGAL == 0)
        .map(|good| Listing {
            good,
            name: names[good].clone(),
            price: market.market.entries[good].price,
            stock: market.market.entries[good].stock,
            held: state.hold.get(good).copied().unwrap_or(0),
        })
        .collect())
}

/// Whether this system's stock string marks any commodity illegal (F639/F640
/// only place a smuggler advertisement when one exists).
pub fn illegal_available(nav: &Navigator, cache: &VerifiedCache) -> Result<bool, String> {
    Ok(nav
        .stock_flags(cache)?
        .iter()
        .any(|flags| flags & ILLEGAL != 0))
}

/// F542/F543: a row action at F640's fake smuggler advertisement. The offence
/// is F237(5) ("Trading in illegal goods") and F543 charges `0x1388` (500.0
/// credits) to the starport's ledger; F554 returns before F555/F556, so no
/// tonne moves.
pub fn police_sting(nav: &mut Navigator, station: usize) -> OffenceReport {
    let ordinal = nav.starport_ordinal(station);
    let report = nav
        .legal
        .record(npc::TRADING_ILLEGAL, Some(ordinal))
        .unwrap_or(OffenceReport {
            message: String::new(),
            fine: 0,
            charged: 0,
            additional: false,
        });
    nav.legal.charge(npc::POLICE_FINE, Some(ordinal));
    OffenceReport {
        message: format!(
            "Trading in illegal goods is not permitted here. Pay a ${} fine.",
            credits(npc::POLICE_FINE as i32)
        ),
        charged: report.charged.saturating_add(npc::POLICE_FINE),
        ..report
    }
}

/// Buy (or sell) up to `count` tonnes of `good` at `station`, stopping at the first
/// refusal. Returns the original's console message (`0x98e1..0x98e7`): a refusal
/// names the commodity, otherwise a completed trade says "Buying <good>." or
/// "Selling <good>.".
pub fn trade(
    nav: &mut Navigator,
    cache: &VerifiedCache,
    state: &mut Flight,
    station: usize,
    good: usize,
    buying: bool,
    count: u32,
) -> Result<String, String> {
    if good >= GOODS {
        return Err("no such commodity".to_owned());
    }
    let space = capacity(nav, cache, state);
    let name = nav
        .market_tables()
        .and_then(|tables| tables.name(good).map(str::to_owned))
        .ok_or("no market tables")?;
    let mut hold = hold_of(state, space);
    let mut cash = i32::try_from(state.credits).unwrap_or(i32::MAX);
    let market = nav.market(cache, station)?;
    let mut done = 0;
    let mut refusal = None;
    for _ in 0..count.max(1) {
        let result = if buying {
            market.market.buy(good, &mut cash, &mut hold)
        } else {
            market.market.sell(good, &mut cash, &mut hold)
        };
        match result {
            Ok(()) => done += 1,
            Err(why) => {
                refusal = Some(why);
                break;
            }
        }
    }
    state.credits = u32::try_from(cash.clamp(0, 1_000_000_000)).unwrap_or(0);
    state.hold = hold.cargo[..NAMES].to_vec();
    Ok(match (done, refusal) {
        (_, Some(why)) => why.message(&name),
        (0, None) => String::new(),
        (_, None) if buying => format!("Buying {name}."),
        (_, None) => format!("Selling {name}."),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_price_is_low_average_or_high_against_the_galactic_average() {
        assert_eq!(trend(890, 1000), Trend::Low);
        assert_eq!(trend(900, 1000), Trend::Average);
        assert_eq!(trend(1100, 1000), Trend::Average);
        assert_eq!(trend(1101, 1000), Trend::High);
        assert_eq!(trend(5, 0), Trend::Average);
        assert_eq!(percent_text(1120, 1000), "+12%");
        assert_eq!(percent_text(700, 1000), "-30%");
        assert_eq!(percent_text(1000, 1000), "0%");
    }
}
