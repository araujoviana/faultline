import type { StudioCore } from "./core";

/**
 * Adapter over the Rust core compiled by `bun run build:wasm`
 * (`wasm-pack build ../wasm --target web --out-dir src/lib/wasm`).
 *
 * Not yet on the default path — `main.ts` uses `createMemoryCore()` until the
 * wasm build is wired into `predev` / `prebuild`. Swap the import there once
 * `wasm-pack` is available.
 */
export async function loadWasmCore(): Promise<StudioCore> {
  const mod = await import("./wasm/strata_wasm.js");
  await mod.default();
  const studio = new mod.Studio();
  return {
    addResource: (kind, label, x, y) => studio.addResource(kind, label, x, y),
    connect: (from, to) => studio.connect(from, to),
    removeResource: (id) => studio.removeResource(id),
    configure: (id, variant, region, az) =>
      studio.configure(id, variant ?? "", region ?? "", az ?? ""),
    simulateFailure: (region, az) => studio.simulateFailure(region, az),
    findSpofs: () => studio.findSpofs(),
    profileJson: () => studio.profileJson(),
    stateJson: () => studio.stateJson(),
    loadJson: (json) => studio.loadJson(json),
  };
}
