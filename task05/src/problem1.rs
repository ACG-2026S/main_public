/// Fast Computation of Ray-Triangle Mesh Intersection using BVH

type Vec3 = nalgebra::Vector3<f32>;

pub fn problem1() {
    const NUM_LEV: usize = 7;
    let (vtx2xyz, tri2vtx) = make_triangle_mesh(NUM_LEV);
    let bvhnodes = build_bvh(&tri2vtx, &vtx2xyz);
    {
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let key: usize = bvhnodes.iter().map(|bvhnode| {
            let i: usize = rng.gen_range(0..=3);
            i * bvhnode.i_node_left
        }).sum();
        println!("key {}", key);
    }

    const IMG_WIDTH: u32 = 128;
    const IMG_HEIGHT: u32 = 128;

    let mut pix2nrm = vec![0.0f32; (IMG_HEIGHT * IMG_WIDTH * 3) as usize];

    let timer_start = std::time::Instant::now();
    for i_pix in 0..IMG_WIDTH * IMG_HEIGHT {
        let iw = i_pix % IMG_WIDTH;
        let ih = i_pix / IMG_HEIGHT;
        let (ray_src, ray_dir) = get_ray_from_camera(IMG_WIDTH, IMG_HEIGHT, iw, ih);
        // let res = find_intersection_bvh(&ray_src, &ray_dir, &tri2vtx, &vtx2xyz, &bvhnodes);
        let res = find_intersection_bruteforce(&ray_src, &ray_dir, &tri2vtx, &vtx2xyz);
        // draw normal map
        let idx = (ih * IMG_WIDTH + iw) as usize;
        match &res {
            None => {
                pix2nrm[idx * 3 + 0] = 0.0;
                pix2nrm[idx * 3 + 1] = 0.0;
                pix2nrm[idx * 3 + 2] = 0.0;
            }
            Some((_pos, tri_index)) => {
                let nrm = triangle_normal(*tri_index, &tri2vtx, &vtx2xyz);
                pix2nrm[idx * 3 + 0] = nrm.x * 0.5 + 0.5;
                pix2nrm[idx * 3 + 1] = nrm.y * 0.5 + 0.5;
                pix2nrm[idx * 3 + 2] = nrm.z * 0.5 + 0.5;
            }
        }
    }
    println!("problem1: rendering time: {:.3}s", timer_start.elapsed().as_secs_f32());

    // Convert the normal buffer into an RGB image and save it to disk.
    let mut img = image::RgbImage::new(IMG_WIDTH, IMG_HEIGHT);
    pix2nrm.chunks_exact(3).enumerate().for_each(|(idx, pix)| {
        let iw = (idx as u32) % IMG_WIDTH;
        let ih = (idx as u32) / IMG_WIDTH;
        let r = (pix[0].clamp(0.0, 1.0) * 255.0) as u8;
        let g = (pix[1].clamp(0.0, 1.0) * 255.0) as u8;
        let b = (pix[2].clamp(0.0, 1.0) * 255.0) as u8;
        img.put_pixel(iw, ih, image::Rgb([r, g, b]));
    });

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let output_path = std::path::Path::new(manifest_dir).join("output2.png");
    img.save("output1.png")
        .expect("failed to save normal map image");
    println!("Saved image: {}", output_path.display());
}


// A BVH node stores either two children or a single triangle leaf, plus its bounds.
// if `i_node_right` is `usize::MAX`, then the `i_node_left` stores the triangle index
#[derive(Clone, Debug)]
struct BvhNode {
    i_node_left: usize,
    i_node_right: usize,
    // aabb of this bounding box
    v_min: Vec3,
    v_max: Vec3,
}

impl Default for BvhNode {
    fn default() -> Self {
        Self {
            i_node_left: 0,
            i_node_right: usize::MAX,
            v_min: Vec3::zeros(),
            v_max: Vec3::zeros(),
        }
    }
}

impl BvhNode {
    fn is_leaf(&self) -> bool {
        self.i_node_right == usize::MAX
    }

    // Fast axis-aligned bounding-box test for a ray.
    fn intersect_bv(&self, ray_org: &Vec3, ray_dir: &Vec3) -> bool {
        let mut tmin = f32::NEG_INFINITY;
        let mut tmax = f32::INFINITY;

        for i in 0..3 {
            if ray_dir[i].abs() > 1.0e-10 {
                let t1 = (self.v_min[i] - ray_org[i]) / ray_dir[i];
                let t2 = (self.v_max[i] - ray_org[i]) / ray_dir[i];

                tmin = tmin.max(t1.min(t2));
                tmax = tmax.min(t1.max(t2));
            } else if ray_org[i] < self.v_min[i] || ray_org[i] > self.v_max[i] {
                return false;
            }
        }

        tmax >= tmin && tmax >= 0.0
    }
}

