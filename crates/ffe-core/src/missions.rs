//! Mission contracts, the journal and the message pages.
//!
//! The BBS's reply page (`F521`) ends with the "OK - agreed." reply (`F534`),
//! which clears the advertisement (`F535`) and adds a contract (`F290`). The
//! contract is the 20-byte record F301/F302/F303 read:
//!
//! ```text
//! +0x00  reward (whole credits, from the row's StrVars[0])
//! +0x04  basic_data (passenger/parcel/additive flags)
//! +0x08  destination system code
//! +0x0c  deadline: game day (F534: accept day + 0x1c)
//! +0x10  name seed (F791's `0x92xx` names)
//! ```
//!
//! F301 marks an overdue contract and posts its 0x9c36 passage notice; F302
//! tests that mark before the destination, so an overdue contract fails at any
//! system (0x9c37, or 0x9c38 for a late military package), and otherwise
//! completes when the current system is its destination. F303 pays/removes a
//! completed contract, writing 0x9c31/0x9c32/0x9c33/0x9c34. The military
//! completion also shows F307's medal and F306's promotion/demotion pages,
//! which start songs 10 and 7. The strings are the original 0xff-coded text
//! from `DATA_005299`/`DATA_005104`; [`text`] expands them through the same
//! F1348 subset as the advertisements.
//!
//! The port keeps the contract fields, the journal entries the player sees and
//! the message pages. Its cash is the original's tenths (`$10.0` is 100), so a
//! completion returns whole credits and the caller adds `reward * 10`; the
//! equivalence is checked in `docs/notes/missions.md`.

use crate::bulletin::{self, Destination, Post, Var, World};
use serde::{Deserialize, Serialize};

/// F534's `0x3b` contract cap (`DATA_008804+0x4a18` / `DATA_009073`).
pub const MAX_MISSIONS: usize = 0x3b;
/// The journal keeps the last entries; the original journal page shows one
/// issue at a time and the port's screen scrolls a ring.
pub const MAX_JOURNAL: usize = 256;
/// The message pages the comms screen can list.
pub const MAX_PAGES: usize = 64;
/// F534's acceptance deadline: the current day + 0x1c days.
pub const ACCEPT_DAYS: u32 = 0x1c;
/// F303/DATA_008922's reputation ceiling (`cmp ...,0x5`).
pub const MAX_REPUTATION: i32 = 5;

/// F303's `basic_data` test for a passenger who cannot pay:
/// `(basic_data >> 8) & 0x1f == 0x14`. F638 puts the passage's
/// `DATA_006173` additive index in bits 9..12, so this is index 10 (the
/// `$30` extra-money tier), not a removal title: F643's title code
/// (`0x99b4..`) shifts to `0x19`, and a parcel's `0x2000` bit falls outside
/// the mask.
pub fn refuses_payment(basic_data: u32) -> bool {
    (basic_data >> 8) & 0x1f == 0x14
}

/// The contract kinds the BBS can generate. F638's 0x981a/0x981b posts are the
/// passenger/parcel jobs (the passage string picks which); F643's 0x981f..
/// 0x9822 posts are the removal/assassination jobs. The military delivery
/// (F509's 0x9854 page) uses the same contract record but has no BBS generator
/// in this port; see `docs/notes/missions.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Passenger,
    Parcel,
    Assassination,
    MilitaryDelivery,
    /// F626/F627/F628's military assassination (0x9857), photograph
    /// (0x9855) and bombing (0x9856) contracts; the mission's `code` says
    /// which.
    MilitaryStrike,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Passenger => "passenger",
            Kind::Parcel => "parcel",
            Kind::Assassination => "removal",
            Kind::MilitaryDelivery => "military delivery",
            Kind::MilitaryStrike => "military operation",
        }
    }
}

/// The contract's state. Completed/failed contracts stay in the list (so the
/// journal has their details) and an active one is removed by [`State::sweep`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Active,
    Complete,
    Failed,
    Expired,
    Abandoned,
}

/// One accepted contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mission {
    pub id: u32,
    pub kind: Kind,
    /// The BBS advertisement's message code (0x981a/0x981b/0x981f..0x9822).
    pub code: u16,
    /// The system and station the contract was accepted at.
    pub origin: u32,
    pub origin_name: String,
    pub destination: u32,
    pub destination_name: String,
    /// Whole credits, as the BBS row's money slot stores them.
    pub reward: u32,
    pub accepted_day: u32,
    pub deadline_day: u32,
    /// F638's basic_data low three bits + one; 0 for a parcel/removal.
    pub passengers: u8,
    /// The passenger passage text, parcel description or removal title.
    pub cargo: String,
    /// F791's `0x92xx` name seed (the passenger or the removal target).
    pub seed: u32,
    /// The raw F638/F643 data word, kept for the source record.
    pub basic_data: u32,
    /// F295's client name (saved; older saves have none).
    #[serde(default)]
    pub client: String,
    pub status: Status,
    pub settled_day: Option<u32>,
    /// A military photograph or bombing has been carried out; the contract
    /// pays on return to the system it was accepted in.
    #[serde(default)]
    pub done: bool,
    /// The issuing military's allegiance for a strike (1 Imperial, 2
    /// Federal); 0 for a civilian contract or an older save.
    #[serde(default)]
    pub side: u8,
    /// F301's `basic_data` bit 0x8000: the contract is past `+0xc` and its
    /// overdue notice has been made. F302's pass tests this bit before the
    /// current-system compare, so it fails at any destination.
    #[serde(default)]
    pub overdue_noticed: bool,
}

impl Mission {
    /// The military strike this is (0x9855 photograph, 0x9856 bombing,
    /// 0x9857 assassination), if any.
    pub fn strike(&self) -> Option<u16> {
        (self.kind == Kind::MilitaryStrike).then_some(self.code)
    }

    /// F316's contract-page row: `0x9c1e + kind` for kinds 0..=2 (removal,
    /// Imperial, Federal kill) and `0x9c21 + kind` for 3..=6 (Imperial and
    /// Federal photograph, then bombing). The kind nibble is the strike's
    /// base plus the issuing allegiance (1 Imperial, 2 Federal), matching the
    /// strings' Imperial-then-Federal order. `None` for a non-contract.
    pub fn row_code(&self) -> Option<u16> {
        let side = u16::from(self.side.min(2));
        match (self.kind, self.code) {
            (Kind::Assassination, _) => Some(0x9c1e),
            (Kind::MilitaryStrike, 0x9857) => Some(0x9c1e + side),
            (Kind::MilitaryStrike, 0x9855) if side > 0 => Some(0x9c21 + 2 + side),
            (Kind::MilitaryStrike, 0x9856) if side > 0 => Some(0x9c21 + 4 + side),
            _ => None,
        }
    }

    /// F316's row text for [`Mission::row_code`] with the target and return
    /// point filled in.
    pub fn row_text(&self) -> String {
        let (prefix, verb) = match self.row_code() {
            Some(0x9c1f) => ("IMP. NAVY: ", "Kill"),
            Some(0x9c20) => ("FED. MIL.: ", "Kill"),
            Some(0x9c24) => ("IMPERIAL NAVY: ", "Film"),
            Some(0x9c25) => ("FED. MILITARY: ", "Film"),
            Some(0x9c26) => ("IMPERIAL NAVY: ", "Bomb"),
            Some(0x9c27) => ("FED. MILITARY: ", "Bomb"),
            _ => ("", "Kill"),
        };
        format!(
            "{prefix}{verb} {}, return to {}",
            self.cargo, self.origin_name
        )
    }

    /// The day F303/F301's comparison uses: the contract's `+0xc` deadline.
    pub fn overdue(&self, day: u32) -> bool {
        day > self.deadline_day
    }

    /// `true` when the contract is a parcel (no cabin count).
    pub fn is_parcel(&self) -> bool {
        self.kind == Kind::Parcel
    }
}

/// What a journal entry records. The text is the source page's expanded string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryKind {
    Accepted,
    Completed,
    Failed,
    Expired,
    Abandoned,
    Rank,
}

/// One journal entry (`MISSION.DAT`'s text, but for the contract events the
/// port can generate).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub day: u32,
    pub kind: EntryKind,
    pub text: String,
}

/// The F306/F307 message pages: a normal comm message (F349's bong), the rank
/// page (song 7) or the medal page (song 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageKind {
    Message,
    Rank,
    Medal,
}

impl PageKind {
    /// The song `F306`/`F307` starts for this page, if any.
    pub fn song(self) -> Option<i16> {
        match self {
            PageKind::Message => None,
            PageKind::Rank => Some(7),
            PageKind::Medal => Some(10),
        }
    }

    /// The source routine that shows the page.
    pub fn citation(self) -> &'static str {
        match self {
            PageKind::Message => "F349/F350 (ffe.asm:27059-27185)",
            PageKind::Rank => "F306 (ffe.asm:24542)",
            PageKind::Medal => "F307 (ffe.asm:24622)",
        }
    }
}

/// One message page the comms screen lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    pub kind: PageKind,
    pub day: u32,
    pub text: String,
    /// The original's F307 bit (`DATA_009091`) / F349 bell: the page is new.
    pub unread: bool,
}

/// The mission, journal and message state. All fields default so a save from
/// before this slice loads with no missions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub missions: Vec<Mission>,
    #[serde(default)]
    pub journal: Vec<Entry>,
    #[serde(default)]
    pub pages: Vec<Page>,
    #[serde(default)]
    pub next_id: u32,
    /// `DATA_008922`: 5 or below. F303 raises it on a completed civilian
    /// contract, F521's reply roll (`missions::f521_refuses`) refuses negative
    /// values, and F302/F485 lower it without a floor (the source subtracts
    /// from a signed dword).
    #[serde(default)]
    pub reputation: i32,
    /// The military rank points F306's `DATA_005114` thresholds compare.
    #[serde(default)]
    pub military_points: u32,
    /// The rank index derived from `military_points` (0..=8). Cached so the
    /// screens do not need the thresholds.
    #[serde(default)]
    pub rank: u8,
    /// The mission scripts' own contracts (F314's kind 7), shown on the
    /// contracts page with the removal jobs.
    #[serde(default)]
    pub script_contracts: Vec<ScriptContract>,
}

/// F314's `DATA_008804+0x4ef0` list holds at most six contracts: the removal
/// and military strike jobs and the scripts' own.
pub const MAX_CONTRACTS: usize = 6;

/// A mission script's contract record (F314 with kind 7, from F41's statement
/// 37; statement 38's F315 removes it). F316 asks the scripts for its row:
/// global 8 is the record's id, then the scripts leave the row's message
/// code in global 9 and its words in 1-3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptContract {
    pub id: i32,
    /// `+0x30`: the system it was made in (`DATA_008885`).
    pub system: u32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            missions: Vec::new(),
            journal: Vec::new(),
            pages: Vec::new(),
            next_id: 1,
            reputation: 0,
            military_points: 0,
            rank: 0,
            script_contracts: Vec::new(),
        }
    }
}

/// The `DATA_005114` thresholds: 1, 16, 81, 256, 729, 4096, 6561, 14641, then
/// the end marker. They are the fourth powers the original uses (`F306`).
pub const RANK_THRESHOLDS: [u32; 8] = [1, 16, 81, 256, 729, 4096, 6561, 14641];

/// The military rank names for [`RANK_THRESHOLDS`]' nine bands. These are the
/// first nine Federal rank strings (`DATA_005104`'s 0x9c4e..0x9c56: Private,
/// Corporal, Sergeant, ...); the Imperial titles (0x9c5b..0x9c66) are the
/// other ladder, which needs an allegiance field the port's 20-byte contract
/// does not carry.
pub const RANK_NAMES: [&str; 9] = [
    "Private",
    "Corporal",
    "Sergeant",
    "Sergeant-Major",
    "Major",
    "Colonel",
    "Lieutenant",
    "Lt. Commander",
    "Captain",
];

/// The rank index for a points total (`F306`'s loop over `DATA_005114`).
pub fn rank_for(points: u32) -> u8 {
    RANK_THRESHOLDS
        .iter()
        .position(|threshold| points < *threshold)
        .map_or(8, |index| index as u8)
}

/// An offer read out of a BBS post. `None` for the advertisements that are not
/// contracts (goods, crew, charity, military recruitment).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub kind: Kind,
    pub code: u16,
    pub destination: u32,
    pub destination_name: String,
    pub reward: u32,
    pub passengers: u8,
    pub cargo: String,
    pub seed: u32,
    pub basic_data: u32,
    /// The contract's `+0xc` deadline in days from acceptance. Civilian
    /// offers use F534's 0x1c; F632's military package uses `+0xe` plus
    /// `random(0x3f)` (14..77).
    pub deadline_days: u32,
    /// The row/page text: the BBS post for a civilian offer, F493's 0x9854
    /// mission description for a military one.
    pub text: String,
    /// F295's client (`0x9203`: F791's name of kind 3 from the seed); empty
    /// for a military package, which has no name.
    pub client: String,
    /// The issuing military's allegiance for a strike (1 Imperial, 2
    /// Federal); 0 otherwise.
    pub side: u8,
}

/// Read F453's row variables back into a contract. The post's variables are
/// F646's five args: 0 money, 1 passage description/title, 2 destination,
/// 3 name seed, 4 basic_data (F638) or the removal title (F643).
pub fn offer(post: &Post, world: &World<'_>) -> Option<Offer> {
    let code = post.code;
    let kind = match code {
        0x981a | 0x981b => {
            let basic = match post.vars[4] {
                Var::Int(value) => value as u32,
                _ => 0,
            };
            if basic & 0x2000 != 0 {
                Kind::Parcel
            } else {
                Kind::Passenger
            }
        }
        0x981f..=0x9822 => Kind::Assassination,
        _ => return None,
    };
    let reward = match post.vars[0] {
        Var::Int(value) if value > 0 => u32::try_from(value).ok()?,
        _ => return None,
    };
    let destination = match post.vars[2] {
        Var::System(code) => code,
        _ => return None,
    };
    // F643's removal title is StrVars[4] (`0x301c`); F638's passage
    // description is StrVars[1]. The source's date slot (StrVars[1] for
    // removal) is not the contract deadline, which F534 rewrites.
    let cargo = match (kind, &post.vars[1], &post.vars[4]) {
        (Kind::Assassination, _, Var::Text(title)) => title.clone(),
        (_, Var::Text(text), _) => text.clone(),
        _ => String::new(),
    };
    let seed = match post.vars[3] {
        Var::Seed(seed) => seed,
        Var::Int(seed) => seed as u32,
        Var::System(code) => code,
        Var::Text(_) => 0,
    };
    let basic_data = match post.vars[4] {
        Var::Int(value) => value as u32,
        _ => 0,
    };
    let passengers = if kind == Kind::Passenger {
        ((basic_data & 7) as u8).saturating_add(1)
    } else {
        0
    };
    Some(Offer {
        kind,
        code,
        destination,
        destination_name: world.system_name(destination),
        reward,
        passengers,
        cargo,
        seed,
        basic_data,
        deadline_days: ACCEPT_DAYS,
        text: post.text.clone(),
        client: bulletin::client_name(world, seed),
        side: 0,
    })
}

