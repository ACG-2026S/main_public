import os, math, argparse
#
import numpy as np
from scipy import sparse
from scipy.sparse.linalg import spsolve
#
import util
import polyscope as ps


def matrix_graph_laplacian(tri2vtx, num_vtx):
    '''
    function to make graph laplacian matrix
    :param tri2vtx: index of vertex for triangles
    :param num_vtx: number of vertex
    :return: sparse matrix using scipy.sparse
    '''
    tri2ones = np.ones((tri2vtx.shape[0],))
    W = np.empty(0, np.float32)
    I = np.empty(0, np.uint32)
    J = np.empty(0, np.uint32)
    #
    for (i, j) in ((0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2), (2, 0), (2, 1), (2, 2)):
        tri2vi = tri2vtx[:, i]
        tri2vj = tri2vtx[:, j]
        if i == j:
            coeff = 1.
        else:
            coeff = -0.5
        W = np.append(W, tri2ones * coeff)
        I = np.append(I, tri2vi)
        J = np.append(J, tri2vj)
    return sparse.csr_matrix((W, (I, J)), shape=(num_vtx, num_vtx))


def solve_smooth_deformation(vtx2xyz_ini, vtx2xyz_def, matrix_fix, matrix_smooth):
    """Solve one linear system per coordinate for smooth mesh deformation."""

    # write a few line code below to implement laplacian mesh deformation
    # Problem 2: Laplacian deformation, which minimizes (x-x_def)D(x-x_def) + (x-x_ini)L(x-x_ini) w.r.t x,
    # Problem 3: Bi-Laplacian deformation, which minimizes (x-x_def)D(x-x_def) + (x-x_ini)L^2(x-x_ini) w.r.t x,
    # where D is the diagonal matrix with entry 1 if the vertex is fixed, 0 if the vertex is free, a.k.a `self.matrix_fix`
    # L is the graph Laplacian matrix a.k.a `self.matrix_laplace`
    # you may use `spsolve` to solve the liner system
    # spsolve: https://docs.scipy.org/doc/scipy/reference/generated/scipy.sparse.linalg.spsolve.html#scipy.sparse.linalg.spsolve

    pass


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--method", choices=["laplacian", "bilaplacian"], default="laplacian")
    args, _ = parser.parse_known_args()

    asset_path = os.path.normpath(os.path.join(__file__, '../bunny.obj'))
    tri2vtx, vtx2xyz_ini = util.load_triangle_mesh_from_wavefront_obj_file(asset_path)
    util.normalize_vtx2xyz(vtx2xyz_ini)
    vtx2xyz_def = vtx2xyz_ini.copy()

    fixbase2vtx = (vtx2xyz_ini[:, 1] < -0.48).nonzero()[0].tolist()
    fixear2vtx = (vtx2xyz_ini[:, 1] > 0.48).nonzero()[0].tolist()
    fixback2vtx = (np.logical_and(vtx2xyz_ini[:, 0] > +0.15, vtx2xyz_ini[:, 1] > 0.115)).nonzero()[0].tolist()

    vtx2xyz_def[fixear2vtx, 0] += -0.5 * math.sin(0.0)
    vtx2xyz_def[fixback2vtx, 1] += -0.3 * math.sin(0.0)

    num_vtx = vtx2xyz_ini.shape[0]
    matrix_laplace = matrix_graph_laplacian(tri2vtx, num_vtx)

    coeff_penalty = 100.0
    fix2vtx = np.concatenate([fixback2vtx, fixear2vtx, fixbase2vtx])
    matrix_fix = sparse.csr_matrix(
        (np.ones(fix2vtx.shape[0], np.float32) * coeff_penalty, (fix2vtx, fix2vtx)),
        shape=(num_vtx, num_vtx)
    )

    if args.method == "bilaplacian":
        # problem 3
        # matrix smooth =
        pass
    else:
        matrix_smooth = matrix_laplace

    ps.init()
    ps.register_surface_mesh("mesh_initial", vtx2xyz_ini, tri2vtx, enabled=False)
    mesh_def = ps.register_surface_mesh("mesh_deformed", vtx2xyz_def, tri2vtx)

    pc_base = ps.register_point_cloud("fixed_base", vtx2xyz_def[fixbase2vtx], radius=0.003, color=(0.0, 0.0, 1.0))
    pc_ear = ps.register_point_cloud("fixed_ear", vtx2xyz_def[fixear2vtx], radius=0.003, color=(0.0, 1.0, 0.0))
    pc_back = ps.register_point_cloud("fixed_back", vtx2xyz_def[fixback2vtx], radius=0.003, color=(1.0, 0.0, 0.0))

    t_now = 0.0

    def update_scene():
        vtx2xyz_def[:] = vtx2xyz_ini[:]
        vtx2xyz_def[fixear2vtx, 0] += -0.5 * math.sin(t_now)
        vtx2xyz_def[fixback2vtx, 1] += -0.3 * math.sin(2.0 * t_now)
        solve_smooth_deformation(vtx2xyz_ini, vtx2xyz_def, matrix_fix, matrix_smooth)
        mesh_def.update_vertex_positions(vtx2xyz_def)
        pc_base.update_point_positions(vtx2xyz_def[fixbase2vtx])
        pc_ear.update_point_positions(vtx2xyz_def[fixear2vtx])
        pc_back.update_point_positions(vtx2xyz_def[fixback2vtx])

    update_scene()

    def callback():
        nonlocal t_now
        t_now += 0.1
        update_scene()

    ps.set_user_callback(callback)
    ps.show()

    # save file
    t_now = 4.0
    update_scene()
    util.save_obj("bunny_def.obj", tri2vtx, vtx2xyz_def)

    return


if __name__ == "__main__":
    main()
