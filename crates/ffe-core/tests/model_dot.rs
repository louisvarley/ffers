use ffe_core::model_dot::{
    particle, particle_clipped, point_dot, sprite, DotShape, DotTables, Span, TABLE_BYTES,
};

fn tables() -> DotTables {
    // The original's sizes: the core grows from nothing to nine, the halo from two to ten.
    let mut bytes = vec![0, 2, 2, 3, 4, 6, 7, 8, 9];
    bytes.extend([2, 3, 5, 5, 6, 7, 8, 9, 10]);
    DotTables::new(&bytes).unwrap()
}

fn pixels(spans: &[Span]) -> usize {
    spans.iter().map(|s| s.width as usize).sum()
}

#[test]
fn the_tables_are_validated() {
    assert!(DotTables::new(&[0; TABLE_BYTES - 1]).is_err());
    let mut bad = vec![0u8; TABLE_BYTES];
    bad[3] = 200;
    assert!(DotTables::new(&bad).is_err());
}

#[test]
fn small_particles_are_the_hand_drawn_patterns() {
    let counts: Vec<usize> = (0..6)
        .map(|size| pixels(&particle(100, 50, size)))
        .collect();
    assert_eq!(counts, [1, 4, 5, 9, 12, 13]);
    // Their centre must be inside x 2..=317 and y 2..=155, else nothing is drawn.
    assert!(particle(1, 50, 3).is_empty());
    assert!(particle(100, 156, 3).is_empty());
    assert!(particle(100, 155, 3).len() == 3);
}

#[test]
fn large_particles_are_circles_clipped_to_the_view() {
    let ten = particle(160, 79, 10);
    assert!(pixels(&ten) > 60 && pixels(&ten) < 100, "{}", pixels(&ten));
    assert_eq!(ten.iter().map(|s| s.y).min(), Some(74));
    assert_eq!(ten.iter().map(|s| s.y).max(), Some(83));
    // Beyond the size limit nothing is drawn; off the view the clipped form draws nothing.
    assert!(particle(160, 79, 948).is_empty());
    assert!(particle_clipped(-20, 79, 10).is_empty());
    assert!(!particle_clipped(-3, 79, 10).is_empty());
    assert!(particle_clipped(160, 300, 10).is_empty());
    // The rows stay inside the view and the runs inside the width.
    for span in particle_clipped(2, 3, 30) {
        assert!((0..158).contains(&span.y));
        assert!(span.x >= 0 && span.x + span.width <= 320);
    }
}

#[test]
fn an_unlit_point_is_a_dot_by_its_size() {
    let tables = tables();
    // Radius 2048 at depth 40,000: a size of (2048 << 8) / 40,000 = 13, so one particle.
    let far = point_dot(
        0x2c00,
        1024,
        9,
        [0; 3],
        [160, 79],
        [0, 0],
        40_000,
        &tables,
        [0; 3],
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        far.shape,
        DotShape::One {
            size: 13,
            colour: [12, 0, 0]
        }
    );
    // Radius 2048 at depth 100,000 is 5: a halo of size 6 and a core of size 4, the halo a
    // third of the way to the sky.
    let near = point_dot(
        0x2cc0,
        1024,
        9,
        [0; 3],
        [160, 79],
        [0, 0],
        100_000,
        &tables,
        [3, 3, 3],
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        near.shape,
        DotShape::Two {
            halo_size: 6,
            halo: [9, 9, 1],
            core_size: 4,
            core: [12, 12, 0]
        }
    );
    // Too near to draw, off the view, and a lit material.
    assert_eq!(
        point_dot(
            0x2c00,
            1024,
            9,
            [0; 3],
            [160, 79],
            [0, 0],
            0x3f,
            &tables,
            [0; 3]
        ),
        Ok(None)
    );
    assert_eq!(
        point_dot(
            0x2c00,
            1024,
            9,
            [0; 3],
            [900, 79],
            [0, 0],
            100_000,
            &tables,
            [0; 3]
        ),
        Ok(None)
    );
    assert!(point_dot(
        0x0c00,
        1024,
        9,
        [0; 3],
        [160, 79],
        [0, 0],
        100_000,
        &tables,
        [0; 3]
    )
    .is_err());
}

