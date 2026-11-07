import { mkdir, copyFile } from 'node:fs/promises';
await mkdir('server-dist', { recursive: true });
await Promise.all(
  ['index.html', 'favicon.svg'].map((file) => copyFile(`public/${file}`, `server-dist/${file}`)),
);
console.log(
  'Built Vercel native coordination service. The desktop app builds separately with npm run native:build.',
);