/// F629/F630/F631/F632: the military package (delivery) offers. The three
/// tiers are the source's base strings 0x996e (8 variants), 0x9976 (4) and
/// 0x997a (4); F630/F631 refuse the higher two below `DATA_005114`'s 0x10/0x51
/// rank points. The reward is F632's arithmetic, decoded from the asm's
/// `50 * (base_string + additive + 2 * random(4)) - 0x1df6e6`: the constants
/// give 150, 550 and 750 credits before the additive/random term. The deadline
/// is `day + 0xe + random(0x40)`. F623's other constructors need target
/// ships/military bases and are not generated.
pub fn military_offers(
    rng: &mut crate::npc::Random,
    world: &World<'_>,
    day: u32,
    rank_points: u32,
) -> Vec<Offer> {
    let tiers = [
        (0x996e_u16, 8_u32, 150_u32, 0_u32),
        (0x9976, 4, 550, 0x10),
        (0x997a, 4, 750, 0x51),
    ];
    let mut offers = Vec::new();
    for (base_code, variants, base_reward, minimum_rank) in tiers {
        if rank_points <= minimum_rank && minimum_rank != 0 {
            continue;
        }
        let additive = rng.bounded(variants);
        let Some(destination) = world.neighbour(rng, 3) else {
            continue;
        };
        // F632's `esi` is F922(3), reused by F857's system pick and the
        // reward's 2*esi term; the port draws it after the destination.
        let esi = rng.bounded(4);
        let reward = base_reward
            .saturating_add(50 * additive)
            .saturating_add(100 * esi);
        let deadline_days = 14 + rng.bounded(0x40);
        let seed = rng.bounded(0x1_0000);
        let cargo_code = base_code + additive as u16;
        let cargo = bulletin::render_code(
            cargo_code,
            &[
                Var::Int(0),
                Var::Int(0),
                Var::System(destination.code),
                Var::Seed(seed),
                Var::Int(0),
            ],
            world,
        )
        .unwrap_or_else(|| format!("[{cargo_code:#06x}]"));
        let vars = [
            Var::Int(i64::from(reward)),
            Var::Int(i64::from(day.saturating_add(deadline_days))),
            Var::System(destination.code),
            Var::Seed(seed),
            Var::Text(cargo.clone()),
        ];
        let text = bulletin::render_code(0x9854, &vars, world).unwrap_or_default();
        let tier = if base_code == 0x997a {
            2
        } else if base_code == 0x9976 {
            1
        } else {
            0
        };
        offers.push(Offer {
            kind: Kind::MilitaryDelivery,
            code: 0x9854,
            destination: destination.code,
            destination_name: destination.name.clone(),
            reward,
            passengers: 0,
            cargo,
            seed,
            // F509: 0x2000 plus 0x100 for each rank tier above the first.
            basic_data: 0x2000 | (tier * 0x100),
            deadline_days,
            text,
            client: String::new(),
            side: 0,
        });
    }
    offers
}

/// `DATA_006211`/`DATA_006212`/`DATA_006213`: the assassination, photograph
/// and bombing payments.
const STRIKE_PAYMENTS: [[u32; 8]; 3] = [
    [4000, 4500, 5000, 5500, 6000, 7000, 8000, 10000],
    [10000, 11000, 12000, 13000, 14000, 15000, 16000, 17000],
    [15000, 16000, 17000, 18000, 19000, 20000, 22000, 25000],
];

/// `DATA_006210`: the capital each allegiance's military sends its strikes
/// against (F625 picks a system round it with F857).
pub const ENEMY_CAPITALS: [u32; 6] = [
    0x12a4_9718,
    0x12a4_9718,
    0x0aa4_1718,
    0x0aa4_1718,
    0x0aa4_1718,
    0x0aa4_1718,
];

/// F626/F627/F628: the military assassination (rank points over 0x100,
/// 0x9857 with a 0x99bc title), photograph (over 0x271, 0x9855 with a 0x9966
/// title) and bombing (over 0x510, 0x9856) contracts. `enemy` holds the
/// systems round the enemy capital (F625's F857 pick, population 3 or more,
/// or 1 for a bombing); the deadline is today + 14 + RND(64) and the payment
/// one of eight.
pub fn military_strike_offers(
    rng: &mut crate::npc::Random,
    world: &World<'_>,
    enemy: &[Destination],
    day: u32,
    rank_points: u32,
) -> Vec<Offer> {
    let jobs = [
        (0x9857_u16, 0x100_u32, 3_u32, 0x99bc_u16, 0_usize),
        (0x9855, 0x271, 3, 0x9966, 1),
        (0x9856, 0x510, 1, 0x9966, 2),
    ];
    let mut offers = Vec::new();
    for (code, minimum, population, title_base, table) in jobs {
        if rank_points <= minimum {
            continue;
        }
        let choices: Vec<&Destination> = enemy
            .iter()
            .filter(|destination| {
                destination.code != world.code && destination.population >= population
            })
            .collect();
        if choices.is_empty() {
            continue;
        }
        let destination = choices[rng.bounded(choices.len() as u32) as usize];
        let pick = rng.bounded(8) as usize;
        let deadline_days = 14 + rng.bounded(0x40);
        let seed = rng.bounded(0x1_0000);
        let reward = STRIKE_PAYMENTS[table][pick];
        let title_code = title_base + pick as u16;
        let title = bulletin::render_code(
            title_code,
            &[
                Var::Int(0),
                Var::Int(0),
                Var::System(destination.code),
                Var::Seed(seed),
                Var::Int(0),
            ],
            world,
        )
        .unwrap_or_else(|| format!("[{title_code:#06x}]"));
        let vars = [
            Var::Int(i64::from(reward)),
            Var::Int(i64::from(day.saturating_add(deadline_days))),
            Var::System(destination.code),
            Var::Seed(seed),
            Var::Text(title.clone()),
        ];
        let text = bulletin::render_code(code, &vars, world).unwrap_or_default();
        offers.push(Offer {
            kind: Kind::MilitaryStrike,
            code,
            destination: destination.code,
            destination_name: destination.name.clone(),
            reward,
            passengers: 0,
            cargo: title,
            seed,
            basic_data: 0x2000,
            deadline_days,
            text,
            client: String::new(),
            side: world.allegiance as u8,
        });
    }
    offers
}

/// One row of F508/F495/F496/F506's suggested-response page. `code` is the
/// row's source message code; `DATA_005288[code - 0x9800]` is its handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefingRow {
    pub code: u16,
    pub text: String,
}

/// What a briefing row asks the page to do (the `DATA_005288` handler).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BriefingStep {
    /// F510/F511/F512/F514/F499/F500-F502: the handler's answer page is
    /// printed and the response rows stay.
    Answer,
    /// 0x985a: F513 prints the description page again.
    Description,
    /// 0x9858 (F509/F498) or 0x9861 (F467): accept the offer. The caller
    /// checks the roster and the bombing pylon, then records the contract.
    Accept,
    /// 0x9860: F497/F515 rebuilds F493's mission list.
    List,
    /// 0x983e/0x983f: F527 hangs up, back to F453's bulletin board. F460's
    /// removal conversation has no mission list to return to.
    HangUp,
}

/// F508/F495/F496/F506's military briefing and F460's civilian removal
/// conversation: the description F352 prints when the offer's list row is
/// selected and the suggested responses F342/F345 put on F523's screen 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Briefing {
    pub offer: Offer,
    /// F352's description text.
    pub description: String,
    /// The page F352/F353 and the answer handlers last printed.
    pub page: String,
    /// The acceptance day; the deadline pages add `deadline_days`.
    pub day: u32,
    /// F453/F460's advert date (`+0x8`, absolute): F643's removal ads carry a
    /// 7..70-day departure date that F469/F468 print. Equals `day` for the
    /// military pages, whose date argument is the acceptance day.
    pub date: u32,
    /// F466/F499's destination body/starport name.
    pub place: String,
    /// F470's target ship (its statistics name).
    pub ship: String,
    /// F791 kind 0x12 registration from the offer's seed.
    pub registration: String,
    /// F498/F509/F467's acceptance (or refusal) closes the conversation: F342
    /// leaves only the 0x9860 "Return to Mission List" row (0x983f "Hang up."
    /// for F460's removal page, which has no mission list).
    pub closed: bool,
    /// The board row the advertisement was selected from (F453's list slot),
    /// so F535 can clear it when F467 accepts.
    pub row: usize,
}

/// F460's civilian removal conversation: F467's accept, F469/F470/F468's
/// answers and F527's two ways out. The share of `briefing_codes` the source
/// does not put on the military pages.
pub const REMOVAL_CODES: [u16; 6] = [0x9861, 0x9862, 0x9863, 0x9864, 0x983e, 0x983f];

/// The response rows F508 (0x9854), F495/F496 (0x9855/0x9856), F506 (0x9857)
/// and F460 (0x981f..0x9822) add after the description, in source order.
pub fn briefing_codes(code: u16) -> Vec<u16> {
    const SHARED: [u16; 7] = [0x9859, 0x985a, 0x985b, 0x985c, 0x985d, 0x985e, 0x985f];
    match code {
        // F508: all eight replies, then the way back.
        0x9854 => (0x9858..=0x985f).chain([0x9860]).collect(),
        // F496 for a photograph: accept, the camera answer, the destination,
        // the shared seven, the way back.
        0x9855 => [0x9858, 0x9866, 0x9865]
            .into_iter()
            .chain(SHARED)
            .chain([0x9860])
            .collect(),
        // F496 for a bombing: accept, the missile answers, the destination,
        // the shared seven, the way back.
        0x9856 => [0x9858, 0x9867, 0x9868, 0x9865]
            .into_iter()
            .chain(SHARED)
            .chain([0x9860])
            .collect(),
        // F506 for an assassination: accept, how to find them, their ship,
        // the deadline, then the shared seven and the way back.
        0x9857 => (0x9861..=0x9864).chain(SHARED).chain([0x9860]).collect(),
        // F460 for a removal: the same four answers, then two ways to hang up.
        0x981f..=0x9822 => REMOVAL_CODES.to_vec(),
        _ => Vec::new(),
    }
}

/// The handler `DATA_005288` gives a briefing row.
pub fn briefing_step(code: u16) -> BriefingStep {
    match code {
        0x9858 | 0x9861 => BriefingStep::Accept,
        0x985a => BriefingStep::Description,
        0x9860 => BriefingStep::List,
        0x983e | 0x983f => BriefingStep::HangUp,
        _ => BriefingStep::Answer,
    }
}

/// F460's Elite-score gate: `((seed << 14) >> 16) + 0x1000` against
/// `DATA_008911`, signed exactly as the source's `sar`/`jg`.
pub fn removal_rating(seed: u32) -> i32 {
    ((seed << 14) as i32 >> 16).saturating_add(0x1000)
}

/// F791 kind 0x12: the ship registration the assassination page prints.
pub fn registration_name(names: Option<&crate::names::Names>, seed: u32) -> String {
    names
        .and_then(|names| names.generate(0x12, seed))
        .map(|bytes| bytes.iter().map(|byte| char::from(*byte)).collect())
        .unwrap_or_default()
}

impl Briefing {
    /// Open F508/F495/F496/F506's page for `offer`. `place` is F466/F499's
    /// destination body/starport, `ship` F470's target ship and `registration`
    /// its F791 kind 0x12 number; the caller generates the three from the
    /// destination and the offer's seed.
    pub fn open(
        offer: &Offer,
        description: String,
        day: u32,
        place: String,
        ship: String,
        registration: String,
    ) -> Self {
        Self {
            offer: offer.clone(),
            page: description.clone(),
            description,
            day,
            date: day,
            place,
            ship,
            registration,
            closed: false,
            row: 0,
        }
    }

    /// F460's civilian removal conversation (0x981f..0x9822), not F493's
    /// military list page.
    pub fn is_removal(&self) -> bool {
        matches!(self.offer.code, 0x981f..=0x9822)
    }

