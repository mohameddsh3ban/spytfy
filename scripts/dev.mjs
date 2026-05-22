import { execSync, spawn } from 'child_process';
import { resolve } from 'path';

const ROOT = resolve(import.meta.dirname, '..');
const CRATE = resolve(ROOT, 'spytfy');

const PORTS = [4200, 1420];

function killPorts() {
  for (const port of PORTS) {
    try {
      const out = execSync(
        `netstat -ano | findstr :${port} | findstr LISTENING`,
        { encoding: 'utf-8', stdio: ['pipe', 'pipe', 'pipe'] }
      );
      const pids = new Set(
        out.split('\n')
          .map(l => l.trim().split(/\s+/).pop())
          .filter(Boolean)
          .filter(p => p !== '0')
      );
      for (const pid of pids) {
        try {
          execSync(`taskkill /F /PID ${pid}`, { stdio: 'pipe' });
          console.log(`Killed PID ${pid} on port ${port}`);
        } catch {}
      }
    } catch {}
  }
}

function killOrphanProcesses() {
  const names = ['spytfy-app.exe', 'yt-dlp-x86_64-pc-windows-gnu.exe'];
  for (const name of names) {
    try {
      execSync(`taskkill /F /IM ${name}`, { stdio: 'pipe' });
      console.log(`Killed orphan ${name}`);
    } catch {}
  }
}

function cleanTempFiles() {
  try {
    execSync('del /s /q *.webm.part 2>nul', { cwd: CRATE, stdio: 'pipe', shell: true });
  } catch {}
}

console.log('--- Cleaning up ---');
killPorts();
killOrphanProcesses();
cleanTempFiles();
console.log('--- Starting Spytfy ---\n');

const child = spawn('pnpm', ['tauri:dev'], {
  cwd: CRATE,
  stdio: 'inherit',
  shell: true,
  env: {
    ...process.env,
    CARGO_TARGET_DIR: 'C:\\dev\\cargo-target',
    NX_INTERACTIVE: 'false',
  },
});

child.on('exit', (code) => process.exit(code ?? 0));

process.on('SIGINT', () => {
  child.kill('SIGINT');
  process.exit(0);
});
