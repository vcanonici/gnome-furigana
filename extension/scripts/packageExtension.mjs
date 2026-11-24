#!/usr/bin/env node
// Builds the installable GNOME Shell extension:
//   build/extension/<uuid>/            compiled JS + metadata + stylesheet + compiled schema
//   build/<uuid>.shell-extension.zip   the same tree, for `gnome-extensions install`
// Exit code 1 with the failing step on any error.
import {copyFileSync, mkdirSync, readFileSync, rmSync} from 'node:fs';
import {dirname, join, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';

import {run} from '../../ci/process.mjs';

const extensionRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repositoryRoot = resolve(extensionRoot, '..');
const {uuid} = JSON.parse(readFileSync(join(extensionRoot, 'metadata.json'), 'utf8'));
const outputDir = join(repositoryRoot, 'build', 'extension', uuid);
const zipPath = join(repositoryRoot, 'build', `${uuid}.shell-extension.zip`);

function step(name, action) {
    try {
        action();
    } catch (error) {
        throw new Error(`packageExtension: ${name}: ${error.message}`);
    }
}

function packageExtension() {
    step('clean', () => rmSync(outputDir, {recursive: true, force: true}));
    step('compile', () => run(process.execPath, [join(repositoryRoot, 'node_modules/typescript/bin/tsc'),
        '-p', join(extensionRoot, 'tsconfig.json')]));
    step('copy assets', () => {
        for (const file of ['metadata.json', 'stylesheet.css'])
            copyFileSync(join(extensionRoot, file), join(outputDir, file));
        mkdirSync(join(outputDir, 'schemas'), {recursive: true});
        copyFileSync(join(extensionRoot, 'schemas', 'org.gnome.shell.extensions.furigana.gschema.xml'),
            join(outputDir, 'schemas', 'org.gnome.shell.extensions.furigana.gschema.xml'));
    });
    step('compile schemas', () => run('glib-compile-schemas', ['--strict', join(outputDir, 'schemas')]));
    step('zip', () => {
        rmSync(zipPath, {force: true});
        run('zip', ['-qr', zipPath, '.'], {cwd: outputDir});
    });
    return {uuid, directory: outputDir, zip: zipPath};
}

try {
    console.log(JSON.stringify(packageExtension(), null, 2));
} catch (error) {
    console.error(error.message);
    process.exitCode = 1;
}
