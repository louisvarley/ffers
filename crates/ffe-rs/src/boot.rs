//! Source-game entry guards shared by normal startup and every load path.
use crate::{
    flight::Flight, navigation::Navigator, original_flight::OriginalFlight, saves::SaveGame,
    ships::ShipStats,
};
use ffe_content::VerifiedCache;

/// Essential records must be present before a normal game can enter simulation.
pub fn validate_assets(cache: &VerifiedCache) -> Result<(), String> {
    for id in [
        "base:model/records",
        "base:ship/stats",
        "base:gameplay/tables",
        "base:gameplay/pads",
        "base:terrain/tables",
        "base:terrain/maps",
        "base:world/system-tables",
        "base:world/galaxy-tables",
    ] {
        if cache.asset(id).is_none_or(|bytes| bytes.is_empty()) {
            return Err(format!("Cannot start the source game: missing or empty essential asset {id}. Re-import the game files."));
        }
    }
    Ok(())
}

pub fn ship_stats(cache: &VerifiedCache, ship: u32) -> Result<ShipStats, String> {
    let info = ffe_core::objects::ModelInfo::read(
        cache
            .asset("base:model/records")
            .ok_or("Missing model records.")?,
        cache
            .asset("base:ship/stats")
            .ok_or("Missing ship statistics.")?,
        ship,
    )
    .ok_or_else(|| format!("No source model record for ship {ship}."))?;
    info.stats
        .as_ref()
        .and_then(|block| ShipStats::from_block(block))
        .ok_or_else(|| format!("No source flight statistics for ship {ship}."))
}

/// A prototype or historical snapshot is never silently converted into a new
/// commander's source world. It needs an explicit import or developer tool.
pub fn validate_save(cache: &VerifiedCache, save: &SaveGame) -> Result<(), String> {
    validate_assets(cache)?;
    if save.navigation.is_none() {
        return Err("This snapshot has no source navigation state and cannot resume a normal game. Use a native commander save.".to_owned());
    }
    if !save.state.system_flight && save.state.mode == crate::flight::Mode::Flight {
        return Err(
            "This snapshot uses prototype flight and cannot resume a normal game.".to_owned(),
        );
    }
    ship_stats(cache, save.state.ship)?;
    Ok(())
}

/// All new-commander paths use the same source initializer.
pub fn new_commander(
    cache: &VerifiedCache,
    nav: &Navigator,
) -> Result<(Flight, OriginalFlight), String> {
    validate_assets(cache)?;
    let ship = nav.new_game.ok_or("Missing source start record.")?.ship;
    ship_stats(cache, ship)?;
    OriginalFlight::new_commander(cache, nav)
}

/// The navigation computer's general-equipment bit (bit 5, 0 t). F998 fits it only in the
/// default (Hope) start; the owner asked for every start to have one (EXP-056), so the
/// Ross, Mars and Lave words below are the source's with this bit added.
pub const NAVIGATION_COMPUTER: u32 = 1 << 5;
/// The automatic pilot's general-equipment bit (bit 21) and its mass. F998 fits it at Hope
/// and Ross 154 only; the owner asked for every start to have one (EXP-056), so the Mars
/// and Lave words add it and their holds lose its tonne as the source's do for the
/// equipment they fit.
pub const AUTOPILOT: u32 = 1 << 21;
pub const AUTOPILOT_TONNES: i32 = 1;

