//! Local-only whole-frame scene oracle (`v0.48-14`).
//!
//! `jjffe-smoke --scene-frame` draws a fixed station and three ships through
//! the real F1537/F1680/F1772 into the 320x200 framebuffer, builds the runtime
//! palette with F1330/F1331 and flushes the depth-sorted queue with F1789. Its
//! `scene-frame.bin` is the 64000 indexed pixels + 768-byte palette and the one
//! JSONL row on stdout is the exact camera, objects, tics, light and shades.
//!
//! This test reproduces the same scene through the port's verified
//! `setup_object` (F1680) and `ModelViewer` (F1772), depth-sorts by the same
//! keys (F1789) and draws it with the flight view's rasterisers, then compares
//! every pixel with the original's RGB.
use ffe_core::model::JITTER_SEED;
use ffe_core::model_animation::{setup_object, Camera, Descriptor, Draw, ObjectState};
use ffe_rs::model_viewer::{AnimationInputs, ModelViewer, Shape};
use ffe_rs::screen::Screen;
use serde_json::Value;

fn ints<const N: usize>(value: &Value) -> [i32; N] {
    value
        .as_array()
        .unwrap_or_else(|| panic!("not an array: {value}"))
        .iter()
        .map(|v| v.as_i64().unwrap() as i32)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

fn longs<const N: usize>(value: &Value) -> [i64; N] {
    value
        .as_array()
        .unwrap_or_else(|| panic!("not an array: {value}"))
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

fn cache() -> ffe_content::VerifiedCache {
    ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
        .unwrap()
}

fn read(name: &str) -> Vec<u8> {
    let path = std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set"));
    std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

/// One object of the reference scene with the instance F1680 built for it.
struct SceneObject {
    model: u32,
    instance: Box<ffe_core::model_animation::TopLevel>,
}

#[test]
#[ignore = "Local-only: needs FFE_SCENE_TRACE, FFE_SCENE_FRAME and a cache with object descriptors"]
fn the_scene_frame_matches_the_executed_original() {
    let trace: Value = serde_json::from_str(
        std::str::from_utf8(&read("FFE_SCENE_TRACE"))
            .unwrap()
            .trim(),
    )
    .unwrap();
    assert_eq!(trace["scenario"], "jjffe-scene-frame-v1");
    let tics = trace["tics"].as_u64().unwrap() as u32;
    let light = ints::<3>(&trace["light"]);
    let shades: Vec<[u8; 3]> = trace["shades"]
        .as_array()
        .unwrap()
        .iter()
        .map(|shade| {
            let c = ints::<3>(shade);
            [c[0] as u8, c[1] as u8, c[2] as u8]
        })
        .collect();
    let shades: [[u8; 3]; 8] = shades.try_into().unwrap();
    let camera = Camera {
        matrix: ints::<9>(&trace["camera"]["matrix"]),
        position: longs::<3>(&trace["camera"]["position"]),
    };

    let cache = cache();
    let viewer = ModelViewer::new(&cache).unwrap();
    assert_eq!(viewer.shades(), shades, "the reference's shade table");
    let entries = trace["objects"].as_array().unwrap();
    assert_eq!(entries.len(), 4, "a station and three ships");

    let build = |entry: &Value| -> SceneObject {
        let model = entry["model"].as_u64().unwrap() as u32;
        let descriptor: Descriptor = viewer
            .f1680_descriptor(model)
            .unwrap_or_else(|| panic!("no F1680 descriptor for model {model}"));
        let mut words = [0u16; 64];
        for (slot, word) in words.iter_mut().zip(entry["words"].as_array().unwrap()) {
            *slot = word.as_u64().unwrap() as u16;
        }
        let state = ObjectState {
            matrix: ints::<9>(&entry["matrix"]),
            position: longs::<3>(&entry["position"]),
            scale: entry["scale"].as_i64().unwrap() as i32,
            words,
            ..Default::default()
        };
        match setup_object(&state, &camera, &descriptor, light, &shades) {
            Draw::Model { instance, .. } => SceneObject {
                model,
                instance: Box::new(*instance),
            },
            other => panic!("model {model} is not drawn: {other:?}"),
        }
    };
    let objects: Vec<SceneObject> = entries.iter().map(build).collect();
    assert_eq!(
        objects[0].instance.context.globals[0],
        ffe_core::model_animation::time_word(tics),
        "F1534's global 0"
    );

    let render = |object: &SceneObject| {
        let instance = &object.instance;
        let unit = 2_147_483_648.0f64;
        let rows = std::array::from_fn(|k| {
            std::array::from_fn(|j| f64::from(instance.context.matrix[3 * k + j]) / unit)
        });
        let position = instance.instance.position.map(f64::from);
        let light_view = instance.light.map(|v| f64::from(v) / 32_768.0);
        let inputs = AnimationInputs {
            globals: instance.context.globals,
            locals: instance.context.locals,
            jitter: JITTER_SEED,
            depth_detail: true,
            detail: [0, 0],
        };
        viewer
            .render_scene_with_instance_scale(
                object.model,
                instance.instance.operand_scale as u32,
                instance.instance.key_scale,
                rows,
                position,
                light_view,
                &inputs,
            )
            .unwrap()
    };

    // F1536/F1330: register every object's colours, build one palette, draw.
    viewer.begin_registration();
    for object in &objects {
        assert!(render(object).shapes.is_empty(), "registration only");
    }
    viewer.finish_registration();
    let mut queue: Vec<(u32, Shape)> = Vec::new();
    let mut texts = 0usize;
    let mut skipped = 0usize;
    for object in &objects {
        let frame = render(object);
        texts += frame.texts;
        skipped += frame.skipped;
        for (key, shape) in frame.keys.iter().zip(frame.shapes) {
            queue.push((*key, shape));
        }
    }
    eprintln!(
        "scene objects: {} shapes, {texts} text records, {skipped} skipped",
        queue.len()
    );
    // F1788: descending key, equal keys in allocation order.
    queue.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    let shapes: Vec<Shape> = queue.into_iter().map(|(_, shape)| shape).collect();
    let mut screen = Screen::new(&cache).unwrap();
    screen.scene_frame_clear();
    screen.scene_frame_draw(&viewer, &shapes);
    viewer.end_frame_palette();

    if let Some(dir) = std::env::var_os("FFE_SHOT_DIR") {
        let mut ppm = b"P6\n320 200\n255\n".to_vec();
        for p in screen.logical_pixels() {
            ppm.extend([(p >> 16) as u8, (p >> 8) as u8, *p as u8]);
        }
        std::fs::write(std::path::Path::new(&dir).join("scene-frame.ppm"), ppm).unwrap();
    }

    // Compare with the original's RGB.
    let original = read("FFE_SCENE_FRAME");
    assert_eq!(original.len(), 64_000 + 768);
    let pal = &original[64_000..];
    let mut differing = 0usize;
    let mut worst = 0i32;
    for (i, pixel) in original[..64_000].iter().enumerate() {
        let real = [
            i32::from(pal[3 * usize::from(*pixel)]) * 4,
            i32::from(pal[3 * usize::from(*pixel) + 1]) * 4,
            i32::from(pal[3 * usize::from(*pixel) + 2]) * 4,
        ];
        let port = screen.logical_pixels()[i];
        let ours = [
            ((port >> 16) & 0xff) as i32,
            ((port >> 8) & 0xff) as i32,
            (port & 0xff) as i32,
        ];
        let delta = (0..3).map(|c| (real[c] - ours[c]).abs()).max().unwrap();
        if delta != 0 {
            differing += 1;
            worst = worst.max(delta);
        }
    }
    eprintln!("scene frame: {differing} of 64000 pixels differ, worst channel delta {worst}");
    // The accepted residual (`docs/ACCEPTED_PARITY.md`, `v0.48-14`): the
    // original's F1747 3D text on the station's name board (opcode 10 records
    // the port's viewer counts but does not draw) and a two-pixel station-edge
    // depth tie. Every other pixel, and the whole scene's coverage, is exact.
    assert!(
        differing <= 64,
        "the scene differs from the original's frame in {differing} pixels (worst delta {worst})"
    );
    assert!(worst <= 160, "worst channel delta {worst}");
}
