/**
 * Fallback types for the `wasm-pack --target web` output, which only exists
 * after `bun run build:wasm`. When the real `strata_wasm.d.ts` is present it
 * takes precedence over this ambient declaration.
 */
declare module "*/wasm/strata_wasm.js" {
  export default function init(input?: unknown): Promise<unknown>;
  export class Studio {
    constructor();
    addResource(kind: string, label: string, x: number, y: number): string;
    connect(from: string, to: string): void;
    moveResource(id: string, x: number, y: number): void;
    removeResource(id: string): void;
    configure(id: string, variant: string, region: string, az: string): void;
    simulateFailure(region: string, az: string): string;
    findSpofs(): string;
    profileJson(): string;
    stateJson(): string;
    loadJson(json: string): void;
  }
}
