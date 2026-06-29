use nalgebra::zero;

type Mat4f = nalgebra::Matrix4<f32>;
type Vec3f = nalgebra::Vector3<f32>;
type Pnt3f = nalgebra::Point3<f32>;
type Vec4f = nalgebra::Vector4<f32>;

#[derive(PartialEq, Clone, Copy)]
enum ObjectIdentity {
    Floor,
    Sphere(usize),
}

#[derive(Clone, Copy)]
enum MaterialType {
    Emission(Vec3f),
    Diffuse(Vec3f),
    Specular(Vec3f),
}


struct Floor {
    normal: Vec3f,
    point: Pnt3f,
}

struct Sphere {
    center: Pnt3f,
    radius: f32,
    material: MaterialType,
}

fn main() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let image_shape: (usize, usize) = (512, 512); // w, h

    let xyz2ndc: Mat4f = camera_transformation();
    let ndc2xyz: Mat4f = xyz2ndc.try_inverse().unwrap();

    let floor = Floor {
        normal: Vec3f::new(0.0, 1.0, 0.0),
        point: Pnt3f::new(0.0, -1.0, 0.0),
    };
    let spheres = vec![
        Sphere {
            center: Pnt3f::new(0.0, 0.8, -3.0),
            radius: 0.7,
            material: MaterialType::Emission(Vec3f::new(1.0, 0.7, 0.7)),
        },
        Sphere {
            center: Pnt3f::new(1.4, -0.3, -3.0),
            radius: 0.7,
            material: MaterialType::Diffuse(Vec3f::new(0.5, 1.0, 1.0)),
        },
        Sphere {
            center: Pnt3f::new(-1.4, -0.3, -3.0),
            radius: 0.7,
            material: MaterialType::Diffuse(Vec3f::new(0.0, 1.0, 0.5)),
        },
        Sphere {
            center: Pnt3f::new(0.0, -0.6, -2.0),
            radius: 0.4,
            material: MaterialType::Specular(Vec3f::new(0.8, 0.5, 0.5)),
        },
    ];
    let sky_radiance = Vec3f::new(0.5, 0.5, 0.5);

    {
        let mut pix2rgb = vec![[0f32; 3]; image_shape.0 * image_shape.1];
        for i_pix in 0..image_shape.0 * image_shape.1 {
            let i_w = i_pix % image_shape.0;
            let i_h = i_pix / image_shape.0;
            let ray = ray_from_xyz2world(&ndc2xyz, i_w as f32 + 0.5, i_h as f32 + 0.5, image_shape);
            pix2rgb[i_pix] =
                radiance_at_hit_point_ver1(&ray, &floor, &spheres, &sky_radiance).into();
        }
        let path = std::path::Path::new(manifest_dir).join("output1.png");
        save_float_rgba_array_as_img(&path, &pix2rgb, image_shape);
    }
    {
        let mut pix2rgb = vec![[0f32; 3]; image_shape.0 * image_shape.1];
        for i_pix in 0..image_shape.0 * image_shape.1 {
            let i_w = i_pix % image_shape.0;
            let i_h = i_pix / image_shape.0;
            let ray = ray_from_xyz2world(&ndc2xyz, i_w as f32 + 0.5, i_h as f32 + 0.5, image_shape);
            pix2rgb[i_pix] =
                radiance_at_hit_point_ver2(&ray, &floor, &spheres, &sky_radiance).into();
        }
        let path = std::path::Path::new(manifest_dir).join("output2.png");
        save_float_rgba_array_as_img(&path, &pix2rgb, image_shape);
    }
    let num_sample = 20usize;
    use rand::Rng;
    use rand::SeedableRng;
    let mut reng = rand_chacha::ChaChaRng::seed_from_u64(0);
    {
        let timer = std::time::Instant::now();
        let mut pix2rgb = vec![[0f32; 3]; image_shape.0 * image_shape.1];
        for i_pix in 0..image_shape.0 * image_shape.1 {
            let mut rad = Vec3f::zeros();
            for _i_sample in 0..num_sample {
                let w = (i_pix % image_shape.0) as f32 + reng.random::<f32>();
                let h = (i_pix / image_shape.0) as f32 + reng.random::<f32>();
                let ray = ray_from_xyz2world(&ndc2xyz, w, h, image_shape);
                rad += radiance_at_hit_point_ver3(&ray, &floor, &spheres, &sky_radiance, &mut reng);
            }
            pix2rgb[i_pix] = (rad / num_sample as f32).into();
        }
        println!(
            "render time (w/o Russian Roulette): {:.4}s",
            timer.elapsed().as_secs_f32()
        );
        let path = std::path::Path::new(manifest_dir).join("output3.png");
        save_float_rgba_array_as_img(&path, &pix2rgb, image_shape);
    }
    {
        let timer = std::time::Instant::now();
        let mut pix2rgb = vec![[0f32; 3]; image_shape.0 * image_shape.1];
        for i_pix in 0..image_shape.0 * image_shape.1 {
            let mut rad = Vec3f::zeros();
            for _i_sample in 0..num_sample {
                let w = (i_pix % image_shape.0) as f32 + reng.random::<f32>();
                let h = (i_pix / image_shape.0) as f32 + reng.random::<f32>();
                let ray = ray_from_xyz2world(&ndc2xyz, w, h, image_shape);
                let mut throughput = Vec3f::new(1., 1., 1.);
                let radiance= throughput_and_light_radiance_at_hit_point(&mut throughput, 0, &ray, &floor, &spheres, &sky_radiance, &mut reng);
                rad += throughput.component_mul(&radiance);
            }
            pix2rgb[i_pix] = (rad / num_sample as f32).into();
        }
        println!(
            "render time (w Russian Roulette): {:.4}s",
            timer.elapsed().as_secs_f32()
        );
        let path = std::path::Path::new(manifest_dir).join("output4.png");
        save_float_rgba_array_as_img(&path, &pix2rgb, image_shape);
    }
}


