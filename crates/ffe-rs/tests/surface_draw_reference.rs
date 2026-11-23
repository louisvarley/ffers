use ffe_core::model_math::SineTable;
use ffe_core::terrain::{Fault, Terrain};
use ffe_core::terrain_draw::Scene;
use ffe_core::terrain_emit::{draw_planet, Queue, Shading};
use ffe_core::terrain_haze::{Haze, Left, Sky};
use serde::Deserialize;

fn cache_terrain() -> (Vec<u8>, Vec<u8>) {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    (
        cache.asset("base:terrain/tables").unwrap().to_vec(),
        cache.asset("base:terrain/maps").unwrap().to_vec(),
    )
}

fn cache_sine() -> SineTable {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    SineTable::from_bytes(cache.asset("base:math/sine").unwrap()).unwrap()
}

#[derive(Deserialize)]
struct Case {
    case: u32,
    #[serde(default)]
    model: i32,
    #[serde(default)]
    seed: u32,
    #[serde(default)]
    kind: usize,
    #[serde(default)]
    detail: i32,
    #[serde(default)]
    zoom: i32,
    #[serde(default)]
    origin: [i32; 3],
    #[serde(default)]
    vectors: [i32; 9],
    #[serde(default)]
    stream: Vec<u8>,
    fault: bool,
    #[serde(default)]
    overflow: u8,
    #[serde(default)]
    crashed: bool,
    #[serde(default)]
    signal: u8,
    #[serde(default)]
    emits: usize,
    #[serde(default)]
    hash: String,
    #[serde(default)]
    nodes: u32,
    #[serde(default)]
    edges: u32,
    #[serde(default)]
    first: Vec<Vec<i64>>,
    #[serde(default)]
    word0: i32,
    #[serde(default)]
    word8: i32,
    #[serde(default)]
    word10: i32,
    #[serde(default)]
    scale: i32,
    #[serde(default)]
    sky: [u8; 3],
    #[serde(default)]
    haze: [i64; 4],
    #[serde(default)]
    sky_out: [i64; 4],
    #[serde(default)]
    light: [i32; 3],
    #[serde(default)]
    shades: Vec<u8>,
    #[serde(default)]
    words: usize,
    #[serde(default)]
    queue_hash: String,
    #[serde(default)]
    queue: Vec<u32>,
}

fn shading(case: &Case) -> Shading {
    let mut shades = [[0u8; 3]; 8];
    for (k, shade) in shades.iter_mut().enumerate() {
        shade.copy_from_slice(&case.shades[3 * k..3 * k + 3]);
    }
    Shading {
        light: case.light,
        shades,
        detail: case.detail,
        atmosphere: None,
        material: [
            ((case.word0 >> 8) & 15) as u8,
            ((case.word0 >> 4) & 15) as u8,
            (case.word0 & 15) as u8,
        ],
    }
}

fn planet(
    case: &Case,
    scene: &Scene,
    table: &[u8],
    maps: &[u8],
    sine: &SineTable,
    queue: &mut Queue,
) -> Result<(Left, Sky), Fault> {
    let mut terrain = Terrain::new(table, maps);
    let shading = shading(case);
    let record = Haze {
        origin: case.origin,
        axes: case.vectors,
        material: shading.material,
        strength: case.word8.wrapping_shl(16),
        scale: case.word10,
        light: case.light,
        shades: shading.shades,
    };
    let mut sky = Sky {
        colour: case.sky,
        flag: 0,
    };
    let (_, left) = draw_planet(
        &mut terrain,
        scene,
        &case.stream,
        &shading,
        &record,
        case.scale,
        sine,
        &mut sky,
        queue,
    )?;
    Ok((left, sky))
}

fn mix(hash: &mut u64, value: u32) {
    for k in 0..4 {
        *hash ^= u64::from((value >> (8 * k)) & 0xff);
        *hash = hash.wrapping_mul(1_099_511_628_211);
    }
}

/// The original F1876 in draw mode (`--surface-draw`): the triangles F1868 and
/// F1869 hand the emitter (both nodes' screen and view positions, region codes,
/// heights and levels, the normal, the depth key and the level), and the arena
/// counts, for planet-scale patches of the planet models' records and random ones.
#[test]
#[ignore = "Local-only original world data; use tools/reference/verify-world.py"]
fn executed_reference_surface_draw() {
    compare_surface_vectors(350, 0);
}

#[test]
#[ignore = "Local-only near-pad original F1876 vectors: FFE_SURFACE_VECTORS"]
fn executed_reference_near_surface_draw() {
    compare_surface_vectors(3, 0);
}