    /// F331's `DATA_009052` title. F460 opens on 0x98d8 ("Video Link"); the
    /// military briefings keep F490/F491's HQ title.
    pub fn screen_title(&self) -> &'static str {
        if self.is_removal() {
            "Video Link"
        } else {
            "Military Missions"
        }
    }

    pub fn codes(&self) -> Vec<u16> {
        if self.closed {
            return vec![if self.is_removal() { 0x983f } else { 0x9860 }];
        }
        briefing_codes(self.offer.code)
    }

    fn world<'a>(destinations: &'a [Destination]) -> World<'a> {
        World {
            code: 0,
            name: "",
            population: 0,
            danger: 0,
            allegiance: 0,
            station_name: "",
            destinations,
            names: None,
        }
    }

    /// F327's entry texts for [`Self::codes`]. F345 expands each row with the
    /// offer's own StrVars; slot 1 (0x19, the target's name in "How do I find
    /// X?") is the mission's title, as F469/F470 use it.
    pub fn rows(&self) -> Vec<BriefingRow> {
        let destinations: [Destination; 0] = [];
        let world = Self::world(&destinations);
        let vars = [
            Var::Int(i64::from(self.offer.reward)),
            Var::Text(self.offer.cargo.clone()),
            Var::System(self.offer.destination),
            Var::Seed(self.offer.seed),
            Var::Text(self.offer.cargo.clone()),
        ];
        self.codes()
            .into_iter()
            .map(|code| BriefingRow {
                code,
                text: bulletin::render_code(code, &vars, &world)
                    .unwrap_or_else(|| format!("[{code:#06x}]")),
            })
            .collect()
    }

    /// The page for reply `code`, with the briefing's destination, place, ship
    /// and registration in the source's variable slots. The row codes are the
    /// `DATA_005288` handlers; each prints its own answer page.
    fn page(&self, code: u16) -> Option<String> {
        let deadline = self.day.saturating_add(self.offer.deadline_days);
        // F469/F468 print F453's `+0x8` date slot. A removal advertisement
        // carries F643's own departure date; the military pages use the
        // contract deadline.
        let date = if self.is_removal() {
            self.date
        } else {
            deadline
        };
        let (page, vars): (u16, [Var; 5]) = match code {
            // F469's 0x99c6: leaving PLACE in the SYSTEM system at TIME on
            // DATE. The port has no target time, so the date's own midnight
            // (0 tics) is used.
            0x9862 => (
                0x99c6,
                [
                    Var::Int(i64::from(date)),
                    Var::Int(0),
                    Var::System(self.offer.destination),
                    Var::Seed(self.offer.seed),
                    Var::Text(self.place.clone()),
                ],
            ),
            // F470's 0x99c7: the target's description, ship and registration.
            0x9863 => (
                0x99c7,
                [
                    Var::Int(0),
                    Var::Text(self.offer.cargo.clone()),
                    Var::Text(self.ship.clone()),
                    Var::Seed(self.offer.seed),
                    Var::Text(self.registration.clone()),
                ],
            ),
            // F468's 0x99c8: the attempt at PLACE on DATE.
            0x9864 => (
                0x99c8,
                [
                    Var::Int(i64::from(date)),
                    Var::Int(0),
                    Var::Seed(self.offer.seed),
                    Var::Seed(self.offer.seed),
                    Var::Text(self.place.clone()),
                ],
            ),
            // F499's 0x995b: the target on the surface of PLACE in the SYSTEM.
            0x9865 => (
                0x995b,
                [
                    Var::Int(i64::from(self.offer.reward)),
                    Var::Int(0),
                    Var::System(self.offer.destination),
                    Var::Text(self.place.clone()),
                    Var::Text(self.offer.cargo.clone()),
                ],
            ),
            // F500/F501/F502: the camera and missile answers.
            0x9866 => (0x9958, [const { Var::Int(0) }; 5]),
            0x9867 => (0x9959, [const { Var::Int(0) }; 5]),
            0x9868 => (0x995a, [const { Var::Int(0) }; 5]),
            // F510: fixed payment on completion only.
            0x9859 | 0x985b | 0x985c => (0x990c, [const { Var::Int(0) }; 5]),
            // F511/F512: the lateness answers.
            0x985d => (0x990d, [const { Var::Int(0) }; 5]),
            0x985e => (0x990e, [const { Var::Int(0) }; 5]),
            // F514: the danger answer by the offer's own code (0x9911 below
            // 0x9857, 0x9910 for the assassination).
            0x985f => (
                if self.offer.code == 0x9857 {
                    0x9910
                } else {
                    0x9911
                },
                [const { Var::Int(0) }; 5],
            ),
            _ => return None,
        };
        let destinations = [Destination {
            code: self.offer.destination,
            name: self.offer.destination_name.clone(),
            population: 0,
            danger: 0,
        }];
        bulletin::render_code(page, &vars, &Self::world(&destinations))
    }

    /// F513's description page again.
    pub fn description_page(&self) -> String {
        self.description.clone()
    }

    /// F459: F460's refusal, 0x99c5, with only F527's "Hang up." row.
    pub fn refuse(&mut self) {
        self.closed = true;
        self.page = removal_refusal();
    }

    /// F327/F353: print reply `code` and keep the response rows.
    pub fn respond(&mut self, code: u16) -> BriefingStep {
        let step = briefing_step(code);
        match step {
            BriefingStep::Answer => {
                if let Some(page) = self.page(code) {
                    self.page = page;
                }
            }
            BriefingStep::Description => self.page = self.description_page(),
            BriefingStep::Accept | BriefingStep::List | BriefingStep::HangUp => {}
        }
        step
    }
}

fn same_offer(mission: &Mission, offer: &Offer) -> bool {
    mission.kind == offer.kind
        && mission.code == offer.code
        && mission.destination == offer.destination
        && mission.reward == offer.reward
        && mission.seed == offer.seed
}

/// A successful acceptance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accepted {
    pub id: u32,
    pub text: String,
}

/// A settled contract: what the caller pays and which page to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub id: u32,
    pub status: Status,
    /// Whole credits the contract pays (`0` on a failure).
    pub reward: u32,
    pub page: Page,
    pub journal: Entry,
}

fn text_world<'a>(origin: u32, origin_name: &'a str, destinations: &'a [Destination]) -> World<'a> {
    World {
        code: origin,
        name: origin_name,
        population: 0,
        danger: 0,
        allegiance: 0,
        station_name: "",
        destinations,
        names: None,
    }
}

/// Expand one source page. `vars` are F646's StrVars slots.
fn page_text(
    code: u16,
    vars: &[Var; 5],
    origin: u32,
    origin_name: &str,
    destination: u32,
    destination_name: &str,
) -> String {
    let list = [Destination {
        code: destination,
        name: destination_name.to_owned(),
        population: 0,
        danger: 0,
    }];
    let world = text_world(origin, origin_name, &list);
    bulletin::render_code(code, vars, &world)
        .unwrap_or_else(|| format!("[mission page {code:#06x}]"))
}

/// F534's acceptance page: 0x992a ($reward on arrival at the destination).
/// F509/F498/F467's accept uses 0x99ca ("on your successful return").
fn accepted_text(offer: &Offer, origin: u32, origin_name: &str) -> String {
    let code = if offer.kind == Kind::MilitaryStrike {
        0x99ca
    } else {
        0x992a
    };
    accepted_page(code, offer, origin, origin_name)
}

fn accepted_page(page: u16, offer: &Offer, origin: u32, origin_name: &str) -> String {
    let vars = [
        Var::Int(i64::from(offer.reward)),
        Var::Text(offer.cargo.clone()),
        Var::System(offer.destination),
        Var::Seed(offer.seed),
        Var::Int(i64::from(offer.basic_data)),
    ];
    page_text(
        page,
        &vars,
        origin,
        origin_name,
        offer.destination,
        &offer.destination_name,
    )
}

/// F467's acceptance page for F460's removal conversation: 0x99ca, "$<reward>
/// will be paid on your successful return." F467 prints it before F535 clears
/// the advertisement; the removal contract itself is the port's existing
/// F290-record settlement.
pub fn removal_accepted_text(offer: &Offer, origin: u32, origin_name: &str) -> String {
    accepted_page(0x99ca, offer, origin, origin_name)
}

/// F506/F460's 0x99c4 description: "Greetings. We want <title> in the
/// <system> system killed, for $<reward>." The assassination/removal list
/// row's own 0x9857/0x981f.. page is the advertise text; the briefing opens
/// with this one.
pub fn assassination_description(offer: &Offer, origin: u32, origin_name: &str) -> String {
    let vars = [
        Var::Int(i64::from(offer.reward)),
        Var::Int(0),
        Var::System(offer.destination),
        Var::Seed(offer.seed),
        Var::Text(offer.cargo.clone()),
    ];
    page_text(
        0x99c4,
        &vars,
        origin,
        origin_name,
        offer.destination,
        &offer.destination_name,
    )
}

/// F498's bombing refusal: 0x98f8, when no missile pylon is free.
pub fn bombing_refusal() -> String {
    bulletin::render_code(
        0x98f8,
        &[
            Var::Int(0),
            Var::Int(0),
            Var::Int(0),
            Var::Int(0),
            Var::Int(0),
        ],
        &World {
            code: 0,
            name: "",
            population: 0,
            danger: 0,
            allegiance: 0,
            station_name: "",
            destinations: &[],
            names: None,
        },
    )
    .unwrap_or_else(|| "You have no empty missile positions.".to_owned())
}

/// F459: F460's refusal page, 0x99c5, when the commander's Elite score or the
/// destination does not qualify. The page keeps only F527's "Hang up." row.
pub fn removal_refusal() -> String {
    bulletin::render_code(
        0x99c5,
        &[
            Var::Int(0),
            Var::Int(0),
            Var::Int(0),
            Var::Int(0),
            Var::Int(0),
        ],
        &World {
            code: 0,
            name: "",
            population: 0,
            danger: 0,
            allegiance: 0,
            station_name: "",
            destinations: &[],
            names: None,
        },
    )
    .unwrap_or_else(|| {
        "Hello. Thanks for contacting us, but we want someone with a higher Elite Federation combat rating.".to_owned()
    })
}

/// F467/F498's "no more room" page: the 52-byte military contract list is
/// full (`DATA_008804+0x4ef0` at 6).
pub fn military_contract_full() -> String {
    bulletin::render_code(
        0x99cb,
        &[
            Var::Int(0),
            Var::Int(0),
            Var::Int(0),
            Var::Int(0),
            Var::Int(0),
        ],
        &World {
            code: 0,
            name: "",
            population: 0,
            danger: 0,
            allegiance: 0,
            station_name: "",
            destinations: &[],
            names: None,
        },
    )
    .unwrap_or_else(|| "There is no more room on your contract list".to_owned())
}

impl State {
    /// F534: accept an offer. The caller removes the advertisement (`F535`) and
    /// pays nothing; the reward is paid by [`Self::settle`] on arrival.
    /// The active assassination contracts (civilian and military) whose
    /// target is in `system`.
    pub fn targets_in(&self, system: u32) -> Vec<u32> {
        self.missions
            .iter()
            .filter(|mission| {
                mission.status == Status::Active
                    && (mission.kind == Kind::Assassination || mission.strike() == Some(0x9857))
                    && mission.destination == system
            })
            .map(|mission| mission.id)
            .collect()
    }

    /// The active photograph/bombing contracts whose base is in `system` and
    /// not yet done: `(id, code)`.
    pub fn bases_in(&self, system: u32) -> Vec<(u32, u16)> {
        self.missions
            .iter()
            .filter(|mission| {
                mission.status == Status::Active
                    && !mission.done
                    && mission.destination == system
                    && matches!(mission.strike(), Some(0x9855 | 0x9856))
            })
            .map(|mission| (mission.id, mission.code))
            .collect()
    }

    /// A photograph or bombing was carried out: it pays on the return to the
    /// origin system. Returns the journal line.
    pub fn strike_done(&mut self, id: u32, day: u32) -> Option<String> {
        let mission = self
            .missions
            .iter_mut()
            .find(|mission| mission.id == id && mission.status == Status::Active)?;
        mission.done = true;
        let text = match mission.code {
            0x9855 => format!(
                "Photographs taken. Return to {} by the deadline for payment.",
                mission.origin_name
            ),
            _ => format!(
                "Target destroyed. Return to {} by the deadline for payment.",
                mission.origin_name
            ),
        };
        self.record(day, EntryKind::Accepted, text.clone());
        Some(text)
    }

    /// The target of assassination contract `id` was destroyed in `system`:
    /// F303's completion and payment.
    pub fn target_destroyed(&mut self, id: u32, day: u32, system: u32) -> Result<Outcome, String> {
        let assassination = self.missions.iter().any(|mission| {
            mission.id == id
                && (mission.kind == Kind::Assassination || mission.strike() == Some(0x9857))
        });
        if !assassination {
            return Err("That contract is not paid for a kill.".to_owned());
        }
        self.settle(id, day, system)
    }

    /// The passengers of every active contract: `DATA_008892`, the occupied
    /// cabins F401 and F534 read.
    pub fn passengers_aboard(&self) -> u32 {
        self.missions
            .iter()
            .filter(|mission| mission.status == Status::Active)
            .map(|mission| u32::from(mission.passengers))
            .sum()
    }

    /// F534's cabin check: an offer with more passengers than the free cabins
    /// is refused with 0x9928.
    pub fn cabin_refusal(&self, offer: &Offer, cabins: u32) -> Option<String> {
        let free = cabins.saturating_sub(self.passengers_aboard());
        (u32::from(offer.passengers) > free).then(|| {
            bulletin::template(0x9928)
                .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
                .unwrap_or_else(|| "You do not have enough cabin space available.".to_owned())
        })
    }

    /// F534's first check: `DATA_009073 >= 0x3b` is refused with 0x9929.
    pub fn roster_refusal(&self) -> Option<String> {
        (self
            .missions
            .iter()
            .filter(|mission| mission.status == Status::Active)
            .count()
            >= MAX_MISSIONS)
            .then(|| {
                bulletin::template(0x9929)
                    .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
                    .unwrap_or_else(|| "There's no room on your carriage roster.".to_owned())
            })
    }

    /// F534: accept an offer. The caller removes the advertisement (`F535`) and
    /// pays nothing before arrival; the reward is paid by [`Self::settle`].
    pub fn accept(
        &mut self,
        offer: &Offer,
        day: u32,
        origin: u32,
        origin_name: &str,
    ) -> Result<Accepted, String> {
        let text = accepted_text(offer, origin, origin_name);
        self.accept_page(offer, day, origin, origin_name, text)
    }

    /// [`Self::accept`] with the page F534 writes. The BBS reply page uses
    /// 0x992b/0x992c for the half-now and all-now terms instead of 0x992a.
    pub fn accept_page(
        &mut self,
        offer: &Offer,
        day: u32,
        origin: u32,
        origin_name: &str,
        text: String,
    ) -> Result<Accepted, String> {
        if self
            .missions
            .iter()
            .filter(|m| m.status == Status::Active)
            .count()
            >= MAX_MISSIONS
        {
            return Err("There's no room on your carriage roster.".to_owned());
        }
        let id = self.next_id.max(1);
        let mission = Mission {
            id,
            kind: offer.kind,
            code: offer.code,
            origin,
            origin_name: origin_name.to_owned(),
            destination: offer.destination,
            destination_name: offer.destination_name.clone(),
            reward: offer.reward,
            accepted_day: day,
            deadline_day: day.saturating_add(offer.deadline_days),
            passengers: offer.passengers,
            cargo: offer.cargo.clone(),
            seed: offer.seed,
            basic_data: offer.basic_data,
            client: offer.client.clone(),
            status: Status::Active,
            settled_day: None,
            done: false,
            side: offer.side,
            overdue_noticed: false,
        };
        self.missions.push(mission);
        self.next_id = id.saturating_add(1);
        self.record(day, EntryKind::Accepted, text.clone());
        self.push_page(PageKind::Message, day, text.clone());
        Ok(Accepted { id, text })
    }

