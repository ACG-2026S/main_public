//! methods for Ply file formats

use crate::GaussSplat3D;
use nalgebra::{Matrix3, Point3, Quaternion, UnitQuaternion, Vector3};
use std::io::Read;
enum Format {
    Ascii,
    BinaryLittleEndian,
    BinaryBigEndian,
}

/* --------------------------------------*/

fn parse_f32<const N: usize>(buff: &[u8], i_buff: usize) -> anyhow::Result<[f32; N]> {
    let mut a = [0f32; N];
    for i in 0..N {
        a[i] = f32::from_le_bytes(buff[i_buff + 4 * i..i_buff + 4 * i + 4].try_into()?);
    }
    Ok(a)
}

pub fn read_ply<Path: AsRef<std::path::Path>>(path: Path) -> anyhow::Result<Vec<GaussSplat3D>> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    use std::io::BufRead;
    let mut line = String::new();
    let _hoge = reader.read_line(&mut line)?;
    assert_eq!(line, "ply\n");
    line.clear();
    {
        // reading format
        let _hoge = reader.read_line(&mut line)?;
        let strs: Vec<_> = line.split_whitespace().collect();
        assert_eq!(strs[0], "format");
        let _format = match strs[1] {
            "binary_little_endian" => Format::BinaryLittleEndian,
            "binary_big_endian" => Format::BinaryBigEndian,
            "ascii" => Format::Ascii,
            &_ => panic!(),
        };
        line.clear();
    }
    // skip optional comment lines
    loop {
        let _ = reader.read_line(&mut line)?;
        let strs: Vec<_> = line.split_whitespace().collect();
        if strs[0] != "comment" {
            break;
        }
        line.clear();
    }
    // line already contains the first non-comment line (element vertex)
    let num_elem = {
        let strs: Vec<_> = line.split_whitespace().collect();
        assert_eq!(strs[0], "element");
        assert_eq!(strs[1], "vertex");
        use std::str::FromStr;
        let num_elem = usize::from_str(strs[2]).unwrap();
        line.clear();
        num_elem
    };
    // layout: x y z scale(3) f_dc(3) opacity rot(4) f_rest(45) = 59 floats
    for _i in 0..59 {
        let _ = reader.read_line(&mut line)?;
        let strs: Vec<_> = line.split_whitespace().collect();
        assert_eq!(strs[0], "property");
        assert_eq!(strs[1], "float");
        line.clear();
    }
    {
        // end header
        let _ = reader.read_line(&mut line)?;
        assert_eq!(line, "end_header\n");
    }
    let sh_c0 = 0.28209479177387814;
    let mut buf: Vec<u8> = Vec::new();
    reader.read_to_end(&mut buf)?;
    assert_eq!(buf.len(), 59 * num_elem * 4);
    let mut splats: Vec<GaussSplat3D> = Vec::with_capacity(num_elem);
    for i_elem in 0..num_elem {
        let base = i_elem * 59 * 4;
        let xyz = parse_f32::<3>(&buf, base)?;
        let scale = parse_f32::<3>(&buf, base + 3 * 4)?;
        let rgb = parse_f32::<3>(&buf, base + 6 * 4)?;
        let op = parse_f32::<1>(&buf, base + 9 * 4)?;
        let quaternion = parse_f32::<4>(&buf, base + 10 * 4)?; // w, x, y, z
        let sh = parse_f32::<45>(&buf, base + 14 * 4)?;
        let rgb_dc = Vector3::new(
            (rgb[0] + 0.5) * sh_c0,
            (rgb[1] + 0.5) * sh_c0,
            (rgb[2] + 0.5) * sh_c0,
        );
        let scale = Vector3::new(scale[0].exp(), scale[1].exp(), scale[2].exp());
        let opacity = 1f32 / (1f32 + (-op[0]).exp());
        //
        let quaternion =
            Quaternion::new(quaternion[0], quaternion[1], quaternion[2], quaternion[3]);
        // If q may not be unit-length:
        let quaternion = UnitQuaternion::new_normalize(quaternion);
        let m_rot = quaternion.to_rotation_matrix().into_inner(); // Rotation3<f32>

        let scale = Vector3::<f32>::new(scale[0], scale[1], scale[2]);
        let scale = Matrix3::<f32>::from_diagonal(&scale);

        //let covariance = m_rot.transpose() * scale * scale * m_rot;
        let covariance = m_rot * scale * scale * m_rot.transpose();
        //
        splats.push(GaussSplat3D {
            xyz: Point3::new(xyz[0], xyz[1], xyz[2]),
            rgb: rgb_dc,
            rgb_sh: sh,
            opacity,
            covariance,
        });
    }
    Ok(splats)
}