/// sanity-check where radiance = albedo
fn radiance_at_hit_point_ver1(
    ray: &(Pnt3f, Vec3f),
    floor: &Floor,
    spheres: &[Sphere],
    sky_radiance: &Vec3f,
) -> Vec3f {
    let (obj_type, depth) = ray_scene_intersection(ray, floor, spheres);
    // -------
    if depth == f32::INFINITY {
        return *sky_radiance;
    }
    let hit_pos = ray.0 + depth * ray.1;
    let (material, _hit_normal) = material_and_normal(obj_type, hit_pos, floor, spheres);
    match material {
        MaterialType::Specular(albedo) => albedo,
        MaterialType::Diffuse(albedo) => albedo,
        MaterialType::Emission(radiance) => radiance,
    }
}

/// analytical integration ignoring occlusion and radiance from non-emission surface
fn radiance_at_hit_point_ver2(
    ray: &(Pnt3f, Vec3f),
    floor: &Floor,
    spheres: &[Sphere],
    sky_radiance: &Vec3f,
) -> Vec3f {
    let (obj_type, depth) = ray_scene_intersection(ray, floor, spheres);
    // -------
    if depth == f32::INFINITY {
        return *sky_radiance;
    }
    let hit_pos = ray.0 + depth * ray.1;
    let (material, hit_normal) = material_and_normal(obj_type, hit_pos, floor, spheres);
    //
    match material {
        MaterialType::Diffuse(albedo) => {
            let mut rad_out = Vec3f::zeros();
            // problem 1
            // add contribution from sky here
            for (i_sphere, sphere) in spheres.iter().enumerate() {
                if obj_type == ObjectIdentity::Sphere(i_sphere) {
                    continue;
                }
                match sphere.material {
                    MaterialType::Emission(radiance) => {
                        let t = sphere.radius / (hit_pos - sphere.center).norm();
                        let d = (sphere.center - hit_pos).normalize();
                        rad_out += t * t * d.dot(&hit_normal) * albedo.component_mul(&radiance);
                    }
                    _ => {}
                }
            }
            rad_out
        }
        MaterialType::Specular(albedo) => {
            let ray_material_in = ray.1 - 2. * ray.1.dot(&hit_normal) * hit_normal;
            let rad_material_in = radiance_at_hit_point_ver2(
                &(hit_pos + 0.001 * hit_normal, ray_material_in),
                floor,
                spheres,
                sky_radiance,
            );
            let rad_material_out = rad_material_in.component_mul(&albedo);
            rad_material_out
        }
        MaterialType::Emission(radiance) => radiance,
    }
}