    /// F303: settle a contract that is due at `system`. F303 itself has no
    /// deadline test; the source reaches it only for a contract F301/F302 did
    /// not fail. A direct (non-`tick`) settlement after the deadline keeps the
    /// port's 0x9c2c late-arrival page, which the source uses from F300's
    /// removal/military list, and pays nothing.
    pub fn settle(&mut self, id: u32, day: u32, system: u32) -> Result<Outcome, String> {
        let index = self
            .missions
            .iter()
            .position(|mission| mission.id == id)
            .ok_or_else(|| "No such mission.".to_owned())?;
        let (
            kind,
            origin,
            origin_name,
            destination,
            destination_name,
            reward,
            passengers,
            seed,
            basic_data,
            status,
        ) = {
            let mission = &self.missions[index];
            (
                mission.kind,
                mission.origin,
                mission.origin_name.clone(),
                mission.destination,
                mission.destination_name.clone(),
                mission.reward,
                mission.passengers,
                mission.seed,
                mission.basic_data,
                mission.status,
            )
        };
        if status != Status::Active {
            return Err("That mission is no longer active.".to_owned());
        }
        let returned = self.missions[index].done && origin == system;
        if destination != system && !returned {
            return Err("That mission is not for this system.".to_owned());
        }
        let late = day > self.missions[index].deadline_day;
        let (status, text, reward) = if late {
            self.missions[index].status = Status::Expired;
            self.missions[index].settled_day = Some(day);
            self.reputation = self.reputation.saturating_sub(1);
            (
                Status::Expired,
                page_text(
                    0x9c2c,
                    &[
                        Var::Int(0),
                        Var::Int(0),
                        Var::System(system),
                        Var::Seed(seed),
                        Var::Int(0),
                    ],
                    origin,
                    &origin_name,
                    destination,
                    &destination_name,
                ),
                0,
            )
        } else if refuses_payment(basic_data) {
            // F303's first branch: `(basic_data >> 8) & 0x1f == 0x14` is a
            // passage at the `$30` additive tier. The client pays nothing
            // (no reputation step, no military rank) and the passengers leave
            // (the port derives the occupied cabins from active contracts).
            self.missions[index].status = Status::Failed;
            self.missions[index].settled_day = Some(day);
            (
                Status::Failed,
                page_text(
                    0x9c35,
                    &[
                        Var::Int(i64::from(reward)),
                        Var::Int(i64::from(passengers)),
                        Var::System(destination),
                        Var::Seed(seed),
                        Var::Int(i64::from(basic_data)),
                    ],
                    origin,
                    &origin_name,
                    destination,
                    &destination_name,
                ),
                0,
            )
        } else {
            self.missions[index].status = Status::Complete;
            self.missions[index].settled_day = Some(day);
            let code = match kind {
                Kind::Parcel => 0x9c34,
                Kind::MilitaryDelivery | Kind::MilitaryStrike => 0x9c33,
                Kind::Passenger if passengers <= 1 => 0x9c31,
                Kind::Passenger => 0x9c32,
                Kind::Assassination => 0x9c34,
            };
            if !matches!(kind, Kind::MilitaryDelivery | Kind::MilitaryStrike) {
                self.reputation = self.reputation.saturating_add(1).min(MAX_REPUTATION);
            } else {
                // F303's seed==0 branch awards the military rank points. The
                // contract's `basic_data` high bits are F509's tier flags; a
                // delivery gives the 0x9c33 page and the rank page follows.
                self.military_points = self.military_points.saturating_add(500);
                self.rank = rank_for(self.military_points);
            }
            let text = page_text(
                code,
                &[
                    Var::Int(i64::from(reward)),
                    Var::Int(i64::from(passengers)),
                    Var::System(destination),
                    Var::Seed(seed),
                    Var::Int(i64::from(basic_data)),
                ],
                origin,
                &origin_name,
                destination,
                &destination_name,
            );
            (Status::Complete, text, reward)
        };
        let journal_kind = match status {
            Status::Complete => EntryKind::Completed,
            Status::Failed => EntryKind::Failed,
            _ => EntryKind::Expired,
        };
        self.record(day, journal_kind, text.clone());
        self.push_page(PageKind::Message, day, text.clone());
        if status == Status::Complete
            && matches!(kind, Kind::MilitaryDelivery | Kind::MilitaryStrike)
        {
            // F304/F305's rank page (F306, song 7). The package constructor
            // does not award a medal: F509's basic_data low bits are 0, so
            // F303 selects `DATA_005115[0]`/`DATA_005116[0]`, both 0xff, and
            // F307's medal page is skipped (`test ebx,ebx; jl`). The medal
            // path exists in `PageKind::Medal` for the medal-bearing military
            // constructors, which are not ported.
            let rank_name = RANK_NAMES[usize::from(self.rank.min(8))].to_owned();
            let rank = page_text(
                0x9c39,
                &[
                    Var::Int(0),
                    // 0x9c39's `0xff 0x19` is FF30WriteFFCodeV1, slot 1.
                    Var::Text(rank_name),
                    Var::System(destination),
                    Var::Seed(0),
                    Var::Int(0),
                ],
                origin,
                &origin_name,
                destination,
                &destination_name,
            );
            self.push_page(PageKind::Rank, day, rank.clone());
            self.record(day, EntryKind::Rank, rank);
        }
        Ok(Outcome {
            id,
            status,
            reward,
            page: self.pages.last().cloned().unwrap_or_else(|| Page {
                kind: PageKind::Message,
                day,
                text: text.clone(),
                unread: true,
            }),
            journal: self.journal.last().cloned().unwrap_or(Entry {
                day,
                kind: journal_kind,
                text,
            }),
        })
    }

    /// F301/F302's periodic check. F301 runs first and marks every contract
    /// past its deadline (0x9c36 for a passenger passage); F302 then tests that
    /// mark **before** the current-system compare, so an overdue contract fails
    /// at any destination with its `-2` reputation step. Only a contract that
    /// is not overdue and due at `system` reaches F303. The caller pays
    /// `reward * 10` for a completion.
    pub fn tick(&mut self, day: u32, system: u32) -> Vec<Outcome> {
        // F301/F302 iterate the passenger list (F290's 0x14-byte records).
        // Removals and military strikes are F300's 0x34-byte list, handled
        // below by the port's existing (0x9c29) approximation.
        let passenger_list =
            |kind: Kind| !matches!(kind, Kind::Assassination | Kind::MilitaryStrike);
        for index in 0..self.missions.len() {
            let mission = &self.missions[index];
            if mission.status == Status::Active
                && passenger_list(mission.kind)
                && mission.overdue(day)
                && !mission.overdue_noticed
            {
                self.notice_overdue(index, day);
            }
        }
        let flagged: Vec<u32> = self
            .missions
            .iter()
            .filter(|mission| {
                mission.status == Status::Active
                    && mission.overdue_noticed
                    && passenger_list(mission.kind)
            })
            .map(|mission| mission.id)
            .collect();
        let removals: Vec<u32> = self
            .missions
            .iter()
            .filter(|mission| {
                mission.status == Status::Active
                    && !passenger_list(mission.kind)
                    && mission.overdue(day)
            })
            .map(|mission| mission.id)
            .collect();
        let due: Vec<u32> = self
            .missions
            .iter()
            .filter(|mission| mission.status == Status::Active && !mission.overdue_noticed)
            // An assassination is settled by destroying its target, not by
            // arriving (see `State::target_destroyed`).
            .filter(|mission| {
                (mission.destination == system
                    && !matches!(mission.kind, Kind::Assassination | Kind::MilitaryStrike))
                    || (mission.done && mission.origin == system)
            })
            .map(|mission| mission.id)
            .collect();
        let mut outcomes = Vec::new();
        for id in flagged {
            if let Ok(outcome) = self.expire(id, day, system) {
                outcomes.push(outcome);
            }
        }
        for id in removals {
            if let Ok(outcome) = self.expire_removal(id, day) {
                outcomes.push(outcome);
            }
        }
        for id in due {
            if let Ok(outcome) = self.settle(id, day, system) {
                outcomes.push(outcome);
            }
        }
        outcomes
    }

    /// F301's pass for one contract: set `basic_data`'s 0x8000 mark and post
    /// 0x9c36 for a passenger passage. The mark is set before the seed/package
    /// tests, so a military contract (seed 0) and a package are marked without
    /// a notice. The reputation step is F302's, not this pass's.
    fn notice_overdue(&mut self, index: usize, day: u32) {
        let mission = &self.missions[index];
        if mission.status != Status::Active || mission.overdue_noticed {
            return;
        }
        let seed = mission.seed;
        let package = mission.basic_data & 0x2000 != 0;
        let vars = [
            Var::Int(i64::from(mission.reward)),
            Var::Int(i64::from(mission.passengers)),
            Var::System(mission.destination),
            Var::Seed(mission.seed),
            Var::Int(i64::from(mission.basic_data)),
        ];
        let (origin, origin_name, destination, destination_name) = (
            mission.origin,
            mission.origin_name.clone(),
            mission.destination,
            mission.destination_name.clone(),
        );
        self.missions[index].overdue_noticed = true;
        // F301's `test ebx,ebx`: a military contract (seed 0) is marked but
        // gets no page.
        if seed == 0 {
            return;
        }
        // F301's `test byte [eax+5],0x20`: a package (bit 0x2000) has no page.
        if !package {
            let text = page_text(
                0x9c36,
                &vars,
                origin,
                &origin_name,
                destination,
                &destination_name,
            );
            self.push_page(PageKind::Message, day, text);
        }
    }

    /// F301 + F302 for one contract: mark it overdue, then fail it. Because
    /// F302 tests the 0x8000 mark before the current system, the failure is
    /// destination-independent: `-2` reputation, 0x9c37, and the contract
    /// leaves the active list (the source memcpys the record out; the port
    /// keeps it as the journal's own record). The one exception is a seed-0
    /// package delivered to its destination: 0x9c38, no pay and no reputation
    /// step (`ffe.asm:24081-24147`).
    pub fn expire(&mut self, id: u32, day: u32, system: u32) -> Result<Outcome, String> {
        let index = self
            .missions
            .iter()
            .position(|mission| mission.id == id)
            .ok_or_else(|| "No such mission.".to_owned())?;
        if self.missions[index].status != Status::Active {
            return Err("That mission is no longer active.".to_owned());
        }
        self.notice_overdue(index, day);
        let mission = &self.missions[index];
        let late_military =
            mission.basic_data & 0x2000 != 0 && mission.seed == 0 && mission.destination == system;
        let vars = [
            Var::Int(i64::from(mission.reward)),
            Var::Int(i64::from(mission.passengers)),
            Var::System(mission.destination),
            Var::Seed(mission.seed),
            Var::Int(i64::from(mission.basic_data)),
        ];
        let (origin, origin_name, destination, destination_name) = (
            mission.origin,
            mission.origin_name.clone(),
            mission.destination,
            mission.destination_name.clone(),
        );
        let (status, code) = if late_military {
            (Status::Expired, 0x9c38)
        } else {
            (Status::Failed, 0x9c37)
        };
        self.missions[index].status = status;
        self.missions[index].settled_day = Some(day);
        if !late_military {
            // F302's JUMP_001807: `sub dword [DATA_008922],2`.
            self.reputation = self.reputation.saturating_sub(2);
        }
        let text = page_text(
            code,
            &vars,
            origin,
            &origin_name,
            destination,
            &destination_name,
        );
        let journal_kind = if late_military {
            EntryKind::Expired
        } else {
            EntryKind::Failed
        };
        self.record(day, journal_kind, text.clone());
        self.push_page(PageKind::Message, day, text.clone());
        Ok(Outcome {
            id,
            status,
            reward: 0,
            page: self.pages.last().cloned().unwrap(),
            journal: self.journal.last().cloned().unwrap(),
        })
    }

    /// F300's removal/military list is a separate 0x34-byte record with its own
    /// deadline page (0x9c29, `ffe.asm:23764-23782`). The port has not modelled
    /// that pass; this keeps the previous approximation for an overdue
    /// assassination or military strike: 0x9c29 and one reputation step.
    fn expire_removal(&mut self, id: u32, day: u32) -> Result<Outcome, String> {
        let index = self
            .missions
            .iter()
            .position(|mission| mission.id == id)
            .ok_or_else(|| "No such mission.".to_owned())?;
        if self.missions[index].status != Status::Active {
            return Err("That mission is no longer active.".to_owned());
        }
        let mission = &mut self.missions[index];
        mission.status = Status::Expired;
        mission.settled_day = Some(day);
        let text = page_text(
            0x9c29,
            &[
                Var::Int(i64::from(mission.reward)),
                Var::Int(i64::from(mission.passengers)),
                Var::System(mission.destination),
                Var::Seed(mission.seed),
                Var::Int(i64::from(mission.basic_data)),
            ],
            mission.origin,
            &mission.origin_name,
            mission.destination,
            &mission.destination_name,
        );
        self.reputation = self.reputation.saturating_sub(1);
        self.record(day, EntryKind::Expired, text.clone());
        self.push_page(PageKind::Message, day, text.clone());
        Ok(Outcome {
            id,
            status: Status::Expired,
            reward: 0,
            page: self.pages.last().cloned().unwrap(),
            journal: self.journal.last().cloned().unwrap(),
        })
    }

    /// A deliberate abandonment. F485 removes the contract and takes `-8`
    /// (`ffe.asm:32983`); the port's existing page is 0x9c37 and its loss is
    /// the one-step approximation of that `-8`.
    pub fn abandon(&mut self, id: u32, day: u32) -> Result<Outcome, String> {
        let index = self
            .missions
            .iter()
            .position(|mission| mission.id == id)
            .ok_or_else(|| "No such mission.".to_owned())?;
        if self.missions[index].status != Status::Active {
            return Err("That mission is no longer active.".to_owned());
        }
        let mission = &mut self.missions[index];
        mission.status = Status::Abandoned;
        mission.settled_day = Some(day);
        let text = page_text(
            0x9c37,
            &[
                Var::Int(i64::from(mission.reward)),
                Var::Int(i64::from(mission.passengers)),
                Var::System(mission.destination),
                Var::Seed(mission.seed),
                Var::Int(i64::from(mission.basic_data)),
            ],
            mission.origin,
            &mission.origin_name,
            mission.destination,
            &mission.destination_name,
        );
        self.reputation = self.reputation.saturating_sub(1);
        self.record(day, EntryKind::Abandoned, text.clone());
        self.push_page(PageKind::Message, day, text.clone());
        Ok(Outcome {
            id,
            status: Status::Abandoned,
            reward: 0,
            page: self.pages.last().cloned().unwrap(),
            journal: self.journal.last().cloned().unwrap(),
        })
    }

