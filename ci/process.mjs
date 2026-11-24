import { spawnSync } from 'node:child_process';

/** Execute argv without a shell; keep credentials out of diagnostics. */
export function run(command, args, options = {}) {
  const result = spawnSync(command, args, { encoding: 'utf8', maxBuffer: 16 * 1024 * 1024, ...options });
  if (result.error) throw new Error(`${command}: ${result.error.message}`);
  if (result.status !== 0) throw new Error(`${command} failed (${result.status}): ${(result.stderr ?? '').slice(-2000)}`);
  return (result.stdout || '').trim();
}

/** Decode options without interpreting user-provided values as shell commands. */
export function options(args) {
  const values = {};
  for (let index = 0; index < args.length; index++) {
    const key = args[index];
    if (!/^--[a-z][a-z-]*$/.test(key)) throw new Error(`Expected an option, got ${key}`);
    if (Object.hasOwn(values, key.slice(2))) throw new Error(`Duplicate option ${key}`);
    values[key.slice(2)] = args[index + 1] && !args[index + 1].startsWith('--') ? args[++index] : true;
  }
  return values;
}
