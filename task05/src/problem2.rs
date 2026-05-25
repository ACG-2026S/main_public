/// Tile-based acceleration


use crate::obj_parser::parse_obj;
use nalgebra::{Matrix4, Vector2, Vector4};

/// Renders textured geometry using tile-culling + per-pixel ray intersection.
pub fn problem2() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let obj_path = std::path::Path::new(manifest_dir).join("spot_triangulated.obj");

    let (vtx2xyz, vt2uv, tri2vtx, tri2vt) = parse_obj(&obj_path);
    assert_eq!(tri2vtx.len(), tri2vt.len());
    assert_eq!(tri2vtx.len() % 3, 0);

    let transform_xyz2ndc = camera_transformation();
    let vtx2ndcw: Vec<Vector4<f32>> = vtx2xyz
        .iter()
        .map(|xyz| transform_xyz2ndc * Vector4::new(xyz[0], xyz[1], xyz[2], 1.0))
        .collect();

    let input_tex_path = std::path::Path::new(manifest_dir).join("spot_texture.png");
    let texture = image::open(&input_tex_path)
        .expect("Failed to load texture image")
        .to_rgb8();
    let tex_shape = (texture.width() as usize, texture.height() as usize);
    let tex2rgb: Vec<u8> = texture.into_raw(); // packed RGB bytes

    // ── Output image (RGB, initialised black) ────────────────────────────────
    const IMG_SHAPE: (usize, usize) = (512, 512); // w, h
    // change the tile size to 1,2,4,8,16,32,64,128,256,512
    const TILE_SIZE: usize = 16; // for tile-based rendering

    let mut pix2rgb = vec![0u8; IMG_SHAPE.0 * IMG_SHAPE.1 * 3];

    let start = std::time::Instant::now();
    // Build offset arrays for tile bins.
    let (tile2idx_offset, idx2tri) =
        build_tile2idx_idx2tri(&vtx2ndcw, &tri2vtx, IMG_SHAPE, TILE_SIZE);

    println!(
        "avg triangles par tile: {:.4}",
        idx2tri.len() as f32 / (tile2idx_offset.len() - 1) as f32,
    );
    println!("precomputation_time: {:.4}s", start.elapsed().as_secs_f32());

    let start = std::time::Instant::now();
    for i_pix in 0..IMG_SHAPE.0 * IMG_SHAPE.1 {
        let iw = i_pix % IMG_SHAPE.0;
        let ih = i_pix / IMG_SHAPE.0;
        let iw_tile = iw / TILE_SIZE;
        let ih_tile = ih / TILE_SIZE;
        let i_tile = ih_tile * (IMG_SHAPE.0 / TILE_SIZE) + iw_tile;
        let ray_ndc_xy = Vector2::new(
            ((iw as f32 + 0.5) * 2.0) / IMG_SHAPE.0 as f32 - 1.0,
            1.0 - ((ih as f32 + 0.5) * 2.0) / IMG_SHAPE.1 as f32,
        );
        let mut closest_z = -f32::INFINITY;
        let mut closest_tri = usize::MAX;
        let mut closest_bc = [0f32; 3];
        for &i_tri in &idx2tri[tile2idx_offset[i_tile]..tile2idx_offset[i_tile + 1]] {
            let i0_vtx = tri2vtx[i_tri * 3];
            let i1_vtx = tri2vtx[i_tri * 3 + 1];
            let i2_vtx = tri2vtx[i_tri * 3 + 2];
            if let Some((z_hit, bc)) = intersect_ray_parallel_z_with_triangle_ndcw(
                vtx2ndcw[i0_vtx],
                vtx2ndcw[i1_vtx],
                vtx2ndcw[i2_vtx],
                ray_ndc_xy,
            ) {
                if z_hit > closest_z {
                    closest_z = z_hit;
                    closest_tri = i_tri;
                    closest_bc = bc;
                }
            }
        }
        if closest_tri == usize::MAX {
            continue;
        }
        let iv0 = tri2vtx[closest_tri * 3];
        let iv1 = tri2vtx[closest_tri * 3 + 1];
        let iv2 = tri2vtx[closest_tri * 3 + 2];
        debug_assert!(iv0 < vtx2ndcw.len() && iv1 < vtx2ndcw.len() && iv2 < vtx2ndcw.len());
        let iuv0 = tri2vt[closest_tri * 3];
        let iuv1 = tri2vt[closest_tri * 3 + 1];
        let iuv2 = tri2vt[closest_tri * 3 + 2];
        let uv0 = Vector2::new(vt2uv[iuv0][0], vt2uv[iuv0][1]);
        let uv1 = Vector2::new(vt2uv[iuv1][0], vt2uv[iuv1][1]);
        let uv2 = Vector2::new(vt2uv[iuv2][0], vt2uv[iuv2][1]);
        let uv = uv0 * closest_bc[0] + uv1 * closest_bc[1] + uv2 * closest_bc[2];
        // Texture pixel lookup (wrapping via fractional part)
        let iw_tex = ((uv.x - uv.x.floor()) * tex_shape.0 as f32) as usize;
        let ih_tex = ((1.0 - uv.y - (1.0 - uv.y).floor()) * tex_shape.1 as f32) as usize;
        //
        let i_tex = ih_tex * tex_shape.0 + iw_tex;
        pix2rgb[i_pix * 3] = tex2rgb[i_tex * 3];
        pix2rgb[i_pix * 3 + 1] = tex2rgb[i_tex * 3 + 1];
        pix2rgb[i_pix * 3 + 2] = tex2rgb[i_tex * 3 + 2];
    }

    // --------------------------------------------
    println!("problem2: rendering_time: {:.4}s", start.elapsed().as_secs_f32());

    let output_path = std::path::Path::new(manifest_dir).join("output2.png");
    image::save_buffer(
        &output_path,
        &pix2rgb,
        IMG_SHAPE.0 as u32,
        IMG_SHAPE.1 as u32,
        image::ColorType::Rgb8,
    )
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", output_path.display()));
}


