//! Synthetic opcode 10 text rendering: a two-vertex parent places a string whose
//! glyph program runs over the font model's vertices.
use ffe_core::model_math::SineTable;
use ffe_rs::model_viewer::{GlyphSet, ModelViewer, Shape, ViewerModel};

fn table() -> SineTable {
    let bytes: Vec<u8> = (0..16384i32).flat_map(|i| (i * 4).to_le_bytes()).collect();
    SineTable::from_bytes(&bytes).unwrap()
}

/// Font model (slot 0): vertices 0/1 at the origin and 2/3 at (10, 0, 0).
fn font_model() -> ViewerModel {
    let vertices = [1, 0, 0, 0, 0, 0, 1, 0, 10, 0, 0, 0];
    ViewerModel::new(0, 4, 0, &vertices, &[0, 0], &[], 2)
}

/// One glyph program: a point on vertex 0, then a return whose bits 6.. name the
/// next pen vertex (0, so the pen does not move).
fn glyph_set() -> GlyphSet {
    glyph_set_with_newline(None)
}

fn glyph_set_with_newline(newline_vertex: Option<u32>) -> GlyphSet {
    // Every character in the set aliases the one program (as lowercase aliases
    // uppercase in the original fonts).
    let count = ffe_core::model_commands::GLYPH_CHAR_LIMIT;
    // An 8-byte point record on vertex 0 (material 0x0fff, angle 0, flag 0), then
    // a return word naming pen vertex 0.
    let program = [1u8, 0, 0xff, 0x0f, 0, 0, 0, 0, 0, 0];
    let mut payload: Vec<u8> = std::iter::repeat_n(0u32, count)
        .flat_map(u32::to_le_bytes)
        .collect();
    payload.extend_from_slice(&program);
    GlyphSet::from_payload(&payload, count as u32, 4, 0, newline_vertex).unwrap()
}

/// Parent model (slot 9): vertex 0 at (0, 0, 40) and one text command.
fn text_model(string: u16, glyphs: u8) -> ViewerModel {
    let vertices = [1, 0, 0, 0, 40, 0];
    // 10-byte record: opcode, material, normal, (size | glyph set), vertex,
    // frame high byte, string; then the return.
    let commands: Vec<u8> = [
        10u16,
        0x0fff,                               // material
        u16::from_le_bytes([0, glyphs << 4]), // normal 0, size 0, glyph set
        0,                                    // vertex 0, frame high byte 0
        string,
        0, // return
    ]
    .iter()
    .flat_map(|word| word.to_le_bytes())
    .collect();
    ViewerModel::new(9, 2, 0, &vertices, &commands, &[], 2)
}

#[test]
fn text_records_run_their_glyph_programs_and_move_the_pen() {
    let mut viewer =
        ModelViewer::from_models(vec![font_model(), text_model(0x4000, 0)], table()).unwrap();
    viewer.set_model_strings(vec![b"AA".to_vec()]);
    viewer.set_glyph_sets(vec![Some(glyph_set())]);
    // Sorted by index: slot 0 first, so select the parent.
    assert_eq!(viewer.model().index, 0);
    viewer.step(1);
    assert_eq!(viewer.model().index, 9);

    let frame = viewer.render().unwrap();
    assert_eq!(frame.skipped, 0);
    assert_eq!(frame.texts, 1);
    let points: Vec<_> = frame
        .shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Point(position, _) => Some(*position),
            _ => None,
        })
        .collect();
    assert_eq!(
        points.len(),
        2,
        "one point per character: {:?}",
        frame.shapes
    );

    // A code outside the model string table is reported, not guessed.
    let mut unknown =
        ModelViewer::from_models(vec![font_model(), text_model(0x9c00, 0)], table()).unwrap();
    unknown.set_glyph_sets(vec![Some(glyph_set())]);
    unknown.step(1);
    let frame = unknown.render().unwrap();
    assert_eq!(frame.skipped, 1);
    assert!(frame.shapes.is_empty());

    // A glyph set the cache does not have is skipped too.
    let mut missing =
        ModelViewer::from_models(vec![font_model(), text_model(0x4000, 1)], table()).unwrap();
    missing.set_model_strings(vec![b"AA".to_vec()]);
    missing.set_glyph_sets(vec![Some(glyph_set())]);
    missing.step(1);
    assert_eq!(missing.render().unwrap().skipped, 1);
}

#[test]
fn text_control_codes_move_to_the_font_newline_anchor() {
    let mut viewer =
        ModelViewer::from_models(vec![font_model(), text_model(0x4000, 0)], table()).unwrap();
    viewer.set_model_strings(vec![b"A\nA".to_vec()]);
    viewer.set_glyph_sets(vec![Some(glyph_set_with_newline(Some(2)))]);
    viewer.step(1);
    let frame = viewer.render().unwrap();
    assert_eq!(frame.skipped, 0);
    assert_eq!(frame.texts, 1);
    assert_eq!(
        frame
            .shapes
            .iter()
            .filter(|shape| matches!(shape, Shape::Point(_, _)))
            .count(),
        2
    );

    // Without the cache's anchor, F1747 cannot place the next line and stops.
    let mut missing =
        ModelViewer::from_models(vec![font_model(), text_model(0x4000, 0)], table()).unwrap();
    missing.set_model_strings(vec![b"A\nA".to_vec()]);
    missing.set_glyph_sets(vec![Some(glyph_set())]);
    missing.step(1);
    let frame = missing.render().unwrap();
    assert_eq!(frame.skipped, 1);
    assert_eq!(
        frame
            .shapes
            .iter()
            .filter(|shape| matches!(shape, Shape::Point(_, _)))
            .count(),
        1
    );
}

#[test]
#[ignore = "Local-only: checks source font descriptors in the content cache"]
fn imported_font_newline_anchors_match_the_source_descriptors() {
    let cache_path = std::env::var_os("FFE_TEXT_CACHE").expect("FFE_TEXT_CACHE");
    let cache = ffe_content::read(&std::fs::read(cache_path).unwrap()).unwrap();
    for (index, expected) in [(0, 58), (1, 38), (2, 20)] {
        let id = format!("base:model-glyphs/{index:03}");
        let record = cache
            .manifest()
            .files
            .iter()
            .find(|record| record.id == id)
            .unwrap();
        let ffe_content::AssetKind::GlyphPrograms {
            index: actual_index,
            newline_vertex: Some(actual),
            vertex_count,
            ..
        } = record.asset
        else {
            panic!("font {index} has no imported newline anchor");
        };
        assert_eq!(actual_index, index);
        assert_eq!(actual, expected);
        assert!(actual < vertex_count);
    }
}
