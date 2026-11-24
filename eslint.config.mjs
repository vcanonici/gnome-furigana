// Lint rules for the TypeScript sources (GNOME Shell extension and CI tooling).
import js from '@eslint/js';
import tseslint from 'typescript-eslint';

export default tseslint.config(
    {ignores: ['build/**', 'node_modules/**', 'service/target/**', 'settings-panel/**']},
    js.configs.recommended,
    ...tseslint.configs.strictTypeChecked,
    {
        files: ['extension/**/*.ts'],
        languageOptions: {
            parserOptions: {project: ['extension/tsconfig.json', 'extension/tsconfig.test.json'], tsconfigRootDir: import.meta.dirname},
        },
        rules: {
            '@typescript-eslint/no-explicit-any': 'error',
            '@typescript-eslint/restrict-template-expressions': ['error', {allowNumber: true}],
            'max-lines-per-function': ['warn', {max: 40, skipBlankLines: true, skipComments: true}],
            'max-depth': ['error', 3],
            'eqeqeq': 'error',
        },
    },
    // node:test's test() returns a promise the runner awaits itself.
    {files: ['extension/test/**/*.ts'], rules: {'@typescript-eslint/no-floating-promises': 'off'}},
    {files: ['**/*.mjs'], ...tseslint.configs.disableTypeChecked},
    {files: ['**/*.mjs'], languageOptions: {globals: {process: 'readonly', console: 'readonly'}}},
);
