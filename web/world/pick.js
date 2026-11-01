// Which tile a screen point means: a tap or click on the scene (walk there, or up to what is there) and the
// map editor's pointer.
import * as THREE from "three";

const ray = new THREE.Raycaster(), ndc = new THREE.Vector2(), hit = new THREE.Vector3();
const floor = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);
const box = new THREE.Box3();

function aim(canvas, camera, clientX, clientY) {
  const b = canvas.getBoundingClientRect();
  ndc.set(((clientX - b.left) / b.width) * 2 - 1, -((clientY - b.top) / b.height) * 2 + 1);
  ray.setFromCamera(ndc, camera);
}

// Raised ground first: the highest level whose tile under the ray stands at that height.
function floorTile(w) {
  for (const y of w.levels) {
    floor.constant = -y;
    if (!ray.ray.intersectPlane(floor, hit)) continue;
    const tx = Math.floor(hit.x), tz = Math.floor(hit.z);
    if (!y || Math.abs(w.floorAt(tx, tz) - y) <= 0.3) return [tx, tz];
  }
  return null;
}

/** The floor tile `[x, z]` under a screen point, or null. */
export function tileAt(w, canvas, camera, clientX, clientY) {
  aim(canvas, camera, clientX, clientY);
  return floorTile(w);
}

/** The tile `[x, z]` a tap means, or null: a person or something to use first (a standing box per tile, so
 * tapping a head or a signboard counts), else the floor. */
export function tapped(w, canvas, camera, clientX, clientY) {
  aim(canvas, camera, clientX, clientY);
  let best = null, bestD = Infinity;
  const test = (x, z, cx, cz, top) => {
    const y = w.floorAt(x, z);
    box.min.set(cx - 0.5, y, cz - 0.5);
    box.max.set(cx + 0.5, y + top, cz + 0.5);
    if (ray.ray.intersectBox(box, hit) && hit.distanceTo(ray.ray.origin) < bestD) (bestD = hit.distanceTo(ray.ray.origin)), (best = [x, z]);
  };
  for (const n of w.npcs) test(n.x, n.z, n.obj.position.x, n.obj.position.z, 1.7);
  for (const i of w.talk.values()) test(i.x, i.z, i.x + 0.5, i.z + 0.5, 1.1);
  for (const wp of w.warps.values()) if (!wp.dir) test(wp.x, wp.z, wp.x + 0.5, wp.z + 0.5, 2.2); // a door: walking into it enters
  for (const l of w.lifts) test(l.x, l.z, l.x + 0.5, l.z + 0.5, 2.4); // a lift's cage: walk in and ride
  return best ?? floorTile(w);
}
