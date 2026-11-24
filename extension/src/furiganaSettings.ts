/** Typed view over the extension's GSettings keys. */

import Gio from 'gi://Gio';

/** Snapshot of the preferences the Shell side uses. */
export interface FuriganaPreferences {
    displayForm: 'ruby' | 'inline';
    annotate: boolean;
    hoverEnabled: boolean;
    hoverDelayMs: number;
    hoverScope: 'word' | 'label';
    minScale: number;
    quickSettings: boolean;
    panel: boolean;
}

const KEYS = ['display-form', 'annotate', 'hover-enabled', 'hover-delay-ms', 'hover-scope', 'min-scale',
    'component-quick-settings', 'component-panel'] as const;

/** Reads all keys at once; values come from a schema with ranges, so no extra validation. */
export function readPreferences(settings: Gio.Settings): FuriganaPreferences {
    return {
        displayForm: settings.get_string('display-form') === 'inline' ? 'inline' : 'ruby',
        annotate: settings.get_string('annotate') !== 'off',
        hoverEnabled: settings.get_boolean('hover-enabled'),
        hoverDelayMs: settings.get_uint('hover-delay-ms'),
        hoverScope: settings.get_string('hover-scope') === 'label' ? 'label' : 'word',
        minScale: settings.get_double('min-scale'),
        quickSettings: settings.get_boolean('component-quick-settings'),
        panel: settings.get_boolean('component-panel'),
    };
}

/** Calls `onChange` with a fresh snapshot whenever any key changes; returns the signal ids. */
export function watchPreferences(settings: Gio.Settings, onChange: (preferences: FuriganaPreferences) => void): number[] {
    return KEYS.map(key => settings.connect(`changed::${key}`, () => { onChange(readPreferences(settings)); }));
}