#[test]
fn a_sprite_is_vertically_symmetric_and_ends_on_a_negative_byte() {
    let at = |x, y, width| Span { x, y, width };
    // F1609 reads each byte before stepping out a row: centre 3, then 2 on
    // rows 4 and 6, then 1 on rows 3 and 7; -1 ends it. The routine's x + 1
    // with runs of 2h - 1 from x + 1 - h centres every row on x.
    let mut spans = sprite(10, 5, &[3, 2, 1, -1]);
    spans.sort_by_key(|span| span.y);
    assert_eq!(
        spans,
        [
            at(10, 3, 1),
            at(9, 4, 3),
            at(8, 5, 5),
            at(9, 6, 3),
            at(10, 7, 1)
        ]
    );
    // DATA_007784's first record, a cross-shaped glint: a 9-pixel centre row,
    // two empty steps (0), then single pixels two rows further out each way.
    let mut glint = sprite(100, 50, &[5, 0, 0, 1, 1, -1, 0, 0, 0]);
    glint.sort_by_key(|span| span.y);
    assert_eq!(
        glint,
        [
            at(100, 46, 1),
            at(100, 47, 1),
            at(96, 50, 9),
            at(100, 53, 1),
            at(100, 54, 1)
        ]
    );
    // Rows outside the view (y >= 0x9e, or negative) are skipped; runs clip.
    assert!(sprite(10, 200, &[2, -1]).is_empty());
    let edge = sprite(0, 157, &[3, 2, -1]);
    assert!(edge.iter().all(|span| span.x >= 0 && span.y < 158));
    assert_eq!(edge.len(), 2, "row 158 is outside the view");
    // A non-positive centre draws nothing; empty data draws nothing.
    assert!(sprite(10, 5, &[-1]).is_empty());
    assert!(sprite(10, 5, &[]).is_empty());
}

mod object_dots {
    use ffe_core::model_dot::{
        object_dot, DotShape, ObjectDot, ObjectDotTables, OBJECT_MODEL_BYTES, OBJECT_TABLE_BYTES,
        SIZE_MAP_BYTES, SPRITE_FIRST,
    };

    /// Sprite records -7..=13 (halo then core). Every half is `[centre, arm, 0xff, ...]`, with
    /// the centre half-width the record's size plus one so a test can tell the records apart.
    fn tables(models: &[[u8; OBJECT_MODEL_BYTES]]) -> ObjectDotTables {
        let mut bytes = Vec::new();
        for size in SPRITE_FIRST..=13 {
            for _half in 0..2 {
                let centre = (size - SPRITE_FIRST + 1) as u8;
                bytes.extend([centre, 1, 0xff, 0, 0, 0, 0, 0, 0]);
            }
        }
        // The size map: sub-pixel sizes 0..=7 (index >> 3 of 0..=255) climb from -7 to 0.
        for index in 0..SIZE_MAP_BYTES {
            bytes.push(((index * 8 / SIZE_MAP_BYTES) as i8 - 7) as u8);
        }
        assert_eq!(bytes.len(), OBJECT_TABLE_BYTES);
        for model in models {
            bytes.extend(model);
        }
        ObjectDotTables::new(&bytes).unwrap()
    }

    const SKY: [u8; 3] = [0, 0, 6];

    #[test]
    fn the_tables_are_validated() {
        let good = tables(&[]);
        assert!(good.model(0).is_none());
        assert!(ObjectDotTables::new(&[0; OBJECT_TABLE_BYTES - 1]).is_err());
        // A sprite half with no end marker, and a size map entry outside -7..=0.
        let mut bytes = vec![1u8; OBJECT_TABLE_BYTES];
        assert!(ObjectDotTables::new(&bytes).is_err());
        for record in 0..21 {
            for half in 0..2 {
                bytes[record * 18 + half * 9 + 2] = 0xff;
            }
        }
        assert!(
            ObjectDotTables::new(&bytes).is_err(),
            "size map entries of 1"
        );
        // Model records must be whole.
        let mut ragged = vec![0u8; OBJECT_TABLE_BYTES];
        ragged.push(1);
        assert!(ObjectDotTables::new(&ragged).is_err());
    }

