mod ply_parser;

type Mat2f = nalgebra::Matrix2<f32>;
type Mat3f = nalgebra::Matrix3<f32>;
type Mat3x4f = nalgebra::Matrix3x4<f32>;
type Mat4f = nalgebra::Matrix4<f32>;
type Vec2f = nalgebra::Vector2<f32>;
type Vec3f = nalgebra::Vector3<f32>;
type Pnt2f = nalgebra::Point2<f32>;
type Pnt3f = nalgebra::Point3<f32>;

/// 3D Gaussian primitive describing 3D scene
pub struct GaussSplat3D {
    pub xyz: Pnt3f,        // 3D coordinates of the center
    pub rgb: Vec3f,        // view independent component of color
    pub rgb_sh: [f32; 45], // view dependent component of color (not used here)
    pub opacity: f32,
    pub covariance: Mat3f, // covariance of this primitive
}

/// projected 2D Gaussian primitive (depends on camera projection matrix)
pub struct GaussSplat2D {
    pub xy: Pnt2f,  // pixel coordinate of the center
    pub ndcz: f32,  // z coordinate in NDC of the center
    pub rgb: Vec3f, // color of this primitive
    pub opacity: f32,
    pub inverse_covariance_2d: Mat2f, // inverse covariance of this primitive
}

fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path_ply = std::path::Path::new(manifest_dir).join("scene.ply");
    let splat3s = ply_parser::read_ply(&path_ply)
        .unwrap_or_else(|e| panic!("Failed to load ply {:?}: {e}", path_ply.as_os_str()));
    let (aabb_min, aabb_max) = aabb(&splat3s).unwrap(); // axis aligned bounding box of scene
    let xyzw2ndcxyzw = transform_world_to_ndc(aabb_min, aabb_max); // 4x4 homography transformation matrix (world -> NDC)
    let image_shape: (usize, usize) = (300, 300);
    let ndcxyzw2pixxyw = transform_ndc_to_image(image_shape); // 3x4 homography transformation matrix (ndc -> pixel coordinate)
    let xyzw2pixxyw = ndcxyzw2pixxyw * xyzw2ndcxyzw; // 3x4 homography transformation matrix (world -> pixel coordinate)

    {
        // point splatting (render and save image)
        let pix2rgba = render_by_point_splatting(&splat3s, image_shape, &xyzw2pixxyw);
        let path = std::path::Path::new(manifest_dir).join("output1.png");
        save_float_rgba_array_as_img(&path, &pix2rgba, image_shape);
    }

    // project 3D Gaussian primitives into 2D Gaussian primitive
    let splat2s: Vec<GaussSplat2D> = project_3d_splats_to_2d(&splat3s, &xyzw2ndcxyzw, &xyzw2pixxyw);

    // depth order (back to front) of splat2.ndcz by sorting
    let idx2splat2 = {
        let mut idx2splat2: Vec<usize> = (0..splat2s.len()).collect();
        idx2splat2.sort_by(|&i, &j| splat2s[i].ndcz.total_cmp(&splat2s[j].ndcz));
        idx2splat2
    };
    const SPLAT_SIZE: i32 = 24; // constant for simplicity
    {
        // Gaussian splatting (back-to-front) (render and save image)
        let timer = std::time::Instant::now();
        let pix2rgba = render_by_gaussian_splatting_back_to_front(
            &splat2s,
            &idx2splat2,
            image_shape,
            SPLAT_SIZE,
        );
        let path = std::path::Path::new(manifest_dir).join("output2.png");
        save_float_rgba_array_as_img(&path, &pix2rgba, image_shape);
        println!(
            "render time (back_to_front): {:.4}s",
            timer.elapsed().as_secs_f32()
        );
    }

    {
        // Gaussian splatting (front-to-back) (render and save image)
        let timer = std::time::Instant::now();
        let pix2rgba = render_by_gaussian_splatting_front_to_bck(
            &splat2s,
            &idx2splat2,
            image_shape,
            SPLAT_SIZE,
        );
        let path = std::path::Path::new(manifest_dir).join("output3.png");
        save_float_rgba_array_as_img(&path, &pix2rgba, image_shape);
        println!(
            "render time (front_to_back): {:.4}s",
            timer.elapsed().as_secs_f32()
        );
    }
}

/// make image by coloring corresponding pixel for each gaussian primitive (not sorted)
/// such image is convenient for debugging camera
fn render_by_point_splatting(
    splat3s: &[GaussSplat3D],
    image_shape: (usize, usize),
    xyzw2pixxyw: &Mat3x4f,
) -> Vec<[f32; 4]> {
    let mut pix2rgba = vec![[0f32; 4]; image_shape.0 * image_shape.1];
    for splat3 in splat3s {
        let pixxyw = xyzw2pixxyw * splat3.xyz.to_homogeneous(); // adding 1 at the end
        let pixxy = Vec2f::new(pixxyw.x / pixxyw.z, pixxyw.y / pixxyw.z);
        if !(0.0..image_shape.0 as f32).contains(&pixxy.x) {
            continue;
        }
        if !(0.0..image_shape.1 as f32).contains(&pixxy.y) {
            continue;
        }
        let ix = pixxy.x as usize;
        let iy = pixxy.y as usize;
        pix2rgba[iy * image_shape.0 + ix][0] = splat3.rgb[0];
        pix2rgba[iy * image_shape.0 + ix][1] = splat3.rgb[1];
        pix2rgba[iy * image_shape.0 + ix][2] = splat3.rgb[2];
        pix2rgba[iy * image_shape.0 + ix][3] = 1.0;
    }
    pix2rgba
}

