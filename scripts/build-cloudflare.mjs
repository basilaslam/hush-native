import { build } from '../setup/node_modules/esbuild/lib/main.js';
await build({entryPoints:['cloudflare/worker.ts'],bundle:true,platform:'node',target:'es2022',format:'esm',outfile:'setup/worker.mjs',external:['cloudflare:workers']});