    #[test]
    fn a_model_reads_its_classes_colour_and_light() {
        // Planet model 134 as the floppy executable has it: far 45, dot 31, colour 8,6,4, lit.
        // A star (139): far 69, dot 38, 0x9f,7,4: its own light.
        let t = tables(&[
            [45, 31, 8, 6, 4, 0, 0, 0],
            [69, 38, 0x9f, 7, 4, 0, 0, 0],
            [0; 8],
        ]);
        let planet = t.model(0).unwrap();
        assert_eq!(
            (planet.far, planet.dot, planet.colour, planet.unlit),
            (45, 31, [8, 6, 4], false)
        );
        let star = t.model(1).unwrap();
        assert_eq!(
            (star.far, star.dot, star.colour, star.unlit),
            (69, 38, [0xf, 7, 4], true)
        );
        assert!(
            t.model(2).is_none(),
            "a descriptor-less slot has no far class"
        );
        assert!(t.model(3).is_none());
    }

    #[test]
    fn a_sub_pixel_lit_planet_is_a_shaded_two_sprite_glint() {
        let t = tables(&[]);
        // Radius 1,000,000 at depth 5,000,000,000: size 0 by the first division; the finer
        // one is (radius << 8) / (depth >> 8) = 13 (13 >> 3 = 1 → map entry -7).
        let dot = object_dot(
            [160, 79],
            0,
            1_000_000,
            1_000_000_000,
            [8, 6, 4],
            false,
            [1, 1, 1],
            &t,
            SKY,
        )
        .unwrap()
        .unwrap();
        let ObjectDot::Sprites {
            x,
            y,
            size,
            halo,
            core,
        } = dot
        else {
            panic!("{dot:?}");
        };
        // F1802 draws at the point less one, in the descriptor colour plus the star's shade,
        // the halo a third of the way to the sky.
        assert_eq!((x, y), (159, 78));
        assert_eq!(core, [9, 7, 5]);
        assert_eq!(
            halo,
            [(9 * 2 + 1) / 3, (7 * 2 + 1) / 3, (5 * 2 + 6 + 1) / 3]
        );
        assert!((-7..=0).contains(&size));
        let spans = dot.spans(&t);
        assert!(!spans.is_empty());
        assert!(spans.iter().any(|(_, c)| *c == halo) && spans.iter().any(|(_, c)| *c == core));
    }

    #[test]
    fn a_star_adds_no_shade_and_a_zero_finer_size_is_one_pixel() {
        let t = tables(&[]);
        // Too small even for the finer division: a one-pixel particle in the plain colour.
        let dot = object_dot(
            [100, 50],
            0,
            1,
            i32::MAX / 2,
            [0xf, 7, 4],
            true,
            [9, 9, 9],
            &t,
            SKY,
        )
        .unwrap()
        .unwrap();
        let ObjectDot::Particle(particle) = dot else {
            panic!("{dot:?}")
        };
        assert_eq!(
            particle.shape,
            DotShape::One {
                size: 0,
                colour: [0xf, 7, 4]
            }
        );
    }

    #[test]
    fn a_dot_a_few_pixels_across_uses_that_sprite_record() {
        let t = tables(&[]);
        let dot = object_dot(
            [160, 79],
            4,
            400,
            100_000,
            [8, 6, 4],
            false,
            [0; 3],
            &t,
            SKY,
        )
        .unwrap()
        .unwrap();
        let ObjectDot::Sprites { size, .. } = dot else {
            panic!("{dot:?}")
        };
        assert_eq!(size, 4);
        // Over nine pixels an unlit star is one particle of that size; a lit body would be the
        // original's shaded sphere, which is not ported.
        let big = object_dot(
            [160, 79],
            12,
            400,
            100_000,
            [8, 6, 4],
            true,
            [0; 3],
            &t,
            SKY,
        )
        .unwrap()
        .unwrap();
        assert!(
            matches!(big, ObjectDot::Particle(p) if p.shape == DotShape::One { size: 12, colour: [8, 6, 4] })
        );
        assert!(object_dot(
            [160, 79],
            12,
            400,
            100_000,
            [8, 6, 4],
            false,
            [0; 3],
            &t,
            SKY
        )
        .is_err());
    }

    #[test]
    fn a_dot_off_the_view_is_not_drawn() {
        let t = tables(&[]);
        for screen in [[-40, 79], [400, 79], [160, -40], [160, 300]] {
            assert_eq!(
                object_dot(screen, 2, 400, 100_000, [8, 6, 4], false, [0; 3], &t, SKY).unwrap(),
                None,
                "{screen:?}"
            );
        }
    }
}

