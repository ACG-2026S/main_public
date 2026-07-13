import pathlib
#
import numpy as np
from numpy.typing import NDArray


def load_triangle_mesh_from_wavefront_obj_file(file_path: str):
    try:
        vtx2xyz = []
        tri2vtx = []
        with open(file_path) as f:
            for line in f:
                if line[0] == "v":
                    xyz = list(map(float, line[2:].strip().split()))
                    vtx2xyz.append(xyz)
                elif line[0] == "f":
                    vtx = list(map(int, line[2:].strip().split()))
                    tri2vtx.append(vtx)
        tri2vtx = np.array(tri2vtx, dtype=np.uint32) - 1
        vtx2xyz = np.array(vtx2xyz, dtype=np.float32)
        return (tri2vtx, vtx2xyz)

    except FileNotFoundError:
        print(f"{file_path} not found.")
    except:
        print("An error occurred while loading the shape.")


def save_obj(
    path: str | pathlib.Path,
    triangles: NDArray[np.integer],
    vertices: NDArray[np.floating],
) -> None:
    """
    Save vertex coordinates and triangle indices as a Wavefront OBJ file.

    Parameters
    ----------
    path
        Output OBJ file path.
    vertices
        Vertex coordinate array with shape (num_vertices, 3).
    triangles
        Triangle index array with shape (num_triangles, 3).
        Indices must be zero-based, following the NumPy convention.
    """
    path = pathlib.Path(path)
    vertices = np.asarray(vertices)
    triangles = np.asarray(triangles)

    if vertices.ndim != 2 or vertices.shape[1] != 3:
        raise ValueError(
            f"vertices must have shape (N, 3), but got {vertices.shape}"
        )

    if triangles.ndim != 2 or triangles.shape[1] != 3:
        raise ValueError(
            f"triangles must have shape (M, 3), but got {triangles.shape}"
        )

    if not np.issubdtype(vertices.dtype, np.number):
        raise TypeError("vertices must contain numeric values")

    if not np.issubdtype(triangles.dtype, np.integer):
        raise TypeError("triangles must contain integer indices")

    if not np.all(np.isfinite(vertices)):
        raise ValueError("vertices contain NaN or infinity")

    if triangles.size > 0:
        if triangles.min() < 0:
            raise ValueError("triangles contain negative indices")

        if triangles.max() >= len(vertices):
            raise ValueError(
                "triangles contain an index larger than the vertex count"
            )

    path.parent.mkdir(parents=True, exist_ok=True)

    with path.open("w", encoding="utf-8") as file:
        for x, y, z in vertices:
            file.write(f"v {x:.17g} {y:.17g} {z:.17g}\n")

        # OBJ indices are one-based.
        for i0, i1, i2 in triangles:
            file.write(f"f {i0 + 1} {i1 + 1} {i2 + 1}\n")

def normalize_vtx2xyz(vtx2xyz):
    """
    fit the points inside unit cube [0,1]^3
    :param tri2center:
    :return:
    """
    vmin = np.min(vtx2xyz, axis=0)
    vmax = np.max(vtx2xyz, axis=0)
    vtx2xyz -= (vmin+vmax)*0.5
    vtx2xyz *= 1.0/(vmax - vmin).max()


def vtx2nrm(tri2vtx, vtx2xyz) -> np.typing.NDArray:
    v0 = tri2vtx[:, 0]
    v1 = tri2vtx[:, 1]
    v2 = tri2vtx[:, 2]
    u = vtx2xyz[v1] - vtx2xyz[v0]
    v = vtx2xyz[v2] - vtx2xyz[v0]
    c = np.cross(u, v)
    c = c / np.linalg.norm(c, axis=1)[:, np.newaxis]
    vtx2nrm = np.zeros_like(vtx2xyz)
    vtx2nrm[v0] += c
    vtx2nrm[v1] += c
    vtx2nrm[v2] += c
    vtx2nrm = vtx2nrm / np.linalg.norm(vtx2nrm, axis=1)[:, np.newaxis]
    return vtx2nrm


def octahedron() -> (np.typing.NDArray, np.typing.NDArray):
    tri2vtx = [
        [0, 1, 2],
        [0, 2, 3],
        [0, 3, 4],
        [0, 4, 1],
        [5, 4, 3],
        [5, 3, 2],
        [5, 2, 1],
        [5, 1, 4]]
    vtx2xyz = [
        [0., 0., -1.],
        [1., 0., 0.],
        [0., 1., 0.],
        [-1., 0., 0.],
        [0., -1., 0.],
        [0., 0., 1.]]
    tri2vtx = numpy.array(tri2vtx, dtype=numpy.uint32)
    vtx2xyz = numpy.array(vtx2xyz, dtype=numpy.float32)

    return (tri2vtx, vtx2xyz)
