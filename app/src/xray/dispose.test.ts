// A car drawn again must give back everything the old one held on the
// graphics card: every geometry, material and texture of the tree.
import * as THREE from "three";
import { describe, expect, it } from "vitest";
import { disposeTree } from "./dispose";

function watch(target: THREE.EventDispatcher<{ dispose: object }>): () => number {
  let n = 0;
  target.addEventListener("dispose", () => n++);
  return () => n;
}

describe("disposeTree", () => {
  it("frees geometries, materials and their textures at every depth", () => {
    const root = new THREE.Group();
    const texture = new THREE.Texture();
    const geometry = new THREE.BoxGeometry();
    const material = new THREE.MeshBasicMaterial({ map: texture });
    const inner = new THREE.Mesh(new THREE.SphereGeometry(), [new THREE.MeshStandardMaterial(), new THREE.MeshStandardMaterial()]);
    const mesh = new THREE.Mesh(geometry, material);
    mesh.add(inner);
    root.add(new THREE.Group().add(mesh));

    const geometryFreed = watch(geometry);
    const materialFreed = watch(material);
    const textureFreed = watch(texture);
    const innerGeometryFreed = watch(inner.geometry);
    const innerMaterialsFreed = (inner.material as THREE.Material[]).map(watch);

    disposeTree(root);
    expect(geometryFreed()).toBe(1);
    expect(materialFreed()).toBe(1);
    expect(textureFreed()).toBe(1);
    expect(innerGeometryFreed()).toBe(1);
    expect(innerMaterialsFreed.map((f) => f())).toEqual([1, 1]);
  });

  it("leaves what is not in the tree alone", () => {
    const shared = new THREE.SphereGeometry();
    const sharedFreed = watch(shared);
    const root = new THREE.Group();
    root.add(new THREE.Mesh(new THREE.BoxGeometry(), new THREE.MeshBasicMaterial()));
    // The packets' ball lives outside the trees that are thrown away.
    const outside = new THREE.Mesh(shared, new THREE.MeshBasicMaterial());
    disposeTree(root);
    expect(sharedFreed()).toBe(0);
    expect(outside.geometry).toBe(shared);
  });
});
