import { build } from '../deployment/node_modules/esbuild/lib/main.js';
import { mkdir, copyFile } from 'node:fs/promises';
await mkdir('deployment', {recursive:true});
await build({entryPoints:['cloudflare/worker.ts'],bundle:true,platform:'node',target:'es2022',format:'esm',outfile:'deployment/worker.mjs',external:['cloudflare:workers']});
await copyFile(process.execPath, 'deployment/node.exe');
