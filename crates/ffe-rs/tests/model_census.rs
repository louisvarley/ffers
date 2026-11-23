//! The last model slots that the native executor cannot run standalone.
//!
//! The 466-slot table has three groups the top-level census reports instead of
//! executing (`ffe-tools model-census`):
//!
//! - 16 child-only programs whose first drawn primitive is an external vertex
//!   (`-1`..`-4`): the parent's cached vertex is only defined when the slot runs
//!   as a child, and the original reads a null parent pointer when it runs
//!   standalone (the 13 slots `verify-station-models.py` records as original
//!   crashes, plus 200, 215, 225, 321, 324 and 460 from the child probe);
//! - model 0x59 (89), whose floppy 1.02/CD 1.01 block is shorter than the
//!   release declarations JJFFE was built from, so the declared extent walks
//!   past a truncated command (the shareware 1.06 data decodes it);
//! - slot 149, four zero bytes in the ship object table (a null entry, not a
//!   model).
//!
//! None of them is a missing opcode: every slot the original can execute at all
//! executes natively. This file locks that partition down, and proves that the
//! 16 child-only slots execute their real programs when they are called as a
//! child (the only way the game draws them) instead of being silently skipped.
//!
//! Local-only (the originals are not committed):
//!
//! ```text
//! cargo test -p ffe-rs --test model_census -- --ignored
//! FFE_CENSUS_CD_GAME=<extracted CD 1.01 folder> cargo test -p ffe-rs \
//!     --test model_census cd_101_census_matches_the_floppy -- --ignored
//! ```

use ffe_convert::embedded::{
    decode_model_descriptor, decode_model_normals, decode_model_vertices, decode_sine_table,
    model_command_bytes,
};
use ffe_core::model::VertexContext;
use ffe_core::model_commands::{decode, VisibilityContext};
use ffe_core::model_execute::{
    execute_scene, ChildEnvironment, ChildModel, ChildModels, ChildOutcome, InstanceContext,
};
use ffe_core::model_instance::Vertices;
use ffe_core::model_math::SineTable;
use ffe_core::model_normals::Normal;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Slots whose top-level run needs the parent instance's cached vertices.
const CHILD_ONLY: [u32; 16] = [
    179, 200, 215, 225, 227, 229, 259, 270, 316, 319, 321, 324, 335, 388, 458, 460,
];
/// Model 0x59: floppy 1.02/CD 1.01 command block is shorter than its declaration.
const TRUNCATED: u32 = 89;
/// The ship-object table's null entry (four zero bytes).
const NULL_SLOT: u32 = 149;
const SLOTS: u32 = 466;
const EXECUTED: usize = 448;

fn game_root() -> PathBuf {
    std::env::var_os("FFE_CENSUS_GAME")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../game"))
}

fn census(root: &Path) -> ffe_tools::census::Census {
    ffe_tools::census::model_census(root)
        .unwrap_or_else(|error| panic!("{}: {error}", root.display()))
}

/// The executable's model-table and sine-table RVAs, selected by its size (the
/// floppy 1.02 and CD 1.01 builds use different offsets; see
/// `ffe_tools::cache::model_layout`).
fn layouts(exe: &[u8]) -> (u32, u32) {
    match exe.len() {
        2_097_152 => (0x0d0cfc, 0x106390),
        2_093_056 => (0x0d0cf8, 0x105334),
        length => panic!("unexpected FIRSTENC.EXE size {length}"),
    }
}

