// Frees what a part of the scene holds on the graphics card: the buffers of
// its geometries, its materials (shader programs once nobody uses them) and
// their textures. three.js keeps them until dispose() is called, so a car
// that is drawn again would otherwise leave the old one behind in video
// memory. Shared pieces (the packets' ball and materials) are kept out of
// the trees this is called on.
import type * as THREE from "three";

export function disposeTree(root: THREE.Object3D): void {
  root.traverse((o) => {
    const mesh = o as THREE.Mesh;
    if (mesh.geometry) mesh.geometry.dispose();
    const mat = mesh.material as THREE.Material | THREE.Material[] | undefined;
    if (!mat) return;
    for (const m of Array.isArray(mat) ? mat : [mat]) {
      const map = (m as THREE.MeshBasicMaterial).map;
      if (map) map.dispose();
      m.dispose();
    }
  });
}
