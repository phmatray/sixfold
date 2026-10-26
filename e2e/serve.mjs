// Serves ../web/dist for the browser tests: `node serve.mjs <port>`.
// (python's http.server resets connections when a page opens many at once.)
import { createReadStream, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../web/dist/", import.meta.url));
const types = {
  ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm",
  ".json": "application/json", ".webmanifest": "application/manifest+json", ".svg": "image/svg+xml",
  ".webp": "image/webp", ".png": "image/png", ".glb": "model/gltf-binary",
};

createServer((req, res) => {
  try {
    let path = normalize(decodeURIComponent(new URL(req.url, "http://x").pathname)); // "/.." stays at "/"
    if (path.endsWith("/")) path += "index.html";
    const file = join(root, path);
    if (!statSync(file).isFile()) throw new Error("not a file");
    res.writeHead(200, { "content-type": types[extname(file)] ?? "application/octet-stream" });
    createReadStream(file).pipe(res);
  } catch {
    res.writeHead(404).end();
  }
}).listen(Number(process.argv[2]), "127.0.0.1");