/// make image by alpha-blending Gaussian primitives from back to front
fn render_by_gaussian_splatting_back_to_front(
    splat2s: &[GaussSplat2D],
    idx2splat2: &[usize],
    image_shape: (usize, usize),
    splat_size: i32,
) -> Vec<[f32; 4]> {
    assert_eq!(splat2s.len(), idx2splat2.len());
    let mut pix2rgba = vec![[0f32; 4]; image_shape.0 * image_shape.1];
    for &idx in idx2splat2 {
        // back-to-front
        let s2 = &splat2s[idx];
        let ix = s2.xy.x as i32;
        let iy = s2.xy.y as i32;
        for jy in iy - splat_size..iy + splat_size {
            if !(0..image_shape.1 as i32).contains(&jy) {
                continue;
            }
            for jx in ix - splat_size..ix + splat_size {
                if !(0..image_shape.0 as i32).contains(&jx) {
                    continue;
                }
                let i_pix = (jy as usize) * image_shape.0 + (jx as usize);
                let d = Vec2f::new(jx as f32 + 0.5 - s2.xy.x, jy as f32 + 0.5 - s2.xy.y);
                let gauss = (-0.5 * d.transpose() * s2.inverse_covariance_2d * d)[(0, 0)].exp();
                let alpha = gauss * s2.opacity;
                if alpha < 0.01 {
                    continue;
                }
                let color_before = &pix2rgba[i_pix][0..3];
                let alpha_before = pix2rgba[i_pix][3];
                pix2rgba[i_pix] = [
                    (1.0 - alpha) * alpha_before * color_before[0] + alpha * s2.rgb.x,
                    (1.0 - alpha) * alpha_before * color_before[1] + alpha * s2.rgb.y,
                    (1.0 - alpha) * alpha_before * color_before[2] + alpha * s2.rgb.z,
                    1.0 - (1.0 - alpha) * (1.0 - alpha_before),
                ];
            }
        }
    }
    pix2rgba
}

/// make image by alpha-blending Gaussian primitives from front to back
fn render_by_gaussian_splatting_front_to_bck(
    splat2s: &[GaussSplat2D],
    idx2splat2: &[usize],
    image_shape: (usize, usize),
    splat_size: i32,
) -> Vec<[f32; 4]> {
    let mut pix2rgba = vec![[0f32; 4]; image_shape.0 * image_shape.1];
    for &idx in idx2splat2.iter().rev() {
        // front_to_back
        let s2 = &splat2s[idx];
        let ix = s2.xy.x as i32;
        let iy = s2.xy.y as i32;
        for jy in iy - splat_size..iy + splat_size {
            if !(0..image_shape.1 as i32).contains(&jy) {
                continue;
            }
            for jx in ix - splat_size..ix + splat_size {
                if !(0..image_shape.0 as i32).contains(&jx) {
                    continue;
                }
                let i_pix = (jy as usize) * image_shape.0 + (jx as usize);
                let d = Vec2f::new(jx as f32 + 0.5 - s2.xy.x, jy as f32 + 0.5 - s2.xy.y);
                let gauss = (-0.5 * d.transpose() * s2.inverse_covariance_2d * d)[(0, 0)].exp();
                let alpha = gauss * s2.opacity;
                if alpha < 0.01 {
                    continue;
                }
                let color_before = &pix2rgba[i_pix][0..3];
                let alpha_before = pix2rgba[i_pix][3];
                pix2rgba[i_pix] = [
                    (1.0 - alpha) * alpha_before * color_before[0] + alpha * s2.rgb.x,
                    (1.0 - alpha) * alpha_before * color_before[1] + alpha * s2.rgb.y,
                    (1.0 - alpha) * alpha_before * color_before[2] + alpha * s2.rgb.z,
                    1.0 - (1.0 - alpha) * (1.0 - alpha_before),
                ];
            }
        }
    }
    pix2rgba
}