/// cosine-weighted importance sampling
fn radiance_at_hit_point_ver3<R>(
    ray: &(Pnt3f, Vec3f),
    floor: &Floor,
    spheres: &[Sphere],
    sky_radiance: &Vec3f,
    reng: &mut R,
) -> Vec3f
where
    R: rand::Rng,
{
    let (obj_type, depth) = ray_scene_intersection(ray, floor, spheres);
    // -------
    if depth == f32::INFINITY {
        return *sky_radiance;
    }
    let hit_pos = ray.0 + depth * ray.1;
    let (material, hit_normal) = material_and_normal(obj_type, hit_pos, floor, spheres);
    //
    match material {
        MaterialType::Diffuse(albedo) => {
            let ray_material = sample_hemisphere_uniform(hit_normal, (reng.random(), reng.random()));
            // switch to cosine_weighted sampling here
            // let ray_material = sample_hemisphere_cosine_weighted(hit_normal, (reng.random(), reng.random()));
            let rad_material_in = radiance_at_hit_point_ver3(
                &(hit_pos + 0.001 * hit_normal, ray_material),
                floor,
                spheres,
                sky_radiance,
                reng,
            );
            // change code here
            let rad_material_out = rad_material_in.component_mul(&albedo) * 2.0 * ray_material.dot(&hit_normal);
            rad_material_out
        }
        MaterialType::Specular(albedo) => {
            let ray_material = ray.1 - 2. * ray.1.dot(&hit_normal) * hit_normal; // reflection ray direction
            let rad_material_in = radiance_at_hit_point_ver3(
                &(hit_pos + 0.001 * hit_normal, ray_material),
                floor,
                spheres,
                sky_radiance,
                reng,
            );
            let rad_material_out = rad_material_in.component_mul(&albedo);
            rad_material_out
        }
        MaterialType::Emission(radiance) => radiance,
    }
}

/// Russian roulette algorithm
fn throughput_and_light_radiance_at_hit_point<R>(
    throughput: &mut Vec3f,
    rr_depth: usize,
    ray: &(Pnt3f, Vec3f),
    floor: &Floor,   
    spheres: &[Sphere],
    sky_radiance: &Vec3f,
    reng: &mut R,
) -> Vec3f
where
    R: rand::Rng,
{
    if rr_depth >= 1 {
        let continue_prob = throughput.max().clamp(0.05, 0.95);
        // implement Russian roulette here to terminate ray with probability `1-continue_prob`
    }

    let (obj_type, depth) = ray_scene_intersection(ray, floor, spheres);
    // -------
    if depth == f32::INFINITY {
        return *sky_radiance;
    }
    let hit_pos = ray.0 + depth * ray.1;
    let (material, hit_normal) = material_and_normal(obj_type, hit_pos, floor, spheres);
    //
    match material {
        MaterialType::Diffuse(albedo) => {
            let ray_material = sample_hemisphere_uniform(hit_normal, (reng.random(), reng.random()));
            // switch to cosine_weighted sampling here
            //let ray_material = sample_hemisphere_cosine_weighted(hit_normal, (reng.random(), reng.random()));
            let mut throughput0 = throughput.component_mul(&albedo);
            let rad_material_in = throughput_and_light_radiance_at_hit_point(
                &mut throughput0,
                rr_depth + 1,
                &(hit_pos + 0.001 * hit_normal, ray_material),
                floor,
                spheres,
                sky_radiance,
                reng,
            );
            // change code here
            throughput.copy_from(&(throughput0 * 2.0 * ray_material.dot(&hit_normal)));
            rad_material_in
        }
        MaterialType::Specular(albedo) => {
            let ray_material = ray.1 - 2. * ray.1.dot(&hit_normal) * hit_normal; // reflection ray direction
            let mut throughput0 = throughput.component_mul(&albedo);
            let rad_material_in = throughput_and_light_radiance_at_hit_point(
                &mut throughput0,
                rr_depth + 1,
                &(hit_pos + 0.001 * hit_normal, ray_material),
                floor,
                spheres,
                sky_radiance,
                reng,
            );
            throughput.copy_from(&throughput0);
            rad_material_in
        }
        MaterialType::Emission(radiance) => radiance,
    }
}