/// Signed 2D triangle area (twice the geometric area).
///
/// The sign encodes winding and is used for point-in-triangle tests.
#[inline]
fn area_of_triangle_2d(p0: Vector2<f32>, p1: Vector2<f32>, p2: Vector2<f32>) -> f32 {
    let e1 = p1 - p0;
    let e2 = p2 - p0;
    e1.x * e2.y - e1.y * e2.x
}

/// Intersects a ray parallel to the NDC z-axis with one triangle in NDCW.
///
/// Returns perspective-correct hit depth in NDC z and perspective-correct
/// barycentric coordinates when the ray hits inside the triangle.
fn intersect_ray_parallel_z_with_triangle_ndcw(
    ndcw0: Vector4<f32>,
    ndcw1: Vector4<f32>,
    ndcw2: Vector4<f32>,
    ray_ndc_xy: Vector2<f32>,
) -> Option<(f32, [f32; 3])> {
    if ndcw0.w.abs() <= f32::EPSILON
        || ndcw1.w.abs() <= f32::EPSILON
        || ndcw2.w.abs() <= f32::EPSILON
    {
        return None;
    }

    let p0 = Vector2::new(ndcw0.x / ndcw0.w, ndcw0.y / ndcw0.w);
    let p1 = Vector2::new(ndcw1.x / ndcw1.w, ndcw1.y / ndcw1.w);
    let p2 = Vector2::new(ndcw2.x / ndcw2.w, ndcw2.y / ndcw2.w);

    let area = area_of_triangle_2d(p0, p1, p2);
    if area.abs() <= f32::EPSILON {
        return None;
    }

    let a0 = area_of_triangle_2d(ray_ndc_xy, p1, p2);
    let a1 = area_of_triangle_2d(ray_ndc_xy, p2, p0);
    let a2 = area_of_triangle_2d(ray_ndc_xy, p0, p1);
    let eps = 1.0e-7f32;
    let inside = (a0 >= -eps && a1 >= -eps && a2 >= -eps) || (a0 <= eps && a1 <= eps && a2 <= eps);
    if !inside {
        return None;
    }

    let b0 = a0 / area;
    let b1 = a1 / area;
    let b2 = a2 / area;

    // Perspective-correct weights from screen-space barycentric coordinates.
    let inv_w = b0 / ndcw0.w + b1 / ndcw1.w + b2 / ndcw2.w;
    if inv_w.abs() <= f32::EPSILON {
        return None;
    }
    let w = 1.0 / inv_w;
    let bc0 = b0 * w / ndcw0.w;
    let bc1 = b1 * w / ndcw1.w;
    let bc2 = b2 * w / ndcw2.w;

    let z_clip = bc0 * ndcw0.z + bc1 * ndcw1.z + bc2 * ndcw2.z;
    let w_clip = bc0 * ndcw0.w + bc1 * ndcw1.w + bc2 * ndcw2.w;
    if w_clip.abs() <= f32::EPSILON {
        return None;
    }
    Some((z_clip / w_clip, [bc0, bc1, bc2]))
}

