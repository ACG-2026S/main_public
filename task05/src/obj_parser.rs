use std::fs;

fn parse_obj_index(index_str: &str, count: usize, kind: &str, line_no: usize) -> usize {
    let idx = index_str
        .parse::<isize>()
        .unwrap_or_else(|_| panic!("Line {}: invalid {} index '{}'", line_no, kind, index_str));
    if idx == 0 {
        panic!("Line {}: {} index must not be 0", line_no, kind);
    }

    if idx > 0 {
        let i = (idx - 1) as usize;
        if i >= count {
            panic!("Line {}: {} index {} out of range", line_no, kind, idx);
        }
        i
    } else {
        let i = count as isize + idx;
        if i < 0 {
            panic!("Line {}: {} index {} out of range", line_no, kind, idx);
        }
        i as usize
    }
}

fn parse_face_vertex(token: &str, num_v: usize, num_vt: usize, line_no: usize) -> (usize, usize) {
    let mut parts = token.split('/');
    let v_str = parts
        .next()
        .unwrap_or_else(|| panic!("Line {}: malformed face token '{}'", line_no, token));
    let vt_str = parts
        .next()
        .unwrap_or_else(|| panic!("Line {}: malformed face token '{}'", line_no, token));

    if vt_str.is_empty() {
        panic!(
            "Line {}: face token '{}' is missing texture-coordinate index",
            line_no, token
        );
    }

    let v_idx = parse_obj_index(v_str, num_v, "vertex", line_no);
    let vt_idx = parse_obj_index(vt_str, num_vt, "texture", line_no);
    (v_idx, vt_idx)
}

pub fn parse_obj(path: &std::path::Path) -> (Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<usize>, Vec<usize>) {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()));

    let mut vtx2xyz: Vec<[f32; 3]> = Vec::new();
    let mut vt2uv: Vec<[f32; 2]> = Vec::new();
    let mut tri2vtx: Vec<usize> = Vec::new();
    let mut tri2vt: Vec<usize> = Vec::new();

    for (i_line, raw_line) in content.lines().enumerate() {
        let line_no = i_line + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut fields = line.split_whitespace();
        let Some(tag) = fields.next() else {
            continue;
        };

        match tag {
            "v" => {
                let x = fields
                    .next()
                    .unwrap_or_else(|| panic!("Line {}: malformed vertex", line_no))
                    .parse::<f32>()
                    .unwrap_or_else(|_| panic!("Line {}: invalid vertex x", line_no));
                let y = fields
                    .next()
                    .unwrap_or_else(|| panic!("Line {}: malformed vertex", line_no))
                    .parse::<f32>()
                    .unwrap_or_else(|_| panic!("Line {}: invalid vertex y", line_no));
                let z = fields
                    .next()
                    .unwrap_or_else(|| panic!("Line {}: malformed vertex", line_no))
                    .parse::<f32>()
                    .unwrap_or_else(|_| panic!("Line {}: invalid vertex z", line_no));
                vtx2xyz.push([x, y, z]);
            }
            "vt" => {
                let u = fields
                    .next()
                    .unwrap_or_else(|| panic!("Line {}: malformed texture coordinate", line_no))
                    .parse::<f32>()
                    .unwrap_or_else(|_| panic!("Line {}: invalid texture u", line_no));
                let v = fields
                    .next()
                    .unwrap_or_else(|| panic!("Line {}: malformed texture coordinate", line_no))
                    .parse::<f32>()
                    .unwrap_or_else(|_| panic!("Line {}: invalid texture v", line_no));
                vt2uv.push([u, v]);
            }
            "f" => {
                let tokens: Vec<&str> = fields.collect();
                if tokens.len() < 3 {
                    panic!("Line {}: face has fewer than 3 vertices", line_no);
                }

                let mut parsed: Vec<(usize, usize)> = Vec::with_capacity(tokens.len());
                for token in tokens {
                    parsed.push(parse_face_vertex(
                        token,
                        vtx2xyz.len(),
                        vt2uv.len(),
                        line_no,
                    ));
                }

                for i in 1..(parsed.len() - 1) {
                    tri2vtx.push(parsed[0].0);
                    tri2vtx.push(parsed[i].0);
                    tri2vtx.push(parsed[i + 1].0);

                    tri2vt.push(parsed[0].1);
                    tri2vt.push(parsed[i].1);
                    tri2vt.push(parsed[i + 1].1);
                }
            }
            _ => {
                // Ignore tags not needed for this assignment (vn, mtllib, usemtl, ...)
            }
        }
    }

    (vtx2xyz, vt2uv, tri2vtx, tri2vt)
}