fn read_exe(root: &Path) -> Vec<u8> {
    let path = root.join("FIRSTENC.EXE");
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
#[ignore = "Local-only: reads the original game's FIRSTENC.EXE"]
fn census_partitions_all_466_slots() {
    let root = game_root();
    let report = census(&root);
    assert_eq!(report.models, SLOTS as usize);
    assert_eq!(
        report.executed, EXECUTED,
        "only the child-only, truncated and null slots may fail"
    );

    let mut failures: BTreeMap<u32, (String, String)> = BTreeMap::new();
    for model in &report.reports {
        if let Some((stage, message)) = &model.failure {
            assert!(
                failures
                    .insert(model.index, (stage.clone(), message.clone()))
                    .is_none(),
                "model {} reported twice",
                model.index
            );
        }
    }
    let expected: BTreeSet<u32> = CHILD_ONLY
        .iter()
        .copied()
        .chain([TRUNCATED, NULL_SLOT])
        .collect();
    assert_eq!(failures.keys().copied().collect::<BTreeSet<_>>(), expected);

    for index in CHILD_ONLY {
        let (stage, message) = &failures[&index];
        assert_eq!(stage, "execute", "model {index:#04x}");
        assert!(
            message.starts_with("External vertex -") && message.contains("needs a parent instance"),
            "model {index:#04x} must fail on F1478's missing parent, not on an \
             unsupported command: {message}"
        );
    }
    let (stage, message) = &failures[&TRUNCATED];
    assert_eq!(stage, "decode", "model {TRUNCATED:#04x}");
    assert!(
        message.contains("Truncated model command"),
        "model {TRUNCATED:#04x}: {message}"
    );
    let (stage, message) = &failures[&NULL_SLOT];
    assert_eq!(stage, "import", "slot {NULL_SLOT:#04x}");
    assert!(
        message.contains("null table entry"),
        "slot {NULL_SLOT:#04x}: {message}"
    );

    // Every executed slot really runs its program; none is an empty success.
    for model in &report.reports {
        if model.failure.is_none() {
            assert!(
                model.commands > 0,
                "model {} executed no command",
                model.index
            );
            assert!(model.extent > 0, "model {} has no bytes", model.index);
        }
    }
}

#[test]
#[ignore = "Local-only: needs an extracted CD 1.01 folder in FFE_CENSUS_CD_GAME"]
fn cd_101_census_matches_the_floppy() {
    let cd_root = std::env::var_os("FFE_CENSUS_CD_GAME")
        .map(PathBuf::from)
        .expect("FFE_CENSUS_CD_GAME must name an extracted CD 1.01 game folder");
    let floppy = census(&game_root());
    let cd = census(&cd_root);
    assert_eq!(cd.source_profile, "ffe-cd-1.01-en");
    assert_eq!(cd.models, SLOTS as usize);
    assert_eq!(cd.executed, EXECUTED);
    assert_eq!(cd.blockers, floppy.blockers);
    for (left, right) in floppy.reports.iter().zip(&cd.reports) {
        assert_eq!(left.index, right.index);
        assert_eq!(left.extent, right.extent, "model {} extent", left.index);
        assert_eq!(
            left.vertex_count, right.vertex_count,
            "model {} vertices",
            left.index
        );
        assert_eq!(left.failure, right.failure, "model {} failure", left.index);
        assert_eq!(
            left.commands, right.commands,
            "model {} commands",
            left.index
        );
        assert_eq!(left.faces, right.faces, "model {} faces", left.index);
        assert_eq!(left.ordered, right.ordered, "model {} queue", left.index);
    }
}

struct Loaded {
    commands: Vec<u8>,
    vertices: Vec<u8>,
    vertex_count: u32,
    normals: Vec<u8>,
    normal_count: u32,
    scale: i32,
    scale_offset: i32,
    radius: i32,
}

struct Table(Vec<Result<Loaded, String>>);

impl ChildModels for Table {
    fn model(&self, index: u16) -> Result<ChildModel<'_>, String> {
        let loaded = self
            .0
            .get(usize::from(index))
            .ok_or_else(|| format!("Child model {index} is outside the table."))?
            .as_ref()
            .map_err(|error| format!("Child model {index} not imported: {error}"))?;
        Ok(ChildModel {
            commands: &loaded.commands,
            vertices: &loaded.vertices,
            vertex_count: loaded.vertex_count,
            normals: &loaded.normals,
            normal_count: loaded.normal_count,
            scale: loaded.scale,
            scale_offset: loaded.scale_offset,
            radius: loaded.radius,
        })
    }
}