/// F998's four delivered setups; 4/5 take its default arm.
#[derive(Debug, Clone, Copy)]
pub struct StartSpec {
    pub ship: u32,
    pub cash: i32,
    pub port: usize,
    pub player: u8,
    pub mode: u8,
    pub hold_loss: i32,
    pub drive: u8,
    pub general: u32,
    pub laser: u8,
    pub missiles: usize,
    pub fuel: u16,
    pub energy: u32,
}
impl StartSpec {
    pub fn get(start: u8) -> Result<Self, String> {
        let common = Self {
            ship: 31,
            cash: 10000,
            port: 0x6a,
            player: 87,
            mode: 0x30,
            hold_loss: 16,
            drive: 2,
            general: 0x40240020,
            laser: 0x88,
            missiles: 2,
            fuel: 1,
            energy: 0x20000000,
        };
        Ok(match start {
            0 | 4 | 5 => common,
            1 => Self {
                ship: 23,
                cash: 1000,
                port: 0x6e,
                player: 107,
                general: 0x40240000 | NAVIGATION_COMPUTER,
                ..common
            },
            2 => Self {
                ship: 23,
                cash: 1000,
                port: 0x5f,
                player: 61,
                hold_loss: 12 + AUTOPILOT_TONNES,
                drive: 1,
                general: 0x40000000 | NAVIGATION_COMPUTER | AUTOPILOT,
                laser: 0xa0,
                fuel: 0,
                energy: 0x40000000,
                ..common
            },
            3 => Self {
                ship: 38,
                cash: 1000,
                port: 0x70,
                player: 111,
                mode: 0x2a,
                hold_loss: 31 + AUTOPILOT_TONNES,
                drive: 3,
                general: 0x40000 | NAVIGATION_COMPUTER | AUTOPILOT,
                missiles: 4,
                fuel: 4,
                energy: 0x40000000,
                ..common
            },
            _ => return Err(format!("Unsupported source start type {start}.")),
        })
    }
}

impl StartSpec {
    pub fn combat(self) -> crate::combat::Combat {
        let mut combat = crate::combat::Combat::new_game();
        combat.drive = self.drive;
        combat.energy = self.energy;
        combat.lasers = [self.laser, 0, 0, 0];
        combat.missiles.fill(0);
        combat.missiles[..self.missiles].fill(0x82);
        combat
    }
    pub fn loadout(self) -> ffe_core::equipment::Loadout {
        let mut loadout = ffe_core::equipment::Loadout::new_game();
        loadout.general = self.general;
        loadout.drive = self.drive;
        loadout.missiles = self.combat().missiles;
        loadout
    }
}

/// Restore the source frame adapter as well as the public flight snapshot.
/// Pad saves must not resume as free flight with no supporting planet.
pub fn resume_model(
    cache: &VerifiedCache,
    state: &Flight,
    nav: &Navigator,
    acceleration: usize,
) -> Result<OriginalFlight, String> {
    validate_assets(cache)?;
    let stats = ship_stats(cache, state.ship)?;
    let mut model =
        OriginalFlight::from_flight(state, Some(&stats)).with_time_acceleration(acceleration);
    if nav.landed_view() {
        let index = nav
            .landed_body()
            .or(nav.docked)
            .ok_or("Landed save has no supporting body.")?;
        let station = nav
            .bodies
            .get(index)
            .ok_or("Landed save has no supporting body.")?;
        let planet = station
            .parent
            .and_then(|i| nav.bodies.get(i))
            .unwrap_or(station);
        let records = cache
            .asset("base:model/records")
            .ok_or("Missing model records.")?;
        let bytes = cache
            .asset("base:ship/stats")
            .ok_or("Missing ship statistics.")?;
        let info = |id| {
            ffe_core::objects::ModelInfo::read(records, bytes, id).ok_or("Missing frame model.")
        };
        let ship = info(state.ship)?;
        model.begin_pad_takeoff(
            &crate::navigation::landing_body(planet, &info(planet.model)?),
            planet.position,
            &crate::navigation::landing_body(station, &info(station.model)?),
            station.position,
            &ship,
        );
    }
    let mode = nav
        .landed_state()
        .or_else(|| {
            nav.new_game
                .filter(|_| nav.on_pad())
                .map(|start| start.player_state)
        })
        .or_else(|| nav.sequence.map(|sequence| sequence.stages.state))
        .unwrap_or(if nav.transit.is_some() {
            4
        } else if nav.autopilot_engaged() {
            2
        } else {
            0
        });
    model.set_mode(mode);
    Ok(model)
}