fn build_bvh(tri2vtx: &[[usize;3]], vtx2xyz: &[Vec3]) -> Vec<BvhNode> {
    let num_tri = tri2vtx.len();
    let mut bvhnodes = vec![BvhNode::default(); 2 * num_tri - 1];
    // implement code here

    // end of implementation
    set_bvh_geometry(0, &mut bvhnodes, tri2vtx, &vtx2xyz);
    bvhnodes
}

// Compute tight bounds for every BVH node from the triangle vertices.
fn set_bvh_geometry(
    i_node: usize,
    bvhnodes: &mut [BvhNode],
    tri2vtx: &[[usize; 3]],
    vtx2xyz: &[Vec3],
) {
    if bvhnodes[i_node].is_leaf() {
        let i_tri = bvhnodes[i_node].i_node_left;
        let [i0, i1, i2] = tri2vtx[i_tri];

        let p0 = vtx2xyz[i0];
        let p1 = vtx2xyz[i1];
        let p2 = vtx2xyz[i2];

        bvhnodes[i_node].v_min = Vec3::new(
            p0.x.min(p1.x).min(p2.x),
            p0.y.min(p1.y).min(p2.y),
            p0.z.min(p1.z).min(p2.z),
        );

        bvhnodes[i_node].v_max = Vec3::new(
            p0.x.max(p1.x).max(p2.x),
            p0.y.max(p1.y).max(p2.y),
            p0.z.max(p1.z).max(p2.z),
        );
    } else {
        let left = bvhnodes[i_node].i_node_left;
        let right = bvhnodes[i_node].i_node_right;

        set_bvh_geometry(left, bvhnodes, tri2vtx, vtx2xyz);
        set_bvh_geometry(right, bvhnodes, tri2vtx, vtx2xyz);

        let min_left = bvhnodes[left].v_min;
        let max_left = bvhnodes[left].v_max;
        let min_right = bvhnodes[right].v_min;
        let max_right = bvhnodes[right].v_max;

        bvhnodes[i_node].v_min = Vec3::new(
            min_left.x.min(min_right.x),
            min_left.y.min(min_right.y),
            min_left.z.min(min_right.z),
        );

        bvhnodes[i_node].v_max = Vec3::new(
            max_left.x.max(max_right.x),
            max_left.y.max(max_right.y),
            max_left.z.max(max_right.z),
        );
    }
}


// Decode a Morton-coded quadrant index into a 2D grid coordinate.
fn int_coord_from_morton(i_quad: u16) -> usize {
    ((i_quad & 0x0001)
        + ((i_quad & 0x0004) >> 1)
        + ((i_quad & 0x0010) >> 2)
        + ((i_quad & 0x0040) >> 3)
        + ((i_quad & 0x0100) >> 4)
        + ((i_quad & 0x0400) >> 5)
        + ((i_quad & 0x1000) >> 6)
        + ((i_quad & 0x4000) >> 7)) as usize
}

// Build a deformed grid mesh, triangle list, and BVH over the triangles.
fn make_triangle_mesh(num_lev: usize) -> (Vec<Vec3>, Vec<[usize; 3]>) {
    let num_div = 2usize.pow(num_lev as u32);

    println!("number of elements on an edge of square mesh: {}", num_div);

    let size = num_div + 1;
    let num_vtx = size * size;

    let mut vtx2xyz = vec![Vec3::zeros(); num_vtx];

    let theta = -3.14f32 / 4.0;

    for i_h in 0..size {
        let y = i_h as f32 / num_div as f32;

        for i_w in 0..size {
            let x = i_w as f32 / num_div as f32;
            let r = ((x - 0.5).powi(2) + (y - 0.5).powi(2)).sqrt();

            let z = 0.5 * (-10.0 * r * r).exp() * (30.0 * r).sin() - 0.5;

            vtx2xyz[i_h * size + i_w] = Vec3::new(
                x,
                theta.cos() * y - theta.sin() * z + 0.4,
                theta.sin() * y + theta.cos() * z,
            );
        }
    }

    let num_quad = num_div * num_div;
    let num_tri = num_quad * 2;

    let mut tri2vtx = vec![[0usize; 3]; num_tri];

    for i_quad in 0..num_quad {
        let i_w = int_coord_from_morton(i_quad as u16);
        let i_h = int_coord_from_morton((i_quad >> 1) as u16);

        let i0_tri = (i_h * num_div + i_w) * 2;
        tri2vtx[i0_tri] = [
            i_h * size + i_w,
            i_h * size + (i_w + 1),
            (i_h + 1) * size + (i_w + 1),
        ];

        let i1_tri = i0_tri + 1;
        tri2vtx[i1_tri] = [
            i_h * size + i_w,
            (i_h + 1) * size + (i_w + 1),
            (i_h + 1) * size + i_w,
        ];
    }
    (vtx2xyz, tri2vtx)
}