/// F1612 traced by hand from `ffe.asm:96457`: a size of 4 (radius 2, decision 4) fills the table
/// `(x-1, 2), (x-2, 4), (x-2, 4), (x-1, 2)`; the first and the last rows drawn are whole runs and
/// the middle rows are their two end pixels.
#[test]
fn a_circle_of_size_four_is_a_four_by_four_round_block() {
    use ffe_core::model_dot::circle;
    let at = |x, y, width| Span { x, y, width };
    assert_eq!(
        circle(100, 50, 4),
        [
            at(99, 48, 2),
            at(98, 49, 1),
            at(101, 49, 1),
            at(98, 50, 1),
            at(101, 50, 1),
            at(99, 51, 2)
        ]
    );
    // Sizes below 2 draw nothing, and a size of 2 or 3 is a radius of one: two whole rows.
    assert!(circle(10, 10, 1).is_empty() && circle(10, 10, 0).is_empty());
    assert_eq!(circle(10, 10, 2), [at(9, 9, 2), at(9, 10, 2)]);
}

/// F1111's circle (size `0xc`, radius 6): twelve rows from `y - 6` to `y + 5`, whole four-pixel runs
/// top and bottom, then two-pixel rows at widths 6, 8, 10 and 12 (the table traced by hand).
#[test]
fn the_combat_circle_has_f1612s_twelve_rows() {
    use ffe_core::model_dot::circle;
    let spans = circle(159, 79, 12);
    let widths = [4, 6, 8, 10, 12, 12, 12, 12, 10, 8, 6, 4];
    let mut lefts = [0; 12];
    for (row, width) in widths.iter().enumerate() {
        let y = 73 + row as i32;
        let on_row: Vec<&Span> = spans.iter().filter(|span| span.y == y).collect();
        let first = on_row.iter().map(|span| span.x).min().unwrap();
        let last = on_row.iter().map(|span| span.x + span.width).max().unwrap();
        assert_eq!(last - first, *width, "row {y}");
        lefts[row] = first;
        // Whole run at the first and last rows, two single pixels between.
        let whole = row == 0 || row == 11;
        assert_eq!(on_row.len(), if whole { 1 } else { 2 }, "row {y}");
    }
    assert_eq!(
        lefts,
        [157, 156, 155, 154, 153, 153, 153, 153, 154, 155, 156, 157]
    );
}

/// The table is filled completely for every size (the original writes through pointers that must
/// stay inside it, so none may run off its ends: this indexes it directly and would panic), every
/// row of a circle that fits the view's width is drawn with a width, and the circle is one row
/// taller than the table's rows it skips.
#[test]
fn every_circle_fills_its_table_inside_its_bounds() {
    use ffe_core::model_dot::circle;
    for size in 2..=400 {
        // Off the view: the original draws rows only up to `0x9e`, so nothing is drawn.
        assert!(circle(500, 500, size).is_empty(), "size {size}");
        let spans = circle(160, 79, size);
        if size <= 300 {
            let radius = size / 2;
            let visible = (79 + radius).min(0x9e) - (79 - radius).max(0);
            let drawn: std::collections::BTreeSet<i32> = spans.iter().map(|span| span.y).collect();
            assert_eq!(drawn.len() as i32, visible, "size {size}");
            assert!(spans.iter().all(|span| span.width > 0), "size {size}");
        }
    }
}

/// The view clips only vertically: rows above 0 are skipped and the table is entered part-way, so the
/// whole run at the table's first row is not drawn; rows past `0x9e` end the circle early and the last
/// row drawn is then a whole run.
#[test]
fn a_circle_is_cut_at_the_top_and_bottom_of_the_view() {
    use ffe_core::model_dot::circle;
    let spans = circle(160, 2, 12);
    assert_eq!(spans.iter().map(|s| s.y).min(), Some(0));
    // y - r = -4, so the table is entered at its fifth row: two end pixels, not a whole run.
    assert_eq!(spans.iter().filter(|s| s.y == 0).count(), 2);
    let spans = circle(160, 0x9e - 2, 12);
    assert_eq!(spans.iter().map(|s| s.y).max(), Some(0x9d));
    let last: Vec<_> = spans.iter().filter(|s| s.y == 0x9d).collect();
    assert_eq!(last.len(), 1, "the last row drawn is a whole run");
}