    /// The active contracts, in acceptance order.
    /// F314 for a script (kind 7): a new record in the contracts list, with
    /// an id one above the list's highest; `-1` when the list is full.
    pub fn add_script_contract(&mut self, system: u32) -> i32 {
        let removals = self
            .active()
            .filter(|m| matches!(m.kind, Kind::Assassination | Kind::MilitaryStrike));
        let ids: Vec<i32> = removals
            .map(|m| m.id as i32)
            .chain(self.script_contracts.iter().map(|c| c.id))
            .collect();
        if ids.len() >= MAX_CONTRACTS {
            return -1;
        }
        let id = ids.iter().copied().max().unwrap_or(0).max(0) + 1;
        self.script_contracts.push(ScriptContract { id, system });
        id
    }

    /// F315: remove the scripts' record `id` (nothing when there is none).
    pub fn remove_script_contract(&mut self, id: i32) {
        self.script_contracts.retain(|contract| contract.id != id);
    }

    /// F290 for a script (statement 49): a delivery contract the scripts
    /// made, with F290's five words (`+4` basic data, `+0` reward, `+8`
    /// destination, `+0xc` deadline day, `+0x10` name seed). A basic word
    /// whose low three bits are set carries that many passengers plus one,
    /// as F638's do; otherwise it is a parcel.
    #[allow(clippy::too_many_arguments)]
    pub fn add_script_delivery(
        &mut self,
        basic_data: u32,
        reward: u32,
        destination: u32,
        destination_name: &str,
        deadline_day: u32,
        seed: u32,
        day: u32,
        origin: u32,
        origin_name: &str,
    ) -> u32 {
        let id = self.next_id.max(1);
        let passengers = (basic_data & 7) as u8;
        self.missions.push(Mission {
            id,
            kind: if passengers == 0 {
                Kind::Parcel
            } else {
                Kind::Passenger
            },
            code: 0,
            origin,
            origin_name: origin_name.to_owned(),
            destination,
            destination_name: destination_name.to_owned(),
            reward,
            accepted_day: day,
            deadline_day,
            passengers: if passengers == 0 { 0 } else { passengers + 1 },
            cargo: String::new(),
            seed,
            basic_data,
            client: String::new(),
            status: Status::Active,
            settled_day: None,
            done: false,
            side: 0,
            overdue_noticed: false,
        });
        self.next_id = id.saturating_add(1);
        id
    }

    pub fn active(&self) -> impl Iterator<Item = &Mission> {
        self.missions
            .iter()
            .filter(|mission| mission.status == Status::Active)
    }

    /// Whether an offer is already an active contract. The military list is
    /// regenerated from the day's stream, so this is how the screen removes an
    /// accepted row (F509's `DATA_009004` slot).
    pub fn is_offer_active(&self, offer: &Offer) -> bool {
        self.active().any(|mission| same_offer(mission, offer))
    }

    /// Whether the offer was taken today, active or settled. The regenerated
    /// military list must not offer the same contract twice in one day; the
    /// next day's stream may offer it again.
    pub fn is_offer_known(&self, offer: &Offer, day: u32) -> bool {
        self.missions
            .iter()
            .any(|mission| mission.accepted_day == day && same_offer(mission, offer))
    }

    /// Drop settled contracts beyond the journal's record (the original
    /// decrements the count on completion; keeping them is the port's journal).
    pub fn sweep(&mut self) {
        if self.missions.len() > MAX_MISSIONS.saturating_mul(2) {
            let keep = self
                .missions
                .iter()
                .rev()
                .take(MAX_MISSIONS)
                .cloned()
                .collect::<Vec<_>>();
            self.missions = keep.into_iter().rev().collect();
        }
        if self.journal.len() > MAX_JOURNAL {
            let drop = self.journal.len() - MAX_JOURNAL;
            self.journal.drain(..drop);
        }
        if self.pages.len() > MAX_PAGES {
            let drop = self.pages.len() - MAX_PAGES;
            self.pages.drain(..drop);
        }
    }

    fn record(&mut self, day: u32, kind: EntryKind, text: String) {
        self.journal.push(Entry { day, kind, text });
        self.sweep();
    }

    fn push_page(&mut self, kind: PageKind, day: u32, text: String) {
        self.pages.push(Page {
            kind,
            day,
            text,
            unread: true,
        });
        self.sweep();
    }

    /// The validation `Flight::validate` runs. Bounds only, no invented values.
    pub fn validate(&self) -> bool {
        self.missions.len() <= MAX_MISSIONS * 2
            && self.journal.len() <= MAX_JOURNAL
            && self.pages.len() <= MAX_PAGES
            && (-0x80..=MAX_REPUTATION).contains(&self.reputation)
            && self.rank <= 8
            && self.missions.iter().all(|mission| {
                mission.reward <= 100_000_000
                    && mission.passengers <= 7
                    && mission.accepted_day <= mission.deadline_day
                    && mission.cargo.len() <= 256
                    && mission.origin_name.len() <= 64
                    && mission.destination_name.len() <= 64
            })
            && self.journal.iter().all(|entry| entry.text.len() <= 1024)
            && self.pages.iter().all(|page| page.text.len() <= 1024)
    }
}

/// F521's reputation roll. The source opens the passenger/parcel reply page
/// (`0x991d`), draws `F922(9)` and compares its negation with `DATA_008922`:
/// `jng` keeps the reply list, so only a negative reputation can refuse
/// (F520's `0x991e`). `draw` is that `F922(9)` word.
pub fn f521_refuses(reputation: i32, draw: u32) -> bool {
    (draw as i32).saturating_neg() > reputation
}

/// F520's `0x991e` refusal page for `offer`:
/// "I'm <client> and I need <passage>, but I won't deal with someone of your
/// reputation."
pub fn refusal_text(offer: &Offer, origin: u32, origin_name: &str) -> String {
    page_text(
        0x991e,
        &[
            Var::Int(i64::from(offer.reward)),
            Var::Text(offer.cargo.clone()),
            Var::System(offer.destination),
            Var::Seed(offer.seed),
            Var::Int(i64::from(offer.basic_data)),
        ],
        origin,
        origin_name,
        offer.destination,
        &offer.destination_name,
    )
}

/// F521's suggested responses, F522's `0x9833..0x983f` range in order.
pub const REPLIES: [u16; 13] = [
    0x9833, 0x9834, 0x9835, 0x9836, 0x9837, 0x9838, 0x9839, 0x983a, 0x983b, 0x983c, 0x983d, 0x983e,
    0x983f,
];

/// F522's `0x983f` "Hang up." row, the only one left after F520/F534.
pub const HANG_UP: u16 = 0x983f;

/// F529's three destinations that need a permit: the system code and the
/// `DATA_009091` bit the permit occupies (`DATA_005242`'s 13..15:
/// "Ross 128 Prison Permit", "Guardian's Permit", "Commune Visitor's
/// Permit").
pub const PERMIT_SYSTEMS: [(u32, u8); 3] =
    [(0x0000_0aa4_b718, 14), (0x16a4_9718, 13), (0x0aa4_971a, 15)];

/// F529: the permit bit a destination needs, if any. The source calls F1094's
/// arrival check with the same three codes.
pub fn permit_for(destination: u32) -> Option<u8> {
    PERMIT_SYSTEMS
        .iter()
        .find(|(code, _)| *code == destination)
        .map(|(_, bit)| *bit)
}

/// One row of F521's page: the response code (`0x9833..0x983f`) and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplyRow {
    pub code: u16,
    pub text: String,
}

/// What a suggested response asks F521's page to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplyStep {
    /// F353/F352: the advertiser answers on the console (`0x992d`'s page is the
    /// advertisement followed by the answer) and the response list stays.
    Answer(String),
    /// F527: hang up; F453 shows the board again.
    HangUp,
    /// F526: the original request (`0x991d`) again.
    Repeat(String),
    /// F534: accept the offer. The caller checks the roster and cabins, clears
    /// the advertisement (F535) and records the contract (F290).
    Accept,
}

/// F521's reply conversation (the passenger/parcel negotiation): F453's entry fields the responses mutate, plus
/// the source's two negotiation globals `DATA_009059` (flags) and
/// `DATA_009060` (the money).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Negotiation {
    /// The board row the conversation started from (F535 clears it).
    pub row: usize,
    /// The offer read from the advertisement. Its `reward` stays the advert's
    /// original money; `money` is the negotiated value.
    pub offer: Offer,
    /// `DATA_009060`: the money the responses change (F530's ten per cent,
    /// F534's halved/all-paid terms).
    pub money: u32,
    /// `DATA_009059`: bit 0x10 a permit was asked for (F528), 0x20 the ten per
    /// cent was granted (F530), 0x40 the all-now terms (F532), 0x80 the
    /// half-now terms (F531).
    pub flags: u8,
    /// F520's refusal or F534's acceptance: the page only offers "Hang up.".
    pub closed: bool,
}

impl Negotiation {
    /// F521: the reply page for board row `row` of `offer`. `closed` opens
    /// F520's refusal page (only "Hang up.") instead of the suggested
    /// responses.
    pub fn open(offer: &Offer, row: usize, closed: bool) -> Self {
        Self {
            row,
            offer: offer.clone(),
            money: offer.reward,
            flags: 0,
            closed,
        }
    }

    /// F522's response codes: the thirteen, less F533's "How many of you are
    /// there?" for a parcel (`+0x11 & 0x20`, the `0x2000` package bit), or the
    /// single "Hang up." once the conversation is over.
    pub fn codes(&self) -> Vec<u16> {
        if self.closed {
            return vec![HANG_UP];
        }
        REPLIES
            .iter()
            .copied()
            .filter(|code| !(*code == 0x9834 && self.offer.basic_data & 0x2000 != 0))
            .collect()
    }

    /// F327's entry texts for [`Self::codes`].
    pub fn rows(&self, origin: u32, origin_name: &str) -> Vec<ReplyRow> {
        self.codes()
            .into_iter()
            .map(|code| ReplyRow {
                code,
                text: self.page(code, self.money, origin, origin_name),
            })
            .collect()
    }

    /// F521's `0x991d` request page.
    pub fn intro(&self, origin: u32, origin_name: &str) -> String {
        self.page(0x991d, self.money, origin, origin_name)
    }

    /// F520's `0x991e` refusal page.
    pub fn refusal(&self, origin: u32, origin_name: &str) -> String {
        self.page(0x991e, self.money, origin, origin_name)
    }

    /// Expand one source page with the conversation's current variables:
    /// `+0x4` (money) is slot 0, the advert's `StrVars[1]` slot 1, the
    /// destination slot 2, the seed slot 3 and `basic_data` slot 4.
    fn page(&self, code: u16, money: u32, origin: u32, origin_name: &str) -> String {
        let text = page_text(
            code,
            &[
                Var::Int(i64::from(money)),
                Var::Text(self.offer.cargo.clone()),
                Var::System(self.offer.destination),
                Var::Seed(self.offer.seed),
                Var::Int(i64::from(self.offer.basic_data)),
            ],
            origin,
            origin_name,
            self.offer.destination,
            &self.offer.destination_name,
        );
        // F1348's `0x9203` name handler needs F791's name table, which the
        // page expander does not carry here; `Offer::client` was resolved by
        // the caller that had it, so substitute it for the `???` fallback.
        if !self.offer.client.is_empty() && self.offer.client != "???" {
            text.replace("???", &self.offer.client)
        } else {
            text
        }
    }

    /// F353's console line: the advertisement's text followed by the answer
    /// (`0x992d` prints `StrVars[1]` and the reply).
    fn answer(&self, code: u16, money: u32, origin: u32, origin_name: &str) -> String {
        let advert = self.advert(origin, origin_name);
        let answer = self.page(code, money, origin, origin_name);
        format!("{advert} {answer}")
    }

    /// F353's `StrVars[1]`: the advice re-expanded with the conversation's
    /// current money/seed/basic words (the source prints the advert entry's
    /// code, not a frozen copy).
    pub fn advert(&self, origin: u32, origin_name: &str) -> String {
        let list = [Destination {
            code: self.offer.destination,
            name: self.offer.destination_name.clone(),
            population: 0,
            danger: 0,
        }];
        let world = text_world(origin, origin_name, &list);
        bulletin::render(
            self.offer.code,
            &[
                Var::Int(i64::from(self.money)),
                Var::Text(self.offer.cargo.clone()),
                Var::System(self.offer.destination),
                Var::Seed(self.offer.seed),
                Var::Int(i64::from(self.offer.basic_data)),
            ],
            &world,
        )
    }

    /// One suggested response: F533, F524-F532 and F527's hang-up. F534 is
    /// [`Self::accept`], which needs the caller's roster and cabin state.
    pub fn respond(
        &mut self,
        code: u16,
        reputation: i32,
        origin: u32,
        origin_name: &str,
    ) -> ReplyStep {
        let money = self.money;
        match code {
            0x9833 => ReplyStep::Accept,
            // F533: "There are <count> of us." (F533 writes the passenger
            // count into the entry's money slot before F353 prints it).
            0x9834 => {
                let passengers = (self.offer.basic_data & 7).saturating_add(1);
                ReplyStep::Answer(self.answer(0x9920, passengers, origin, origin_name))
            }
            // F524: "Why so much money?" — the passage reason from the
            // additive index, and the source's `+0x24 -= 0x1234` seed step.
            0x9835 => {
                let index = u16::try_from((self.offer.basic_data >> 9) & 0x1f).unwrap_or(0);
                self.offer.seed = self.offer.seed.wrapping_sub(0x1234);
                ReplyStep::Answer(self.answer(0x997e + index, money, origin, origin_name))
            }
            // F525: "Will there be any problems?" — the danger answer.
            0x9836 => {
                let index = u16::try_from((self.offer.basic_data >> 9) & 0xf).unwrap_or(0);
                self.offer.seed = self.offer.seed.wrapping_sub(0x1234);
                ReplyStep::Answer(self.answer(0x999e + index, money, origin, origin_name))
            }
            // F527: the three hang-up replies and "Hang up.".
            0x9837 | 0x983d | 0x983e | HANG_UP => ReplyStep::HangUp,
            // F526: the original request again.
            0x9838 => ReplyStep::Repeat(self.intro(origin, origin_name)),
            // F528: "Do I need a permit, and if so can I have one?". F529
            // names the destination's permit; F528 records that one was asked
            // for (bit 0x10) so F534 grants it.
            0x9839 => {
                if permit_for(self.offer.destination).is_some() {
                    self.flags |= 0x10;
                    ReplyStep::Answer(self.answer(0x9930, money, origin, origin_name))
                } else {
                    ReplyStep::Answer(self.answer(0x992f, money, origin, origin_name))
                }
            }
            // F530: "I want more money" — reputation 5 or better gets a ten
            // per cent rise, once; below it, 0x9921 restates the money.
            0x983a => {
                if reputation >= MAX_REPUTATION {
                    if self.flags & 0x20 != 0 {
                        ReplyStep::Answer(self.answer(0x9926, money, origin, origin_name))
                    } else {
                        self.flags |= 0x20;
                        self.money = self.money.saturating_add(self.money / 10);
                        ReplyStep::Answer(self.answer(0x9927, self.money, origin, origin_name))
                    }
                } else {
                    ReplyStep::Answer(self.answer(0x9921, money, origin, origin_name))
                }
            }
            // F531: "I want half the money now".
            0x983b => {
                if refuses_payment(self.offer.basic_data) {
                    ReplyStep::Answer(self.answer(0x9922, money, origin, origin_name))
                } else if self.flags & 0x80 != 0 {
                    ReplyStep::Answer(self.answer(0x9926, money, origin, origin_name))
                } else if self.asks_too_much(2 - reputation) {
                    ReplyStep::Answer(self.answer(0x9925, money, origin, origin_name))
                } else {
                    self.flags |= 0x80;
                    ReplyStep::Answer(self.answer(0x9923, money, origin, origin_name))
                }
            }
            // F532: "I want all the money now".
            0x983c => {
                if refuses_payment(self.offer.basic_data) {
                    ReplyStep::Answer(self.answer(0x9922, money, origin, origin_name))
                } else if self.flags & 0x80 != 0 {
                    ReplyStep::Answer(self.answer(0x9926, money, origin, origin_name))
                } else if self.asks_too_much(0xa - reputation) {
                    ReplyStep::Answer(self.answer(0x9925, money, origin, origin_name))
                } else {
                    self.flags |= 0x40;
                    ReplyStep::Answer(self.answer(0x9924, money, origin, origin_name))
                }
            }
            _ => ReplyStep::HangUp,
        }
    }

