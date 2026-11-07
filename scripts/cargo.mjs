import { existsSync } from 'node:fs';
import { spawn } from 'node:child_process';
import { resolve, delimiter } from 'node:path';
const local = resolve('.tools/cargo/bin', process.platform === 'win32' ? 'cargo.exe' : 'cargo');
const available = existsSync(local);
const env = { ...process.env };
if (available) {
  env.CARGO_HOME = resolve('.tools/cargo');
  env.RUSTUP_HOME = resolve('.tools/rustup');
  env.PATH = `${resolve('.tools/cargo/bin')}${delimiter}${env.PATH ?? ''}`;
}
const child = spawn(available ? local : 'cargo', process.argv.slice(2), { stdio: 'inherit', env });
child.on('error', () => {
  console.error('Install the Rust stable toolchain from https://rustup.rs, then retry.');
  process.exitCode = 1;
});
child.on('exit', (code) => {
  process.exitCode = code ?? 1;
});
