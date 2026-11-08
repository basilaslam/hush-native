// Exercise the shipped PowerShell functions against a real binary HTTP response.
import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

if (process.platform !== 'win32') throw new Error('Run this check on Windows.');
const scratch = await mkdtemp(join(tmpdir(), 'hush text test '));
const expected = `${'0'.repeat(64)}  Example-Installer.exe\r\n`;
const server = createServer((_, response) => {
  response.writeHead(200, { 'Content-Type': 'application/octet-stream' });
  response.end(Buffer.from(expected));
});
await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const quote = (value) => `'${value.replaceAll("'", "''")}'`;
try {
  const script = `
$ErrorActionPreference = 'Stop'
$scratch = ${quote(scratch)}
$ast = [Management.Automation.Language.Parser]::ParseFile(${quote(resolve('scripts/setup-server.ps1'))}, [ref]$null, [ref]$null)
foreach ($name in @('Get-Text', 'Get-Checksum')) {
  $definition = $ast.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $name }, $true)
  if (-not $definition) { throw "Missing shipped function: $name" }
  Invoke-Expression $definition.Extent.Text
}
$text = Get-Text 'http://127.0.0.1:${server.address().port}/SHA256SUMS.txt'
$expected = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('${Buffer.from(expected).toString('base64')}'))
if ($text -isnot [string] -or $text -cne $expected) { throw 'Binary HTTP metadata was not decoded to the exact original text' }
if ((Get-Checksum $text 'Example-Installer.exe') -ne ('0' * 64)) { throw 'Checksum parser rejected valid CRLF metadata' }
$rejected = $false
try { Get-Checksum $text 'Hush-Server-Setup.zip' } catch {
  if ($_.Exception.Message -match 'Missing or ambiguous checksum') { $rejected = $true } else { throw }
}
if (-not $rejected) { throw 'Missing asset checksum was accepted' }
Write-Output 'Shipped PowerShell read and validated binary-served checksum metadata.'
`;
  await new Promise((resolve, reject) => {
    const child = spawn('powershell.exe', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-Command', script], { stdio: 'inherit' });
    child.on('error', reject);
    child.on('exit', (code) => code === 0 ? resolve() : reject(new Error(`PowerShell metadata check failed: ${code}`)));
  });
} finally {
  await new Promise((resolve) => server.close(resolve));
  await rm(scratch, { recursive: true, force: true });
}
