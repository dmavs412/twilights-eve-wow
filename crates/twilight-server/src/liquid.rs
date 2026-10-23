//! **Headless water** (#325): the loaded tiles' ADT `MCLQ` surfaces, which `load_tile_mesh`
//! already builds into each chunk's [`LiquidMesh`]es, sampled for the realm's `liquid_at`. WMO
//! `MLIQ` is left out: the server does not load WMO groups' liquid, so a building's water (the
//! Stormwind canals) is dry here.

use benilla_formats::LiquidMesh;

/// The water surface under `(x, y)` among `liquids`, the lowest where surfaces stack; magma and
/// slime are not water.
pub(crate) fn water_surface_at<'a>(
    liquids: impl Iterator<Item = &'a LiquidMesh>,
    x: f32,
    y: f32,
) -> Option<f32> {
    liquids
        .filter(|l| !l.kind.is_fullbright())
        .filter_map(|l| surface_z_at(l, x, y))
        .min_by(f32::total_cmp)
}

/// The height of `mesh` at `(x, y)`, bilinear over the wet cell containing the point; `None` off
/// the grid or over a dry cell. An MCLQ grid's rows step south (−X) and its columns east (−Y).
fn surface_z_at(mesh: &LiquidMesh, x: f32, y: f32) -> Option<f32> {
    let [cols, rows] = mesh.grid.map(|n| n as usize);
    if cols < 2 || rows < 2 || mesh.positions.len() < cols * rows {
        return None;
    }
    let origin = mesh.positions[0];
    let col_step = origin[1] - mesh.positions[1][1];
    let row_step = origin[0] - mesh.positions[cols][0];
    if col_step <= 0.0 || row_step <= 0.0 {
        return None;
    }
    let fr = (origin[0] - x) / row_step;
    let fc = (origin[1] - y) / col_step;
    let (cells_x, cells_y) = (cols - 1, rows - 1);
    if !(0.0..=cells_y as f32).contains(&fr) || !(0.0..=cells_x as f32).contains(&fc) {
        return None;
    }
    let (row, col) = (
        (fr as usize).min(cells_y - 1),
        (fc as usize).min(cells_x - 1),
    );
    if !mesh.wet.get(row * cells_x + col).copied().unwrap_or(false) {
        return None;
    }
    let (tr, tc) = (fr - row as f32, fc - col as f32);
    let z = |r: usize, c: usize| mesh.positions[r * cols + c][2];
    let top = z(row, col) * (1.0 - tc) + z(row, col + 1) * tc;
    let bottom = z(row + 1, col) * (1.0 - tc) + z(row + 1, col + 1) * tc;
    Some(top * (1.0 - tr) + bottom * tr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use benilla_formats::LiquidKind;

    /// An MCLQ-shaped 9x9 grid from the corner `(100, 200)`, 4 yd cells, its surface at Z 10,
    /// with only the north-west corner cell wet.
    fn corner_pond() -> LiquidMesh {
        let positions = (0..81)
            .map(|n| {
                [
                    100.0 - (n / 9) as f32 * 4.0,
                    200.0 - (n % 9) as f32 * 4.0,
                    10.0,
                ]
            })
            .collect();
        let mut wet = vec![false; 64];
        wet[0] = true;
        LiquidMesh {
            grid: [9, 9],
            shared: vec![false; 64],
            wet,
            positions,
            uvs: vec![[0.0; 2]; 81],
            depths: vec![0.0; 81],
            indices: Vec::new(),
            sound_nibble: 0,
            material_id: None,
            kind: LiquidKind::Still,
        }
    }

    #[test]
    fn a_chunk_with_water_in_its_corner_cell_answers_there_and_not_at_its_centre() {
        let mesh = corner_pond();
        assert_eq!(surface_z_at(&mesh, 98.0, 198.0), Some(10.0));
        assert_eq!(surface_z_at(&mesh, 84.0, 184.0), None);
        assert_eq!(surface_z_at(&mesh, 101.0, 198.0), None, "north of the grid");
    }

    #[test]
    fn magma_is_not_water() {
        let mut mesh = corner_pond();
        mesh.kind = LiquidKind::Magma;
        assert_eq!(water_surface_at([mesh].iter(), 98.0, 198.0), None);
    }
}
