#!/usr/bin/env node
// Trimmed public copy of the starter's CI CLI: local Nx orchestration plus
// GitHub run inspection. The private OCI/registry transport was removed on purpose.
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { options, run } from './process.mjs';

const help = `gnome-furigana CI tooling — Node/Nx orchestration of the real project targets
Usage: node ci/cli.mjs <command> [options] [--json]
  doctor                          Inspect local prerequisites
  plan                            Show configured Nx targets without executing them
  run [--full | --base SHA --head SHA]  Run configured targets locally
  status [--repo OWNER/REPO]      Show the latest GitHub executions
  logs --run ID [--repo OWNER/REPO]   Show failed job logs
  retry --run ID [--repo OWNER/REPO]  Repeat only failed jobs
  watch --run ID [--repo OWNER/REPO]  Wait with a 60-second polling interval`;

/** Read explicit project commands; absent targets are never invented as successes. */
export function projectPlan(path = resolve('ci/config.json')) {
  const value = JSON.parse(readFileSync(path, 'utf8'));
  if (value.schemaVersion !== 1 || !Array.isArray(value.targets) || !value.targets.length
    || value.targets.some(target => !/^[a-z][a-z0-9-]*$/.test(target))) throw new Error('Expected CI schemaVersion 1 and explicit Nx targets');
  return value;
}

function repositoryArgs(settings) {
  const repo = settings.repo || process.env.GITHUB_REPOSITORY;
  return repo ? ['--repo', repo] : [];
}

function optionalVersion(command, args) {
  try { return run(command, args).split('\n')[0]; } catch { return null; }
}

function runTargets(settings) {
  const config = projectPlan();
  const nxPackage = resolve('node_modules/nx');
  const nxManifest = JSON.parse(readFileSync(resolve(nxPackage, 'package.json'), 'utf8'));
  const nx = resolve(nxPackage, typeof nxManifest.bin === 'string' ? nxManifest.bin : nxManifest.bin.nx);
  const task = settings.full ? ['run-many', '--all'] : ['affected', `--base=${settings.base || 'origin/main'}`, `--head=${settings.head || 'HEAD'}`];
  run(process.execPath, [nx, ...task, '-t', config.targets.join(','), '--parallel=1'], { stdio: ['inherit', 2, 2] });
  return { status: 'passed', targets: config.targets, scope: task };
}

async function githubRun(command, settings) {
  if (!/^\d+$/.test(settings.run || '')) throw new Error('Expected --run ID');
  const repo = repositoryArgs(settings);
  if (command === 'logs') return run('gh', ['run', 'view', settings.run, ...repo, '--log-failed']);
  if (command === 'retry') return run('gh', ['run', 'rerun', settings.run, ...repo, '--failed']) || { status: 'retry-requested', run: settings.run };
  while (true) {
    const state = JSON.parse(run('gh', ['run', 'view', settings.run, ...repo, '--json', 'status,conclusion,headSha,url']));
    if (state.status === 'completed') {
      if (state.conclusion !== 'success') throw new Error(`CI completed with ${state.conclusion}: ${state.url}`);
      return state;
    }
    await new Promise(done => setTimeout(done, 60000));
  }
}

/** Execute local checks or explicit GitHub operations with meaningful exit codes. */
export async function main(args) {
  const [command] = args;
  if (!command || command === '--help' || command === 'help') return help;
  const settings = options(args.slice(1));
  if (command === 'doctor') return { node: process.version, git: optionalVersion('git', ['--version']),
    nxInstalled: existsSync(resolve('node_modules/.bin/nx')), python: optionalVersion('python3', ['--version']),
    gjs: optionalVersion('gjs', ['--version']), shellcheck: optionalVersion('shellcheck', ['--version']),
    gnomeShell: optionalVersion('gnome-shell', ['--version']) };
  if (command === 'plan') return projectPlan();
  if (command === 'run') return runTargets(settings);
  if (command === 'status') return JSON.parse(run('gh', ['run', 'list', ...repositoryArgs(settings), '--limit', '5', '--json', 'databaseId,name,headSha,status,conclusion,url']));
  if (['logs', 'retry', 'watch'].includes(command)) return githubRun(command, settings);
  throw new Error(help);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { const result = await main(process.argv.slice(2).filter(arg => arg !== '--json')); console.log(typeof result === 'string' ? result : JSON.stringify(result, null, 2)); }
  catch (error) { console.error(error.message); process.exitCode = 1; }
}