    /// F921's "is this demand too much?" draw: `n` is the source's
    /// `2 - reputation` (half now) or `0xa - reputation` (all now). F921 is a
    /// deterministic scramble of the entry's `basic_data` and seed that returns
    /// `0..n-1` and **mutates both words**; only a zero result accepts, so a
    /// demand is refused with probability `(n-1)/n`.
    fn asks_too_much(&mut self, n: i32) -> bool {
        if n <= 0 {
            return false;
        }
        let mut basic = self.offer.basic_data;
        let mut seed = self.offer.seed;
        let rolled = f921(n as u32, &mut basic, &mut seed);
        self.offer.basic_data = basic;
        self.offer.seed = seed;
        rolled != 0
    }

    /// F534: the negotiated contract. Returns the offer to record (its reward
    /// is what is paid on arrival), the credits paid now, and the permit bit
    /// the destination needs (when 0x9839 asked for one).
    ///
    /// The source's halves are F534's `[+0x4] = [D9060]` then, for the half-now
    /// terms, `[+0x4] /= 2` with the same halved value paid into `DATA_008888`;
    /// the all-now terms zero `[+0x4]` and pay the full money now.
    pub fn acceptance(&self) -> (Offer, u32, Option<u8>) {
        let mut offer = self.offer.clone();
        let mut advance = 0;
        if self.flags & 0x80 != 0 {
            if self.flags & 0x40 != 0 {
                advance = self.money;
                offer.reward = 0;
            } else {
                advance = self.money / 2;
                offer.reward = self.money / 2;
            }
        } else {
            offer.reward = self.money;
        }
        // F534 sets the reply page (0x992a/0x992b/0x992c) and the contract from
        // the same `[+0x4]`; the caller prints it with `acceptance_text`.
        let permit = (self.flags & 0x10 != 0)
            .then(|| permit_for(offer.destination))
            .flatten();
        offer.passengers = if offer.kind == Kind::Passenger {
            ((offer.basic_data & 7) as u8).saturating_add(1)
        } else {
            0
        };
        (offer, advance, permit)
    }

    /// F534's page: 0x992a (paid on arrival), 0x992b (half paid, half on
    /// arrival) or 0x992c (paid in full now).
    pub fn acceptance_text(&self, origin: u32, origin_name: &str) -> String {
        let (offer, advance, _) = self.acceptance();
        let code = if self.flags & 0x80 != 0 {
            if self.flags & 0x40 != 0 {
                0x992c
            } else {
                0x992b
            }
        } else {
            0x992a
        };
        // The page's money is the amount still due on arrival.
        let page_money = if code == 0x992a {
            offer.reward
        } else if code == 0x992b {
            advance
        } else {
            0
        };
        page_text(
            code,
            &[
                Var::Int(i64::from(page_money)),
                Var::Text(offer.cargo.clone()),
                Var::System(offer.destination),
                Var::Seed(offer.seed),
                Var::Int(i64::from(offer.basic_data)),
            ],
            origin,
            origin_name,
            offer.destination,
            &offer.destination_name,
        )
    }

    /// F353's console line for the agreement: the advertisement followed by
    /// the 0x992a/0x992b/0x992c page.
    pub fn acceptance_line(&self, origin: u32, origin_name: &str) -> String {
        format!(
            "{} {}",
            self.advert(origin, origin_name),
            self.acceptance_text(origin, origin_name)
        )
    }
}