fn light_radiance_at_hit_point<R>(
    ray: &(Pnt3f, Vec3f),
    floor: &Floor,
    spheres: &[Sphere],
    sky_radiance: &Vec3f,
    reng: &mut R,
) -> Vec3f
where
    R: rand::Rng,
{
    let (obj_type, depth) = ray_scene_intersection(ray, floor, spheres);
    // -------
    if depth == f32::INFINITY {
        return *sky_radiance;
    }
    let hit_pos = ray.0 + depth * ray.1;
    let (material, _hit_normal) = material_and_normal(obj_type, hit_pos, floor, spheres);
    //
    match material {
        MaterialType::Emission(radiance) => radiance,
        _ => { Vec3f::zeros() }
    }
}

// -----------------------------------------

fn sample_hemisphere_uniform(normal: Vec3f, (u0, u1): (f32, f32)) -> Vec3f {
    let local_coordinates = {
        let z = u0; // cos(theta), uniform in [0, 1]
        let r = (1.0 - z * z).sqrt();
        let phi = 2.0 * std::f32::consts::PI * u1;
        let x = r * phi.cos();
        let y = r * phi.sin();
        Vec3f::new(x, y, z)
    };
    let a_vec = if normal.z.abs() < 0.999 {
        Vec3f::new(0., 0., 1.)
    } else {
        Vec3f::new(1., 0., 0.)
    };
    let x_axis = a_vec.cross(&normal).normalize();
    let y_axis = normal.cross(&x_axis);
    local_coordinates.x * x_axis + local_coordinates.y * y_axis + local_coordinates.z * normal
}

fn sample_hemisphere_cosine_weighted(normal: Vec3f, (u0, u1): (f32, f32)) -> Vec3f {
    // comment out below and implement function
    sample_hemisphere_uniform(normal, (u0, u1))
}


fn material_and_normal(
    obj_type: ObjectIdentity,
    hit_pos: Pnt3f,
    floor: &Floor,
    spheres: &[Sphere],
) -> (MaterialType, Vec3f) {
    match obj_type {
        ObjectIdentity::Floor => {
            // Checkerboard based on world-space x/z at the hit point.
            let ix = hit_pos.x.floor() as i32;
            let iy = hit_pos.z.floor() as i32;
            if (ix + iy) & 1 == 0 {
                (
                    MaterialType::Diffuse(Vec3f::new(0.95, 0.95, 0.95)),
                    floor.normal,
                )
            } else {
                (
                    MaterialType::Specular(Vec3f::new(0.7, 0.7, 0.7)),
                    floor.normal,
                )
            }
        }
        ObjectIdentity::Sphere(obj_idx) => {
            let normal = (hit_pos - spheres[obj_idx].center).normalize();
            (spheres[obj_idx].material, normal)
        }
    }
}

fn ray_scene_intersection(
    ray: &(Pnt3f, Vec3f),
    floor: &Floor,
    spheres: &[Sphere],
) -> (ObjectIdentity, f32) {
    let mut res: (ObjectIdentity, f32) = (ObjectIdentity::Floor, f32::INFINITY);
    for (i_sphere, sphere) in spheres.iter().enumerate() {
        let depth = intersect_sphere(sphere, ray.0, ray.1);
        if depth < res.1 {
            res = (ObjectIdentity::Sphere(i_sphere), depth);
        }
    }
    let depth = intersect_floor(&floor, ray.0, ray.1);
    if depth < res.1 {
        res = (ObjectIdentity::Floor, depth);
    }
    assert!(depth > 0.0);
    res
}