// Moller-Trumbore triangle intersection; returns the hit distance if there is one.
fn intersect_triangle(
    ray_org: &Vec3,
    ray_dir: &Vec3,
    p0: &Vec3,
    p1: &Vec3,
    p2: &Vec3,
) -> Option<f32> {
    let eps = 1.0e-7f32;
    let e1 = p1 - p0;
    let e2 = p2 - p0;
    let pvec = ray_dir.cross(&e2);
    let det = e1.dot(&pvec);

    if det.abs() < eps {
        return None;
    }

    let inv_det = 1.0 / det;
    let tvec = ray_org - p0;
    let u = tvec.dot(&pvec) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let qvec = tvec.cross(&e1);
    let v = ray_dir.dot(&qvec) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = e2.dot(&qvec) * inv_det;
    if t > eps {
        Some(t)
    } else {
        None
    }
}

// Generate a camera ray for one image pixel in normalized device coordinates.
fn get_ray_from_camera(
    width: u32,
    height: u32,
    iw: u32,
    ih: u32,
) -> (nalgebra::Vector3<f32>, nalgebra::Vector3<f32>) {
    let cam_ray_src = Vec3::new(0.5f32, 0.5, 2.0);
    let ndc_x = (iw as f32 + 0.5) * 2.0 / width as f32 - 1.0;
    let ndc_y = 1.0 - (ih as f32 + 0.5) * 2.0 / height as f32;
    let sensor_size = 0.25f32;
    let on_sensor = Vec3::new(ndc_x * sensor_size, ndc_y * sensor_size, -1.0) + cam_ray_src;
    let cam_ray_dir = (on_sensor - cam_ray_src).normalize();
    (cam_ray_src, cam_ray_dir)
}

fn find_intersection_bruteforce(
    cam_ray_src: &Vec3,
    cam_ray_dir: &Vec3,
    tri2vtx: &[[usize; 3]],
    vtx2xyz: &[Vec3]
) -> Option<(Vec3, usize)> {

    let mut nearest_t = f32::INFINITY;
    let mut nearest_tri = 0usize;
    let mut has_hit = false;
    for (i_tri, tri) in tri2vtx.iter().enumerate() {
        let [i0, i1, i2] = *tri;
        let p0 = vtx2xyz[i0];
        let p1 = vtx2xyz[i1];
        let p2 = vtx2xyz[i2];
        if let Some(t) = intersect_triangle(cam_ray_src, cam_ray_dir, &p0, &p1, &p2) {
            if t < nearest_t {
                nearest_t = t;
                nearest_tri = i_tri;
                has_hit = true;
            }
        }
    }

    if has_hit {
        let hit_pos = cam_ray_src + cam_ray_dir * nearest_t;
        Some((hit_pos, nearest_tri))
    } else {
        None
    }
}

// Traverse the BVH and return the closest hit position together with triangle index.
fn find_intersection_bvh(
    cam_ray_src: &Vec3,
    cam_ray_dir: &Vec3,
    tri2vtx: &[[usize; 3]],
    vtx2xyz: &[Vec3],
    bvhnodes: &[BvhNode],
) -> Option<(Vec3, usize)> {
    if bvhnodes.is_empty() {
        return None;
    }

    let mut nearest_t = f32::INFINITY;
    let mut nearest_tri = 0usize;
    let mut has_hit = false;

    let mut stack = vec![0usize];
    while let Some(i_node) = stack.pop() {
        let node = &bvhnodes[i_node];

        if !node.intersect_bv(cam_ray_src, cam_ray_dir) {
            continue;
        }

        if node.is_leaf() {
            let i_tri = node.i_node_left;
            let [i0, i1, i2] = tri2vtx[i_tri];
            let p0 = vtx2xyz[i0];
            let p1 = vtx2xyz[i1];
            let p2 = vtx2xyz[i2];

            if let Some(t) = intersect_triangle(cam_ray_src, cam_ray_dir, &p0, &p1, &p2) {
                if t < nearest_t {
                    nearest_t = t;
                    nearest_tri = i_tri;
                    has_hit = true;
                }
            }
        } else {
            stack.push(node.i_node_left);
            stack.push(node.i_node_right);
        }
    }

    if has_hit {
        let hit_pos = cam_ray_src + cam_ray_dir * nearest_t;
        Some((hit_pos, nearest_tri))
    } else {
        None
    }
}

// Recompute the geometric normal for a triangle from its vertex positions.
fn triangle_normal(tri_index: usize, tri2vtx: &[[usize; 3]], vtx2xyz: &[Vec3]) -> Vec3 {
    let [i0, i1, i2] = tri2vtx[tri_index];
    let p0 = vtx2xyz[i0];
    let p1 = vtx2xyz[i1];
    let p2 = vtx2xyz[i2];
    (p1 - p0).cross(&(p2 - p0)).normalize()
}