/// F921: `n`'s draw from the entry's `basic_data` and seed. The source adds
/// the seed into `basic_data`, rotates the seed left by five, adds it again,
/// swaps the halves of `basic_data` and returns the low word times `n`.
fn f921(n: u32, basic: &mut u32, seed: &mut u32) -> u32 {
    *basic = basic.wrapping_add(*seed);
    *seed = seed.rotate_left(5);
    *basic = basic.wrapping_add(*seed);
    *basic = basic.rotate_left(16);
    ((*basic & 0xffff).wrapping_mul(n)) >> 16
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn post(code: u16, money: i64, destination: u32, basic: i64) -> Post {
        Post {
            code,
            fake: false,
            day: 0,
            date: 0,
            vars: [
                Var::Int(money),
                Var::Text("passage for two people on a fast ship".to_owned()),
                Var::System(destination),
                Var::Seed(42),
                Var::Int(basic),
            ],
            text: "advert".to_owned(),
        }
    }

    #[test]
    fn a_passenger_post_reads_its_f638_fields() {
        let destinations = [Destination {
            code: 0x1234_5678,
            name: "Sol".to_owned(),
            population: 12,
            danger: 1,
        }];
        let world = world(&destinations);
        // basic_data: two passengers (`1` in the low bits), additive 3, no
        // package bit.
        let offer = offer(&post(0x981a, 1234, 0x1234_5678, (3 << 9) | 1), &world).unwrap();
        assert_eq!(offer.kind, Kind::Passenger);
        assert_eq!(offer.destination_name, "Sol");
        assert_eq!(offer.reward, 1234);
        assert_eq!(offer.passengers, 2);
    }

    #[test]
    fn the_package_bit_makes_a_parcel_with_no_cabins() {
        let destinations = [Destination {
            code: 7,
            name: "Hope".to_owned(),
            population: 4,
            danger: 0,
        }];
        let offer = offer(&post(0x981b, 700, 7, 0x2000 | 7), &world(&destinations)).unwrap();
        assert_eq!(offer.kind, Kind::Parcel);
        assert_eq!(offer.passengers, 0);
    }

    #[test]
    fn a_passage_at_the_thirtieth_credit_tier_refuses_to_pay() {
        // F303's first branch: `(basic_data >> 8) & 0x1f == 0x14` is F638's
        // additive index 10 (DATA_006173's $30 tier), for a passenger or a
        // parcel alike. The client pays nothing and reputation does not move.
        assert!(refuses_payment(10 << 9));
        assert!(refuses_payment((10 << 9) | 0x2000 | 1));
        assert!(!refuses_payment(9 << 9));
        // F643's removal title code (0x99b4) and F509's military flags are
        // not this marker.
        assert!(!refuses_payment(0x99b4));
        assert!(!refuses_payment(0x2000));

        let destinations = [Destination {
            code: 0x1234_5678,
            name: "Sol".to_owned(),
            population: 12,
            danger: 1,
        }];
        let offer = offer(
            &post(0x981a, 1500, 0x1234_5678, (10 << 9) | 1),
            &world(&destinations),
        )
        .unwrap();
        let mut state = State::default();
        let id = state.accept(&offer, 4, 1, "Ross 154").unwrap().id;
        state.reputation = 2;
        let outcome = state.settle(id, 7, 0x1234_5678).unwrap();
        assert_eq!(outcome.status, Status::Failed);
        assert_eq!(outcome.reward, 0);
        assert_eq!(state.reputation, 2, "F303 pays no reputation step");
        assert!(outcome.journal.text.contains("I can't pay at the moment"));
        assert_eq!(state.journal.last().unwrap().kind, EntryKind::Failed);
        assert_eq!(state.passengers_aboard(), 0, "the passengers have left");
    }

    #[test]
    fn f521_refuses_only_a_negative_reputation() {
        // `neg F922(9)` against DATA_008922: a non-negative reputation always
        // keeps the reply list.
        for draw in 0..9 {
            assert!(!f521_refuses(0, draw));
            assert!(!f521_refuses(5, draw));
        }
        // -draw > reputation: only the draws below the magnitude refuse.
        for draw in 0..9 {
            assert_eq!(f521_refuses(-1, draw), draw == 0);
            assert_eq!(f521_refuses(-9, draw), draw < 9);
        }
        let destinations = [Destination {
            code: 0x1234_5678,
            name: "Sol".to_owned(),
            population: 12,
            danger: 1,
        }];
        let offer = offer(&post(0x981a, 1500, 0x1234_5678, 1), &world(&destinations)).unwrap();
        let text = refusal_text(&offer, 1, "Ross 154");
        assert!(text.contains("won't deal with someone of your reputation"));
        assert!(
            !text.contains('\u{ff}') && !text.contains('\u{1}'),
            "the page's codes are expanded: {text:?}"
        );
    }

    #[test]
    fn failed_contracts_take_reputation_below_zero() {
        // F302 subtracts 2 from a signed dword with no floor; F303 adds 1 back
        // on a completion.
        let destinations = [Destination {
            code: 0x1234_5678,
            name: "Sol".to_owned(),
            population: 12,
            danger: 1,
        }];
        let offer = offer(&post(0x981a, 1500, 0x1234_5678, 1), &world(&destinations)).unwrap();
        let mut state = State::default();
        let id = state.accept(&offer, 0, 1, "Ross 154").unwrap().id;
        state.expire(id, 1, 1).unwrap();
        assert_eq!(state.reputation, -2);
        assert!(state.validate());
    }

    #[test]
    fn acceptance_records_the_contract_and_the_992a_page() {
        let mut state = State::default();
        let destinations = [Destination {
            code: 0x1234_5678,
            name: "Sol".to_owned(),
            population: 12,
            danger: 1,
        }];
        let offer = offer(&post(0x981a, 1234, 0x1234_5678, 1), &world(&destinations)).unwrap();
        let accepted = state.accept(&offer, 10, 1, "Ross 154").unwrap();
        assert_eq!(accepted.id, 1);
        assert_eq!(state.missions.len(), 1);
        let mission = &state.missions[0];
        assert_eq!(mission.accepted_day, 10);
        assert_eq!(mission.deadline_day, 10 + ACCEPT_DAYS);
        assert_eq!(mission.status, Status::Active);
        assert!(accepted.text.contains("$1234"));
        assert!(accepted.text.contains("Sol"));
        assert_eq!(state.journal.len(), 1);
        assert_eq!(state.pages.len(), 1);
        assert_eq!(state.pages[0].kind, PageKind::Message);
    }

    #[test]
    fn military_strikes_open_by_rank_and_pay_on_return() {
        let destinations = [Destination {
            code: 7,
            name: "Hope".to_owned(),
            population: 4,
            danger: 0,
        }];
        let enemy = [Destination {
            code: 99,
            name: "Achenar".to_owned(),
            population: 6,
            danger: 1,
        }];
        let world = world(&destinations);
        let mut rng = crate::npc::Random::new(5, 6);
        assert!(military_strike_offers(&mut rng, &world, &enemy, 10, 0x100).is_empty());
        let offers = military_strike_offers(&mut rng, &world, &enemy, 10, 0x600);
        let codes: Vec<u16> = offers.iter().map(|offer| offer.code).collect();
        assert_eq!(codes, [0x9857, 0x9855, 0x9856]);
        let photo = &offers[1];
        assert!(photo.reward >= 10000 && photo.reward <= 17000);
        assert_eq!(photo.destination, 99);
        let mut state = State::default();
        let id = state.accept(photo, 10, 1, "Ross 154").unwrap().id;
        assert_eq!(state.bases_in(99), [(id, 0x9855)]);
        // Arriving does not pay, and a kill is not a photograph.
        assert!(state.tick(12, 99).is_empty());
        assert!(state.target_destroyed(id, 12, 99).is_err());
        assert!(state.strike_done(id, 12).is_some());
        assert!(state.bases_in(99).is_empty());
        let outcomes = state.tick(15, 1);
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].reward, photo.reward);
        assert_eq!(state.military_points, 500);
    }

    #[test]
    fn f496s_briefing_lists_the_source_responses_and_pages() {
        let destinations = [Destination {
            code: 99,
            name: "Achenar".to_owned(),
            population: 6,
            danger: 1,
        }];
        let world = world(&destinations);
        let enemy = [Destination {
            code: 99,
            name: "Achenar".to_owned(),
            population: 6,
            danger: 1,
        }];
        let mut rng = crate::npc::Random::new(5, 6);
        let offers = military_strike_offers(&mut rng, &world, &enemy, 10, 0x600);
        let photo = offers.iter().find(|offer| offer.code == 0x9855).unwrap();
        let mut briefing = Briefing::open(
            photo,
            photo.text.clone(),
            10,
            "Sirocco Station".to_owned(),
            "Cobra Mk III".to_owned(),
            "A17-3".to_owned(),
        );
        // F495/F496: accept, the camera answer, the destination, the shared
        // seven, the way back.
        assert_eq!(
            briefing.codes(),
            [
                0x9858, 0x9866, 0x9865, 0x9859, 0x985a, 0x985b, 0x985c, 0x985d, 0x985e, 0x985f,
                0x9860,
            ]
        );
        let rows = briefing.rows();
        assert_eq!(rows[0].text, "\"Agreed, I'll do it Sir.\"");
        assert_eq!(
            rows[1].text,
            format!("\"How do I film the {}?\"", photo.cargo)
        );
        assert_eq!(rows[2].text, format!("\"Where is the {}?\"", photo.cargo));
        assert_eq!(rows[4].text, "Show mission description");
        assert_eq!(rows[10].text, "Return to Mission List");
        // F500: the camera answer page.
        assert_eq!(briefing.respond(0x9866), BriefingStep::Answer);
        assert_eq!(
            briefing.page,
            "We'll fit a camera you can trigger from your console (as often as you like)."
        );
        // F499: the destination place page.
        assert_eq!(briefing.respond(0x9865), BriefingStep::Answer);
        assert!(briefing.page.contains("on the surface of Sirocco Station"));
        assert!(briefing.page.contains("Achenar"));
        // F513: the description again.
        assert_eq!(briefing.respond(0x985a), BriefingStep::Description);
        assert_eq!(briefing.page, photo.text);
        // F509/F498: accepting.
        assert_eq!(briefing.respond(0x9858), BriefingStep::Accept);
        // F515: back to F493's list.
        assert_eq!(briefing.respond(0x9860), BriefingStep::List);
        // The bombing page's two missile answers.
        let bombing = offers.iter().find(|offer| offer.code == 0x9856).unwrap();
        let briefing = Briefing::open(
            bombing,
            bombing.text.clone(),
            10,
            "Sirocco Station".to_owned(),
            "Cobra Mk III".to_owned(),
            "A17-3".to_owned(),
        );
        assert_eq!(briefing.codes().len(), 12);
        assert_eq!(
            briefing.rows()[1].text,
            "\"How do I destroy something so large?\""
        );
        assert_eq!(
            briefing.rows()[2].text,
            "\"Can I use the missile on other targets?\""
        );
        // The package page is F508's eight replies and the way back.
        let package = Offer {
            kind: Kind::MilitaryDelivery,
            code: 0x9854,
            ..photo.clone()
        };
        let briefing = Briefing::open(
            &package,
            package.text.clone(),
            10,
            "Sirocco Station".to_owned(),
            String::new(),
            String::new(),
        );
        assert_eq!(briefing.codes()[0], 0x9858);
        assert_eq!(briefing.codes().len(), 9);
        // The assassination page is F506's: accept, how to find them, their
        // ship, the deadline, the shared seven and the way back.
        let kill = offers.iter().find(|offer| offer.code == 0x9857).unwrap();
        let mut briefing = Briefing::open(
            kill,
            kill.text.clone(),
            10,
            "Sirocco Station".to_owned(),
            "Cobra Mk III".to_owned(),
            "A17-3".to_owned(),
        );
        assert_eq!(briefing.codes()[0], 0x9861);
        assert_eq!(briefing.codes().len(), 12);
        assert_eq!(briefing.codes()[1..5], [0x9862, 0x9863, 0x9864, 0x9859]);
        assert_eq!(briefing.respond(0x9863), BriefingStep::Answer);
        assert!(briefing
            .page
            .contains("will be travelling on a Cobra Mk III"));
        assert!(briefing.page.contains("registration number A17-3"));
        // F467's acceptance and the close: only "Return to Mission List".
        assert_eq!(briefing.respond(0x9861), BriefingStep::Accept);
        briefing.closed = true;
        assert_eq!(briefing.codes(), [0x9860]);
        // F498's two refusals.
        assert_eq!(bombing_refusal(), "You have no empty missile positions.");
        assert_eq!(
            military_contract_full(),
            "There is no more room on your contract list"
        );
    }

    #[test]
    fn f460s_removal_conversation_opens_its_six_rows_and_accepts() {
        let removal = Offer {
            kind: Kind::Assassination,
            code: 0x981f,
            destination: 7,
            destination_name: "Achenar".to_owned(),
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
        let mut briefing = Briefing::open(
            &removal,
            "Greetings. We want double crossing spy in the Achenar system killed, for $5000."
                .to_owned(),
            10,
            "Sirocco Station".to_owned(),
            "Cobra Mk III".to_owned(),
            "A17-3".to_owned(),
        );
        briefing.row = 3;
        assert!(briefing.is_removal());
        assert_eq!(briefing.screen_title(), "Video Link");
        assert_eq!(briefing.codes(), REMOVAL_CODES);
        let rows = briefing.rows();
        assert_eq!(rows[0].text, "\"OK, I'll take the contract.\"");
        assert_eq!(rows[1].text, "\"How do I find double crossing spy?\"");
        assert_eq!(
            rows[4].text,
            "\"Sorry, I'm not going that way\" then hang up."
        );
        assert_eq!(rows[5].text, "Hang up.");
        // F469/F470/F468's three answers.
        assert_eq!(briefing.respond(0x9862), BriefingStep::Answer);
        assert!(briefing
            .page
            .contains("They will be leaving Sirocco Station"));
        assert!(briefing.page.contains("Achenar"));
        assert_eq!(briefing.respond(0x9863), BriefingStep::Answer);
        assert!(briefing.page.contains("travelling on a Cobra Mk III"));
        assert_eq!(briefing.respond(0x9864), BriefingStep::Answer);
        assert!(briefing
            .page
            .contains("The attempt must be made at Sirocco Station"));
        // F527's two ways out.
        assert_eq!(briefing.respond(0x983e), BriefingStep::HangUp);
        assert_eq!(briefing.respond(0x983f), BriefingStep::HangUp);
        // F467 accepts and leaves only F527's "Hang up.".
        assert_eq!(briefing.respond(0x9861), BriefingStep::Accept);
        briefing.closed = true;
        assert_eq!(briefing.codes(), [0x983f]);
        // F459's refusal keeps only "Hang up." too, but prints 0x99c5.
        let mut refused = Briefing::open(
            &removal,
            String::new(),
            10,
            "Sirocco Station".to_owned(),
            String::new(),
            String::new(),
        );
        refused.refuse();
        assert_eq!(refused.codes(), [0x983f]);
        assert_eq!(refused.page, removal_refusal());
        assert!(refused
            .page
            .contains("higher Elite Federation combat rating"));
        // F460's gate and F467's acceptance page.
        assert_eq!(removal_rating(0), 0x1000);
        assert_eq!(removal_rating(0xffff), 0x4fff);
        let accepted = removal_accepted_text(&removal, 1, "Ross 154");
        assert!(accepted.contains("$5000"), "{accepted}");
        assert!(accepted.contains("succesful return"), "{accepted}");
    }

    #[test]
    fn passengers_fill_cabins_and_a_full_ship_is_refused() {
        let mut state = State::default();
        let destinations = [Destination {
            code: 7,
            name: "Hope".to_owned(),
            population: 4,
            danger: 0,
        }];
        let offer = offer(&post(0x981a, 900, 7, 1), &world(&destinations)).unwrap();
        assert!(offer.passengers > 0);
        assert!(state
            .cabin_refusal(&offer, 0)
            .unwrap()
            .contains("cabin space"));
        let cabins = u32::from(offer.passengers);
        assert_eq!(state.cabin_refusal(&offer, cabins), None);
        state.accept(&offer, 0, 1, "Ross 154").unwrap();
        assert_eq!(state.passengers_aboard(), cabins);
        assert!(state.cabin_refusal(&offer, cabins).is_some());
        state.settle(1, 20, 7).unwrap();
        assert_eq!(state.passengers_aboard(), 0);
    }

    #[test]
    fn completion_pays_the_reward_and_raises_reputation() {
        let mut state = State::default();
        let destinations = [Destination {
            code: 7,
            name: "Hope".to_owned(),
            population: 4,
            danger: 0,
        }];
        let offer = offer(&post(0x981a, 900, 7, 1), &world(&destinations)).unwrap();
        state.accept(&offer, 0, 1, "Ross 154").unwrap();
        let outcome = state.settle(1, 20, 7).unwrap();
        assert_eq!(outcome.status, Status::Complete);
        assert_eq!(outcome.reward, 900);
        assert_eq!(state.reputation, 1);
        assert_eq!(state.missions[0].status, Status::Complete);
        assert!(outcome.journal.text.contains("paid in full"));
    }

    #[test]
    fn a_late_delivery_pays_nothing_and_is_0x9c2c() {
        let mut state = State::default();
        let destinations = [Destination {
            code: 7,
            name: "Hope".to_owned(),
            population: 4,
            danger: 0,
        }];
        let offer = offer(&post(0x981a, 900, 7, 1), &world(&destinations)).unwrap();
        state.accept(&offer, 0, 1, "Ross 154").unwrap();
        let outcome = state.settle(1, ACCEPT_DAYS + 1, 7).unwrap();
        assert_eq!(outcome.status, Status::Expired);
        assert_eq!(outcome.reward, 0);
        assert!(outcome.journal.text.contains("too late"));
    }

    #[test]
    fn expiry_writes_f301s_notice_then_f302s_failure() {
        let mut state = State::default();
        let destinations = [Destination {
            code: 7,
            name: "Hope".to_owned(),
            population: 4,
            danger: 0,
        }];
        let offer = offer(&post(0x981a, 900, 7, 1), &world(&destinations)).unwrap();
        state.accept(&offer, 0, 1, "Ross 154").unwrap();
        state.reputation = 3;
        let outcomes = state.tick(ACCEPT_DAYS + 1, 1);
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].status, Status::Failed);
        assert_eq!(outcomes[0].reward, 0);
        // F302's overdue branch is -2.
        assert_eq!(state.reputation, 1);
        assert!(outcomes[0]
            .journal
            .text
            .contains("reporting you to the authorities"));
        assert!(state
            .pages
            .iter()
            .any(|page| page.text.contains("waited a month")));
    }

    #[test]
    fn an_overdue_contract_fails_even_at_its_destination() {
        // F302 tests F301's 0x8000 mark before `DATA_008885 == +0x8`, so an
        // overdue contract fails wherever the player is; the old port settled
        // it with the port-only 0x9c2c at the destination.
        let mut state = State::default();
        let destinations = [Destination {
            code: 7,
            name: "Hope".to_owned(),
            population: 4,
            danger: 0,
        }];
        let offer = offer(&post(0x981a, 900, 7, 1), &world(&destinations)).unwrap();
        let id = state.accept(&offer, 0, 1, "Ross 154").unwrap().id;
        state.reputation = 2;
        let outcomes = state.tick(ACCEPT_DAYS + 1, 7);
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].id, id);
        assert_eq!(outcomes[0].status, Status::Failed);
        assert_eq!(outcomes[0].reward, 0);
        assert_eq!(state.reputation, 0, "F302's -2");
        assert!(state.missions[0].overdue_noticed);
        assert!(outcomes[0]
            .journal
            .text
            .contains("reporting you to the authorities"));
        assert!(
            state
                .pages
                .iter()
                .any(|page| page.text.contains("waited a month")),
            "F301's 0x9c36 notice precedes F302's 0x9c37 failure"
        );
        assert!(!state.active().any(|mission| mission.id == id));
        assert_eq!(state.passengers_aboard(), 0, "the contract is removed");
    }

    #[test]
    fn an_overdue_military_package_gets_f302s_late_arrival_page() {
        // seed 0 + 0x2000 at the destination: F302's JUMP_001804 branch posts
        // 0x9c38 with no pay and no reputation step. Away from the
        // destination the same contract is F302's 0x9c37 failure.
        let offer = Offer {
            kind: Kind::MilitaryDelivery,
            code: 0x9854,
            destination: 7,
            destination_name: "Hope".to_owned(),
            reward: 2000,
            passengers: 0,
            cargo: String::new(),
            seed: 0,
            basic_data: 0x2000,
            deadline_days: ACCEPT_DAYS,
            text: "Deliver a message to the Hope system.".to_owned(),
            client: String::new(),
            side: 0,
        };
        let mut state = State::default();
        state.accept(&offer, 0, 1, "Ross 154").unwrap();
        state.reputation = 2;
        let outcomes = state.tick(ACCEPT_DAYS + 1, 7);
        assert_eq!(outcomes.len(), 1);
        assert_eq!(outcomes[0].status, Status::Expired);
        assert_eq!(outcomes[0].reward, 0);
        assert_eq!(state.reputation, 2, "F301 skips seed 0, F302 pays no step");
        assert!(outcomes[0].journal.text.contains("late arrival"));
        assert_eq!(state.military_points, 0, "F303 did not run");

        let mut state = State::default();
        state.accept(&offer, 0, 1, "Ross 154").unwrap();
        let outcomes = state.tick(ACCEPT_DAYS + 1, 1);
        assert_eq!(outcomes[0].status, Status::Failed);
        assert_eq!(state.reputation, -2);
        assert!(outcomes[0]
            .journal
            .text
            .contains("reporting you to the authorities"));
    }

    #[test]
    fn an_arrival_completes_the_contract_on_tick() {
        let mut state = State::default();
        let destinations = [Destination {
            code: 7,
            name: "Hope".to_owned(),
            population: 4,
            danger: 0,
        }];
        let offer = offer(&post(0x981a, 900, 7, 1), &world(&destinations)).unwrap();
        state.accept(&offer, 0, 1, "Ross 154").unwrap();
        let outcomes = state.tick(2, 7);
        assert_eq!(outcomes.len(), 1);
        assert_eq!(state.missions[0].status, Status::Complete);
        assert_eq!(state.reputation, 1);
    }

    #[test]
    fn a_parcel_completes_with_the_parcel_page() {
        let mut state = State::default();
        let destinations = [Destination {
            code: 7,
            name: "Hope".to_owned(),
            population: 4,
            danger: 0,
        }];
        let offer = offer(&post(0x981b, 700, 7, 0x2000 | 7), &world(&destinations)).unwrap();
        state.accept(&offer, 0, 1, "Ross 154").unwrap();
        let outcome = state.settle(1, 1, 7).unwrap();
        assert!(outcome.journal.text.contains("collected the parcel"));
        assert_eq!(state.missions[0].passengers, 0);
    }

    #[test]
    fn an_assassination_is_settled_like_a_parcel() {
        let mut state = State::default();
        let destinations = [Destination {
            code: 7,
            name: "Hope".to_owned(),
            population: 4,
            danger: 0,
        }];
        let mut post = post(0x981f, 5000, 7, 0);
        post.vars[4] = Var::Text("Mafia boss".to_owned());
        let offer = offer(&post, &world(&destinations)).unwrap();
        assert_eq!(offer.kind, Kind::Assassination);
        assert_eq!(offer.cargo, "Mafia boss", "F643's title is StrVars[4]");
        state.accept(&offer, 0, 1, "Ross 154").unwrap();
        let outcome = state.settle(1, 1, 7).unwrap();
        assert_eq!(outcome.status, Status::Complete);
        assert_eq!(outcome.reward, 5000);
    }

    #[test]
    fn military_completion_adds_f306s_rank_page_and_song() {
        let mut state = State::default();
        let offer = Offer {
            kind: Kind::MilitaryDelivery,
            code: 0x9854,
            destination: 7,
            destination_name: "Hope".to_owned(),
            reward: 2000,
            passengers: 0,
            cargo: String::new(),
            seed: 0,
            basic_data: 0x2000,
            deadline_days: ACCEPT_DAYS,
            text: "Deliver a message to the Hope system.".to_owned(),
            client: String::new(),
            side: 0,
        };
        state.accept(&offer, 0, 1, "Ross 154").unwrap();
        assert!(state.is_offer_active(&offer));
        let outcome = state.settle(1, 1, 7).unwrap();
        assert_eq!(outcome.reward, 2000);
        assert_eq!(state.rank, rank_for(500));
        assert!(!state.is_offer_active(&offer));
        assert!(state.is_offer_known(&offer, 0));
        assert!(!state.is_offer_known(&offer, 1));
        let rank = state
            .pages
            .iter()
            .find(|page| page.kind == PageKind::Rank)
            .unwrap();
        assert!(rank.text.contains("promotion"));
        assert!(rank.text.contains(RANK_NAMES[usize::from(state.rank)]));
        assert_eq!(PageKind::Rank.song(), Some(7));
        // F307's medal page/song 10 is implemented but a package delivery has
        // no medal (`DATA_005115[0]` is 0xff); the path is kept for the
        // medal-bearing constructors.
        assert_eq!(PageKind::Medal.song(), Some(10));
        assert!(!state.pages.iter().any(|page| page.kind == PageKind::Medal));
    }

    #[test]
    fn military_package_offers_use_f632s_reward_and_deadline() {
        let destinations = [Destination {
            code: 7,
            name: "Hope".to_owned(),
            population: 4,
            danger: 0,
        }];
        let world = world(&destinations);
        let mut rng = crate::npc::Random::new(0x1234, 0x5678);
        let offers = military_offers(&mut rng, &world, 100, 0);
        assert_eq!(offers.len(), 1, "only the first tier is open at rank 0");
        let offer = &offers[0];
        assert_eq!(offer.kind, Kind::MilitaryDelivery);
        assert!(offer.reward >= 150);
        assert!(offer.deadline_days >= 14 && offer.deadline_days <= 77);
        assert!(offer.text.contains("Deliver"));
        assert!(offer.text.contains("Hope"));
        // F630/F631's rank gates.
        assert_eq!(military_offers(&mut rng, &world, 0, 0x10).len(), 1);
        assert_eq!(military_offers(&mut rng, &world, 0, 0x11).len(), 2);
        assert_eq!(military_offers(&mut rng, &world, 0, 0x52).len(), 3);
    }

    #[test]
    fn the_state_round_trips_through_serde_and_old_json_defaults() {
        let mut state = State::default();
        let destinations = [Destination {
            code: 7,
            name: "Hope".to_owned(),
            population: 4,
            danger: 0,
        }];
        let offer = offer(&post(0x981a, 900, 7, 1), &world(&destinations)).unwrap();
        state.accept(&offer, 0, 1, "Ross 154").unwrap();
        let json = serde_json::to_string(&state).unwrap();
        let back: State = serde_json::from_str(&json).unwrap();
        assert_eq!(back, state);
        // A save from before this slice has no `missions` field at all.
        let old: State = serde_json::from_str("{}").unwrap();
        assert!(old.missions.is_empty());
        assert!(old.journal.is_empty());
        assert_eq!(old.reputation, 0);
    }

    #[test]
    fn every_message_page_names_its_source_routine() {
        for kind in [PageKind::Message, PageKind::Rank, PageKind::Medal] {
            assert!(kind.citation().contains('F'));
        }
    }

    #[test]
    fn f314_keeps_six_contracts_and_f315_removes_one() {
        let mut state = State::default();
        let ids: Vec<i32> = (0..7).map(|_| state.add_script_contract(9)).collect();
        assert_eq!(ids, [1, 2, 3, 4, 5, 6, -1]);
        state.remove_script_contract(3);
        assert_eq!(state.add_script_contract(9), 7);
        assert_eq!(state.script_contracts.len(), MAX_CONTRACTS);
    }

    #[test]
    fn f290_from_a_script_adds_a_delivery() {
        let mut state = State::default();
        let id = state.add_script_delivery(0, 6000, 22, "Lave", 0x12_1d10, 7, 0x12_1cf7, 1, "Sol");
        let mission = state.active().find(|m| m.id == id).unwrap();
        assert_eq!(mission.kind, Kind::Parcel);
        assert_eq!((mission.reward, mission.deadline_day), (6000, 0x12_1d10));
        let id = state.add_script_delivery(2, 1, 22, "Lave", 0x12_1d10, 7, 0x12_1cf7, 1, "Sol");
        let mission = state.active().find(|m| m.id == id).unwrap();
        assert_eq!((mission.kind, mission.passengers), (Kind::Passenger, 3));
    }

    // ---- F521's reply page (the passenger/parcel negotiation) -------------

    fn negotiation(code: u16, money: i64, basic: i64) -> Negotiation {
        let destinations = [Destination {
            code: 0x1234_5678,
            name: "Sol".to_owned(),
            population: 12,
            danger: 1,
        }];
        let world = world(&destinations);
        let offer = offer(&post(code, money, 0x1234_5678, basic), &world).unwrap();
        Negotiation::open(&offer, 2, false)
    }

    #[test]
    fn f521_offers_the_thirteen_suggested_responses() {
        let reply = negotiation(0x981a, 1200, 1);
        let codes: Vec<u16> = reply.codes();
        assert_eq!(codes, REPLIES);
        let rows = reply.rows(1, "Ross 154");
        assert_eq!(rows.len(), 13);
        assert_eq!(rows[0].text, "\"OK - agreed.\"");
        assert_eq!(rows[12].text, "Hang up.");
        // F522 skips F533's "How many of you are there?" for a parcel
        // (`+0x11 & 0x20`, the 0x2000 package bit).
        let parcel = negotiation(0x981a, 1200, 0x2000);
        assert_eq!(parcel.codes().len(), 12);
        assert!(!parcel.codes().contains(&0x9834));
    }

    #[test]
    fn the_refusal_and_accepted_pages_offer_only_hang_up() {
        let reply = negotiation(0x981a, 1200, 1);
        let refused = Negotiation {
            closed: true,
            ..reply.clone()
        };
        assert_eq!(refused.codes(), [HANG_UP]);
        assert_eq!(refused.rows(1, "Ross 154")[0].text, "Hang up.");
        assert!(refused.refusal(1, "Ross 154").contains("I won't deal with"));
    }

    #[test]
    fn the_money_and_danger_questions_read_the_additive_index() {
        // additive 3: F524's 0x997e + 3 and F525's 0x999e + 3.
        let mut reply = negotiation(0x981a, 1234, (3 << 9) | 1);
        match reply.respond(0x9835, 0, 1, "Ross 154") {
            ReplyStep::Answer(text) => {
                assert!(text.starts_with("WANTED:"), "{text}");
                assert!(text.ends_with("I'm visiting a sick relative."), "{text}");
            }
            other => panic!("{other:?}"),
        }
        match reply.respond(0x9836, 0, 1, "Ross 154") {
            ReplyStep::Answer(text) => assert!(text.ends_with("No."), "{text}"),
            other => panic!("{other:?}"),
        }
        // F524's seed step (`+0x24 -= 0x1234`).
        assert_eq!(reply.offer.seed, 42u32.wrapping_sub(0x1234 * 2));
    }

    #[test]
    fn the_passenger_count_answer_uses_the_low_bits() {
        // Three passengers: `basic & 7` == 2.
        let mut reply = negotiation(0x981a, 1234, 2);
        match reply.respond(0x9834, 0, 1, "Ross 154") {
            ReplyStep::Answer(text) => assert!(text.ends_with("There are 3 of us."), "{text}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn asking_for_more_money_adds_ten_per_cent_once() {
        let mut reply = negotiation(0x981a, 1200, 1);
        // Below DATA_008922's 5 F530 restates the money.
        match reply.respond(0x983a, 4, 1, "Ross 154") {
            ReplyStep::Answer(text) => assert!(text.ends_with("$1200 is all I can pay."), "{text}"),
            other => panic!("{other:?}"),
        }
        assert_eq!(reply.flags, 0);
        match reply.respond(0x983a, MAX_REPUTATION, 1, "Ross 154") {
            ReplyStep::Answer(text) => assert!(text.contains("extra 10%"), "{text}"),
            other => panic!("{other:?}"),
        }
        assert_eq!((reply.money, reply.flags), (1320, 0x20));
        // A second ask is already agreed.
        match reply.respond(0x983a, MAX_REPUTATION, 1, "Ross 154") {
            ReplyStep::Answer(text) => {
                assert!(text.ends_with("We've agreed on payment."), "{text}")
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(reply.money, 1320);
    }

    #[test]
    fn the_half_now_terms_pay_half_and_leave_half_on_arrival() {
        let mut reply = negotiation(0x981a, 1200, 1);
        match reply.respond(0x983b, 1, 1, "Ross 154") {
            ReplyStep::Answer(text) => assert!(text.contains("half once you agree"), "{text}"),
            other => panic!("{other:?}"),
        }
        let (offer, advance, permit) = reply.acceptance();
        assert_eq!((advance, offer.reward, permit), (600, 600, None));
        assert!(reply
            .acceptance_text(1, "Ross 154")
            .contains("$600 has been paid to your account"));
        // F532's all-now row sets 0x40, but F534 pays it only when the 0x80
        // agreement is also set (the two-step conversation); alone the demand
        // leaves the reward on arrival.
        let mut all = negotiation(0x981a, 1200, 1);
        match all.respond(0x983c, 10, 1, "Ross 154") {
            ReplyStep::Answer(text) => assert!(text.contains("trust you at your word"), "{text}"),
            other => panic!("{other:?}"),
        }
        assert_eq!(all.flags, 0x40);
        assert_eq!(all.acceptance().0.reward, 1200);
        assert_eq!(all.acceptance().1, 0);
        // Both bits (F531 then F532 are each refused once 0x80 is set, so the
        // source only reaches this combination in the other order).
        let mut both = negotiation(0x981a, 1200, 1);
        both.respond(0x983c, 10, 1, "Ross 154");
        both.respond(0x983b, 1, 1, "Ross 154");
        assert_eq!(both.flags, 0xc0);
        let (offer, advance, _) = both.acceptance();
        assert_eq!((advance, offer.reward), (1200, 0));
        assert!(both
            .acceptance_text(1, "Ross 154")
            .contains("You have now been paid in full."));
    }

    #[test]
    fn a_low_reputation_advance_runs_the_f921_scramble() {
        // F921 is deterministic on the entry's basic_data/seed; at reputation
        // 0 the half-now demand has a `1/2` chance of refusal. What must hold
        // here is that the source's two-word scramble (F921 writes through
        // both pointers) runs before the decision.
        let mut reply = negotiation(0x981a, 1200, 1);
        let before = (reply.offer.basic_data, reply.offer.seed);
        reply.respond(0x983b, 0, 1, "Ross 154");
        assert_ne!(
            (reply.offer.basic_data, reply.offer.seed),
            before,
            "F921 mutates +0x10 and +0x24"
        );
        // With these words F921(2) returns 0, so the terms are agreed.
        assert_eq!(reply.flags & 0x80, 0x80);
        // A reputation of two or more skips F921 entirely.
        let mut trusted = negotiation(0x981a, 1200, 1);
        let before = (trusted.offer.basic_data, trusted.offer.seed);
        trusted.respond(0x983b, 2, 1, "Ross 154");
        assert_eq!((trusted.offer.basic_data, trusted.offer.seed), before);
        assert_eq!(trusted.flags & 0x80, 0x80);
    }

    #[test]
    fn the_permit_question_grants_the_source_permit_bit() {
        let destinations = [Destination {
            code: PERMIT_SYSTEMS[0].0,
            name: "Facece".to_owned(),
            population: 12,
            danger: 1,
        }];
        let world = world(&destinations);
        let offer = offer(&post(0x981a, 1200, PERMIT_SYSTEMS[0].0, 1), &world).unwrap();
        let mut reply = Negotiation::open(&offer, 0, false);
        match reply.respond(0x9839, 0, 1, "Ross 154") {
            ReplyStep::Answer(text) => assert!(text.ends_with("I'll arrange one."), "{text}"),
            other => panic!("{other:?}"),
        }
        assert_eq!(reply.flags & 0x10, 0x10);
        let (_, _, permit) = reply.acceptance();
        assert_eq!(permit, Some(PERMIT_SYSTEMS[0].1));
        // A destination without one says so and records nothing.
        let mut plain = negotiation(0x981a, 1200, 1);
        match plain.respond(0x9839, 0, 1, "Ross 154") {
            ReplyStep::Answer(text) => {
                assert!(text.ends_with("you don't need a permit."), "{text}")
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(plain.flags & 0x10, 0);
        assert_eq!(plain.acceptance().2, None);
    }

    #[test]
    fn the_hang_up_and_repeat_rows_are_f527_and_f526() {
        let mut reply = negotiation(0x981a, 1200, 1);
        for code in [0x9837u16, 0x983d, 0x983e, 0x983f] {
            assert_eq!(reply.respond(code, 0, 1, "Ross 154"), ReplyStep::HangUp);
        }
        match reply.respond(0x9838, 0, 1, "Ross 154") {
            ReplyStep::Repeat(text) => assert!(text.contains("I need passage"), "{text}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn accepting_records_the_negotiated_contract() {
        let mut reply = negotiation(0x981a, 1200, 1);
        reply.respond(0x983a, MAX_REPUTATION, 1, "Ross 154");
        reply.respond(0x983b, 1, 1, "Ross 154");
        let (offer, advance, _) = reply.acceptance();
        assert_eq!((offer.reward, advance), (660, 660));
        let mut state = State::default();
        let page = reply.acceptance_text(1, "Ross 154");
        let accepted = state.accept_page(&offer, 40, 1, "Ross 154", page).unwrap();
        assert_eq!(accepted.text, reply.acceptance_text(1, "Ross 154"));
        let mission = state.active().find(|m| m.id == accepted.id).unwrap();
        assert_eq!(mission.reward, 660);
        assert_eq!(mission.deadline_day, 40 + ACCEPT_DAYS);
    }
}