fn project_3d_splats_to_2d(
    splat3s: &[GaussSplat3D],
    xyzw2ndcxyzw: &nalgebra::Matrix4<f32>,
    xyzw2pixxyw: &nalgebra::Matrix3x4<f32>,
) -> Vec<GaussSplat2D> {
    splat3s
        .iter()
        .map(|s3| {
            let ndcxyzw = xyzw2ndcxyzw * s3.xyz.to_homogeneous();
            let ndcz = ndcxyzw.z / ndcxyzw.w;
            let pixxyw = xyzw2pixxyw * s3.xyz.to_homogeneous(); // adding 1 at the end
            let pixxy = Vec2f::new(pixxyw.x / pixxyw.z, pixxyw.y / pixxyw.z);
            let proj_jac = projection_jacobian(&xyzw2pixxyw, &s3.xyz);
            let cov2d = proj_jac * s3.covariance * proj_jac.transpose();
            let conv2dinv = (cov2d + Mat2f::new_scaling(1.0e-5)).try_inverse().unwrap();
            GaussSplat2D {
                rgb: s3.rgb,
                opacity: s3.opacity,
                xy: pixxy.into(),
                ndcz,
                inverse_covariance_2d: conv2dinv,
            }
        })
        .collect()
}

fn transform_world_to_ndc(aabb_min: Pnt3f, aabb_max: Pnt3f) -> Mat4f {
    // Compute projection matrix to fit bounding box into [-1, 1]^3 NDC space
    let center = Pnt3f::from((aabb_min.coords + aabb_max.coords) * 0.5);

    // Create translation matrix: translate by -center
    let mut m_translate_aabb = Mat4f::identity();
    m_translate_aabb.column_mut(3)[0] = -center.x;
    m_translate_aabb.column_mut(3)[1] = -center.y;
    m_translate_aabb.column_mut(3)[2] = -center.z;

    // Create scale matrix: scale by 1/max_extent
    let m_scale = {
        let extents = (aabb_max - aabb_min) * 0.5;
        let max_extent = extents.x.max(extents.y).max(extents.z);
        let scale = 1.0 / max_extent;
        let mut scale_matrix = Mat4f::identity();
        scale_matrix[(0, 0)] = scale;
        scale_matrix[(1, 1)] = scale;
        scale_matrix[(2, 2)] = scale;
        scale_matrix
    };

    // Rotate theta radians around the X axis.
    let m_rot_x = {
        let theta = std::f32::consts::PI * -0.81;
        let cos_t = theta.cos();
        let sin_t = theta.sin();
        let mut rotate_x = Mat4f::identity();
        rotate_x[(1, 1)] = cos_t;
        rotate_x[(1, 2)] = -sin_t;
        rotate_x[(2, 1)] = sin_t;
        rotate_x[(2, 2)] = cos_t;
        rotate_x
    };

    // Combine: first translate to AABB center, then rotate, then scale.
    let xyzw2ndcw = m_scale * m_rot_x * m_translate_aabb;
    xyzw2ndcw
}

fn transform_ndc_to_image(image_shape: (usize, usize)) -> nalgebra::Matrix3x4<f32> {
    let width = image_shape.0 as f32;
    let height = image_shape.1 as f32;
    nalgebra::Matrix3x4::<f32>::new(
        0.5 * width,
        0.0,
        0.0,
        0.5 * width,
        0.0,
        -0.5 * height,
        0.0,
        0.5 * height,
        0.0,
        0.0,
        0.0,
        1.0,
    )
}

/// jacobian of projection matrix
/// p = \[(a,b),(c,d)\]
fn projection_jacobian(p: &nalgebra::Matrix3x4<f32>, x: &Pnt3f) -> nalgebra::Matrix2x3<f32> {
    let a = p.fixed_view::<2, 3>(0, 0);
    let b = p.fixed_view::<2, 1>(0, 3);
    let c = p.fixed_view::<1, 3>(2, 0);
    let d = *p.get((2, 3)).unwrap();
    let axb = a * x.coords + b;
    let cxd = (c * x.coords)[(0, 0)] + d;
    let cxdinv = 1.0 / cxd;
    let p_jac = (a - cxdinv * axb * c) * cxdinv;
    p_jac
}

fn save_float_rgba_array_as_img(
    output_path: &std::path::Path,
    pix2rgba: &[[f32; 4]],
    image_shape: (usize, usize),
) {
    let pix2rgb: Vec<u8> = pix2rgba
        .iter()
        .flat_map(|rgb| rgb.iter().map(|&c| (c.clamp(0.0, 1.0) * 255.0) as u8))
        .collect();
    image::save_buffer(
        &output_path,
        &pix2rgb,
        image_shape.0 as u32,
        image_shape.1 as u32,
        image::ColorType::Rgba8,
    )
    .unwrap_or_else(|e| panic!("Failed to write {}: {e}", output_path.display()));
}

pub fn aabb(splats: &[GaussSplat3D]) -> anyhow::Result<(Pnt3f, Pnt3f)> {
    if splats.is_empty() {
        return Err(anyhow::anyhow!(
            "Cannot compute bounding box of empty splat list"
        ));
    }

    let mut min = splats[0].xyz;
    let mut max = splats[0].xyz;

    for splat in &splats[1..] {
        for i in 0..3 {
            if splat.xyz[i] < min[i] {
                min[i] = splat.xyz[i];
            }
            if splat.xyz[i] > max[i] {
                max[i] = splat.xyz[i];
            }
        }
    }

    Ok((
        Pnt3f::new(min[0], min[1], min[2]),
        Pnt3f::new(max[0], max[1], max[2]),
    ))
}