/// World-to-NDC camera transform used by this exercise.
///
/// This combines perspective projection with a simple rigid camera pose.
pub fn camera_transformation() -> Matrix4<f32> {
    let near: f32 = 0.5;
    let far: f32 = 4.0;
    let frustrum_near_size: f32 = 0.30;

    let transform = Matrix4::from_row_slice(&[
        near / frustrum_near_size,
        0.0,
        0.0,
        0.0,
        0.0,
        near / frustrum_near_size,
        0.0,
        0.0,
        0.0,
        0.0,
        (far + near) / (far - near),
        2.0 * far * near / (far - near),
        0.0,
        0.0,
        -1.0,
        0.0,
    ]);

    let transl = Matrix4::from_row_slice(&[
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, -2.0, 0.0, 0.0, 0.0, 1.0,
    ]);
    let theta = 135.0f32.to_radians();
    let rot_y = Matrix4::from_row_slice(&[
        theta.cos(),
        0.0,
        theta.sin(),
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        -theta.sin(),
        0.0,
        theta.cos(),
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ]);
    let world_to_camera = transl * rot_y;
    transform * world_to_camera
}

/// Returns flattened tile indices whose AABB overlaps the input triangle.
///
/// Triangle input is in NDCW and projected to screen space internally.
fn list_of_tiles_overlapping_input_tri(
    ndcw0: Vector4<f32>,
    ndcw1: Vector4<f32>,
    ndcw2: Vector4<f32>,
    img_shape: (usize, usize),
    tile_size: usize,
) -> Vec<usize> {
    if tile_size == 0 {
        return Vec::new();
    }

    if ndcw0.w.abs() <= f32::EPSILON
        || ndcw1.w.abs() <= f32::EPSILON
        || ndcw2.w.abs() <= f32::EPSILON
    {
        return Vec::new();
    }

    let ndc = [
        (ndcw0.x / ndcw0.w, ndcw0.y / ndcw0.w),
        (ndcw1.x / ndcw1.w, ndcw1.y / ndcw1.w),
        (ndcw2.x / ndcw2.w, ndcw2.y / ndcw2.w),
    ];

    let w = img_shape.0 as f32;
    let h = img_shape.1 as f32;
    let mut xs = [0.0f32; 3];
    let mut ys = [0.0f32; 3];
    for i in 0..3 {
        xs[i] = (ndc[i].0 * 0.5 + 0.5) * w;
        ys[i] = (1.0 - (ndc[i].1 * 0.5 + 0.5)) * h;
    }

    let min_x = xs.iter().fold(f32::INFINITY, |a, &b| a.min(b)).floor() as isize;
    let max_x = xs.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b)).ceil() as isize;
    let min_y = ys.iter().fold(f32::INFINITY, |a, &b| a.min(b)).floor() as isize;
    let max_y = ys.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b)).ceil() as isize;

    let w_i = img_shape.0 as isize;
    let h_i = img_shape.1 as isize;
    let x0 = min_x.clamp(0, w_i.saturating_sub(1));
    let x1 = max_x.clamp(0, w_i.saturating_sub(1));
    let y0 = min_y.clamp(0, h_i.saturating_sub(1));
    let y1 = max_y.clamp(0, h_i.saturating_sub(1));
    if x0 > x1 || y0 > y1 {
        return Vec::new();
    }

    let n_tiles_x = img_shape.0.div_ceil(tile_size);
    let tx0 = (x0 as usize) / tile_size;
    let tx1 = (x1 as usize) / tile_size;
    let ty0 = (y0 as usize) / tile_size;
    let ty1 = (y1 as usize) / tile_size;

    let mut tiles = Vec::with_capacity((tx1 - tx0 + 1) * (ty1 - ty0 + 1));
    for ty in ty0..=ty1 {
        for tx in tx0..=tx1 {
            tiles.push(ty * n_tiles_x + tx);
        }
    }
    tiles
}

/// Builds compressed tile bins.
///
/// Output is (tile2idx_offset, idx2tri), where each tile stores a contiguous
/// slice into idx2tri.
fn build_tile2idx_idx2tri(
    vtx2ndcw: &[Vector4<f32>],
    tri2vtx: &[usize],
    img_shape: (usize, usize),
    tile_size: usize,
) -> (Vec<usize>, Vec<usize>) {
    assert!(tile_size > 0);
    assert_eq!(tri2vtx.len() % 3, 0);

    let n_tiles_x = img_shape.0.div_ceil(tile_size);
    let n_tiles_y = img_shape.1.div_ceil(tile_size);
    let n_tiles = n_tiles_x * n_tiles_y;
    let n_tri = tri2vtx.len() / 3;

    // comment out from here
    // this is for brute force computation
    let tile2idx_offset = (0..n_tiles+1).map(|i_tile| i_tile * n_tri).collect();
    let idx2tri = (0..n_tiles).flat_map(|_|(0..n_tri).to_owned() ).collect();
    // end of comment out

    // ----------------------
    // implement code here to make a map from tile to triangle (follow the "Jagged Array" slide)
    // use the "list_of_tiles_overlapping_input_tri" function above

    (tile2idx_offset, idx2tri)
}