fn ray_from_xyz2world(
    ndc2xyz: &Mat4f,
    w: f32,
    h: f32,
    image_shape: (usize, usize),
) -> (Pnt3f, Vec3f) {
    let unproject_ndc = |x: f32, y: f32, z: f32| -> Pnt3f {
        let p_h: Vec4f = ndc2xyz * Pnt3f::new(x, y, z).to_homogeneous();
        Pnt3f::from_homogeneous(p_h).expect("unproject_ndc got w=0 (point at infinity)")
    };
    let ndc_x = 2.0 * w / image_shape.0 as f32 - 1.0;
    let ndc_y = 1.0 - 2.0 * h / image_shape.1 as f32;
    // Unproject the pixel center at the near and far planes to get two points in world space.
    let xyz_near = unproject_ndc(ndc_x, ndc_y, -1.0);
    let xyz_far = unproject_ndc(ndc_x, ndc_y, 1.0);
    // The ray direction is the vector from the near point to the far point, normalized to unit length.
    let ray_dir = (xyz_far - xyz_near).normalize();
    (xyz_near, ray_dir)
}

fn intersect_floor(floor: &Floor, ray_origin: Pnt3f, ray_dir: Vec3f) -> f32 {
    let denom = floor.normal.dot(&ray_dir);
    if denom.abs() < 1.0e-6 {
        return f32::INFINITY;
    }
    let t = floor.normal.dot(&(floor.point - ray_origin)) / denom;
    if t <= 1.0e-4 {
        return f32::INFINITY;
    }
    t
}

fn intersect_sphere(sphere: &Sphere, ray_origin: Pnt3f, ray_dir: Vec3f) -> f32 {
    let oc = ray_origin - sphere.center;
    let a = ray_dir.dot(&ray_dir);
    let b = 2.0 * oc.dot(&ray_dir);
    let c = oc.dot(&oc) - sphere.radius * sphere.radius;

    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return f32::INFINITY;
    }

    let sqrt_disc = disc.sqrt();
    let inv_2a = 0.5 / a;
    let t0 = (-b - sqrt_disc) * inv_2a;
    let t1 = (-b + sqrt_disc) * inv_2a;

    if t0 >= 0.0 {
        t0
    } else if t1 >= 0.0 {
        t1
    } else {
        return f32::INFINITY;
    }
}

fn save_float_rgba_array_as_img(
    output_path: &std::path::Path,
    pix2rgb: &[[f32; 3]],
    image_shape: (usize, usize),
) {
    let pix2rgb: Vec<u8> = pix2rgb
        .iter()
        .flat_map(|rgb| rgb.iter().map(|&c| (c.clamp(0.0, 1.0) * 255.0) as u8))
        .collect();
    image::save_buffer(
        output_path,
        &pix2rgb,
        image_shape.0 as u32,
        image_shape.1 as u32,
        image::ColorType::Rgb8,
    )
    .unwrap_or_else(|e| panic!("Failed to write {}: {e}", output_path.display()));
}

/// World-to-NDC camera transform used by this exercise.
///
/// This combines perspective projection with a simple rigid camera pose.
pub fn camera_transformation() -> Mat4f {
    let near: f32 = 0.5;
    let far: f32 = 4.0;
    let frustrum_near_size: f32 = 0.30;

    let m_perspective = Mat4f::from_row_slice(&[
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
        -(far + near) / (far - near),
        -(2.0 * far * near) / (far - near),
        0.0,
        0.0,
        -1.0,
        0.0,
    ]);

    let m_translation = Mat4f::new_translation(&Vec3f::new(0.0, 0.0, 0.0));

    let m_rot_y = Mat4f::new_rotation(Vec3f::new(0.0, 1.0, 0.0) * 0.0f32.to_radians());

    m_perspective * m_translation * m_rot_y
}