/// The planet-scale half of the same table: the surface records of the planet
/// models at about 1 to 70 radii, details -1 to 4 (the four near-pad records are
/// cases 0-3). The full 512-case `--surface-draw` run forks a child per case
/// and is Linux-only; this test takes the saved vectors and fails unless the
/// file really holds at least 40 cases beyond 1.4 radii, so the near-pad file
/// alone cannot satisfy it.
#[test]
#[ignore = "Local-only planet-scale original F1876 vectors: FFE_SURFACE_VECTORS"]
fn executed_reference_planet_surface_draw() {
    compare_surface_vectors(40, 40);
}

fn compare_surface_vectors(minimum: usize, minimum_far: usize) {
    let (table, maps) = cache_terrain();
    let sine = cache_sine();
    let input = std::fs::read_to_string(std::env::var_os("FFE_SURFACE_VECTORS").unwrap()).unwrap();
    assert!(input.len() < 256 << 20);
    let (mut cases, mut compared, mut emitted, mut queued, mut overflows, mut far) =
        (0, 0, 0, 0, 0, 0);
    for line in input.lines() {
        let case: Case = serde_json::from_str(line).unwrap();
        cases += 1;
        if case.fault {
            // The original ran out of order queue, or divided by zero, on this one.
            assert!(
                !case.crashed && ((case.overflow != 0 && case.signal == 0) || case.signal == 8),
                "case {}: unexpected crash",
                case.case
            );
            let scene = Scene {
                seed: case.seed,
                origin: case.origin,
                axes: case.vectors,
                kind: case.kind,
                detail: case.detail,
                zoom: case.zoom,
            };
            let mut queue = Queue::default();
            let result = planet(&case, &scene, &table, &maps, &sine, &mut queue);
            let expected = if case.signal == 8 {
                Fault::Division
            } else {
                Fault::QueueFull
            };
            assert_eq!(
                result.err(),
                Some(expected),
                "case {}: the fault",
                case.case
            );
            overflows += 1;
            continue;
        }
        let mut terrain = Terrain::new(&table, &maps);
        let scene = Scene {
            seed: case.seed,
            origin: case.origin,
            axes: case.vectors,
            kind: case.kind,
            detail: case.detail,
            zoom: case.zoom,
        };
        let mut hash = 14_695_981_039_346_656_037u64;
        let mut rows: Vec<Vec<i64>> = Vec::new();
        let mut count = 0usize;
        let drawn = terrain
            .draw(&scene, &case.stream, &mut |t, emission| {
                let mut row: Vec<i64> = Vec::with_capacity(35);
                for handle in emission.nodes {
                    let n = t.node(handle);
                    row.extend([
                        i64::from(n.word(0)),
                        i64::from(n.word(2)),
                        i64::from(n.dword(4)),
                        i64::from(n.dword(8)),
                        i64::from(n.dword(0xc)),
                        i64::from(n.word(0x24)),
                        i64::from(n.word(0x26)),
                        i64::from(n.dword(0x3c)),
                        i64::from(n.dword(0x40)),
                        i64::from(n.dword(0x44)),
                    ]);
                }
                row.extend(emission.normal.map(i64::from));
                row.push(i64::from(emission.key as i32));
                row.push(i64::from(emission.level));
                for value in &row {
                    mix(&mut hash, *value as u32);
                }
                if count < 48 {
                    rows.push(row);
                }
                count += 1;
                Ok(())
            })
            .unwrap_or_else(|fault| panic!("case {}: {fault:?}", case.case));
        let _ = drawn;
        let context = format!("case {} (model {})", case.case, case.model);
        assert_eq!(count, case.emits, "{context}: emissions");
        for (k, (got, want)) in rows.iter().zip(&case.first).enumerate() {
            assert_eq!(got, want, "{context}: emission {k}");
        }
        assert_eq!(format!("{hash:016x}"), case.hash, "{context}: hash");
        assert_eq!(terrain.nodes_out(), case.nodes, "{context}: nodes");
        assert_eq!(terrain.edges_out(), case.edges, "{context}: edges");
        {
            // F1877 and the emitter.
            let mut queue = Queue::default();
            let (left, sky) = planet(&case, &scene, &table, &maps, &sine, &mut queue)
                .unwrap_or_else(|fault| panic!("{context}: {fault:?}"));
            if case.word8 != 0 && std::env::var_os("FFE_HAZE_TRACE").is_some() {
                eprintln!(
                    "case {} w10 {} haze {:?} got [{}, {}, {}, {}] sky {:?} got {:?}",
                    case.case,
                    case.word10,
                    case.haze,
                    left.flag,
                    left.distance,
                    left.colour,
                    left.scale,
                    case.sky_out,
                    (sky.flag, sky.colour)
                );
                continue;
            }
            if case.word8 != 0 {
                assert_eq!(
                    [
                        i64::from(left.flag),
                        i64::from(left.distance),
                        i64::from(left.colour),
                        i64::from(left.scale)
                    ],
                    case.haze,
                    "{context}: haze"
                );
                assert_eq!(
                    [
                        i64::from(sky.flag),
                        i64::from(sky.colour[0]),
                        i64::from(sky.colour[1]),
                        i64::from(sky.colour[2])
                    ],
                    case.sky_out,
                    "{context}: sky"
                );
            }
            if queue.words.len() != case.words {
                eprintln!("got  {:x?}", &queue.words[..queue.words.len().min(60)]);
                eprintln!("want {:x?}", &case.queue[..case.queue.len().min(60)]);
            }
            assert_eq!(queue.words.len(), case.words, "{context}: queue words");
            for (k, (got, want)) in queue.words.iter().zip(&case.queue).enumerate() {
                assert_eq!(got, want, "{context}: queue word {k}");
            }
            let mut hash = 14_695_981_039_346_656_037u64;
            for word in &queue.words {
                mix(&mut hash, *word);
            }
            assert_eq!(
                format!("{hash:016x}"),
                case.queue_hash,
                "{context}: queue hash"
            );
            // The modern engine's recording of each entry's view-space corners must
            // leave the queue as it was and stay attached to its own entry. The
            // original does not guarantee that a recorded corner's projection is the
            // queued point: F1849's coarse branch (both ends coarser than the level)
            // averages the two ends' *screen words* while the node's view position is
            // the average of their positions, and the near-pad cases (detail 1) take
            // that branch. The queue words above are the source's contract; here we
            // check the recording's shape and that each entry keeps at least one
            // corner that does project as queued.
            let mut recorded = Queue::default();
            recorded.views = Some(Vec::new());
            let _ = planet(&case, &scene, &table, &maps, &sine, &mut recorded);
            assert_eq!(
                recorded.words, queue.words,
                "{context}: recording changed the queue"
            );
            let mut averaged = 0usize;
            for (at, corners) in recorded.views.as_ref().unwrap() {
                let entry = queue.words[*at];
                let count = match entry {
                    ffe_core::terrain_emit::TEXTURE_QUAD | ffe_core::terrain_emit::DETAIL_QUAD => 4,
                    ffe_core::terrain_emit::FLAT_TRIANGLE
                    | ffe_core::terrain_emit::TEXTURE_TRIANGLE
                    | ffe_core::terrain_emit::DETAIL_TRIANGLE => 3,
                    other => panic!("{context}: entry {at} has draw routine {other:#x}"),
                };
                assert_eq!(corners.len(), count, "{context}: entry {at} corners");
                let mut exact = 0;
                for (k, corner) in corners.iter().enumerate() {
                    let word = queue.words[at + 1 + k];
                    let point = [word as i16, (word >> 16) as i16];
                    if ffe_core::model_instance::project(*corner) == Some(point) {
                        exact += 1;
                    } else {
                        averaged += 1;
                    }
                }
                // A quad's last corners are F1837's sea crossings, projected from
                // the point they are recorded as; a triangle's three are nodes and
                // may all be averages at close range.
                if count == 4 {
                    assert!(
                        exact > 0,
                        "{context}: entry {at} has no corner that projects to its point"
                    );
                }
            }
            if averaged != 0 {
                eprintln!("{context}: {averaged} corners are F1849 screen averages");
            }
            queued += 1;
        }
        // Distance in planet radii: the mean length of the three axis vectors is
        // the patch's radius, the origin is the eye. The near-pad records sit at
        // the surface (about 1.0 radius); the random planet cases run to 60.
        let radius = (0..3)
            .map(|k| {
                let [x, y, z] = [
                    f64::from(case.vectors[3 * k]),
                    f64::from(case.vectors[3 * k + 1]),
                    f64::from(case.vectors[3 * k + 2]),
                ];
                (x * x + y * y + z * z).sqrt()
            })
            .sum::<f64>()
            / 3.0;
        let distance = (0..3)
            .map(|k| f64::from(case.origin[k]).powi(2))
            .sum::<f64>()
            .sqrt();
        if radius > 0.0 && distance > 1.4 * radius {
            far += 1;
        }
        compared += 1;
        emitted += count;
    }
    assert!(compared > minimum, "too few cases: {compared} of {cases}");
    assert!(
        far >= minimum_far,
        "too few planet-scale cases: {far} of {compared} beyond 1.4 radii"
    );
    eprintln!(
        "compared {compared} of {cases} cases ({emitted} triangles, {far} at planet distance); \
         {queued} queues, {overflows} overflows"
    );
}