fn load(exe: &[u8], rva: u32, index: u32) -> Result<Loaded, String> {
    let commands = model_command_bytes(exe, rva, index)?;
    let vertices = decode_model_vertices(exe, rva, index)?;
    let normals = decode_model_normals(exe, rva, index)?;
    let descriptor = decode_model_descriptor(exe, rva, index)?;
    Ok(Loaded {
        commands: commands.bytes,
        vertices: vertices.bytes,
        vertex_count: vertices.vertex_count,
        normals: normals.bytes,
        normal_count: normals.normal_count,
        scale: descriptor.scale as i32,
        scale_offset: descriptor.scale_offset as i32,
        radius: descriptor.radius as i32,
    })
}

/// Every child-only slot is a real part of some model program (not an orphan
/// table entry), and running it as a child executes its own command program.
/// The synthetic parent supplies the slot's position vertex and four extra
/// vertices, so F1478's `-1`..`-5` resolve exactly as they do under the game's
/// real parent; only the parent's scene placement is synthetic.
#[test]
#[ignore = "Local-only: reads the original game's FIRSTENC.EXE"]
fn child_only_slots_execute_with_a_parent_instance() {
    let exe = read_exe(&game_root());
    let (table_rva, sine_rva) = layouts(&exe);
    let models: Vec<Result<Loaded, String>> = (0..SLOTS)
        .map(|index| load(&exe, table_rva, index))
        .collect();

    let mut parents: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for (index, loaded) in models.iter().enumerate() {
        let Ok(loaded) = loaded else { continue };
        let Ok(program) = decode(&loaded.commands, loaded.vertex_count) else {
            continue;
        };
        for submodel in &program.submodels {
            let child = u32::from(submodel.model);
            if CHILD_ONLY.contains(&child) {
                parents.entry(child).or_default().insert(index as u32);
            }
        }
    }

    let sine = SineTable::from_bytes(&decode_sine_table(&exe, sine_rva).unwrap().bytes).unwrap();
    let table = Table(models);
    let normals = vec![
        Normal {
            culled: false,
            colour: [0; 3],
        };
        32768
    ];
    let environment = ChildEnvironment {
        models: &table,
        shades: [[0; 3]; 8],
        light: [0x3000, -0x5000, 0x2000],
        arena: 0,
    };

    for child in CHILD_ONLY {
        let callers = parents.get(&child).unwrap_or_else(|| {
            panic!("child-only model {child:#04x} is not called by any model program")
        });
        assert!(!callers.is_empty());
        let loaded = table.0[child as usize].as_ref().unwrap();
        // Opcode 14: model code >> 5, position vertex 0, frame 0 with bit 7 of
        // the second payload byte adding the extra vertices, then the four
        // extras in resolution order (+4, +2, +3, +5) = 0, 1, 2, 3.
        let word = (child as u16) << 5 | 14;
        let program = [word as u8, (word >> 8) as u8, 0, 0x80, 1, 2, 0, 3, 0, 0];
        let mut live = Vertices::new(&[0u8; 24], 8).unwrap();
        live = live.with_normal_records(&loaded.normals);
        let mut context = VertexContext {
            scale: loaded.scale as u32,
            ..Default::default()
        };
        let visibility = VisibilityContext {
            depth: 2_000,
            scale: loaded.scale,
            detail: [0, 0],
            normals: &normals,
        };
        let instance = InstanceContext {
            position: [0, 0, 2_000],
            generation: 2,
            key_scale: 0,
            operand_scale: loaded.scale,
            textures: true,
        };
        let result = execute_scene(
            &program,
            &mut live,
            &mut context,
            &visibility,
            Some(&sine),
            instance,
            4096,
            &environment,
        )
        .unwrap_or_else(|error| panic!("child-only model {child:#04x}: {error}"));
        let run = result
            .children
            .iter()
            .find(|run| u32::from(run.model) == child)
            .unwrap_or_else(|| {
                panic!(
                    "calling model {child:#04x} queued no child run: {:?}",
                    result.children
                )
            });
        match &run.outcome {
            ChildOutcome::Executed(execution) => {
                assert!(
                    !execution.commands.is_empty(),
                    "child-only model {child:#04x} executed no command"
                );
            }
            other => panic!(
                "child-only model {child:#04x} called by {:?} did not run: {other:?}",
                callers
            ),
        }
    }
}
