/**
 * Fallback preferences window (Extensions app). The primary UI is the "Furigana"
 * panel inside GNOME Settings; both edit the same GSettings keys.
 */

import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';
import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

function switchRow(settings: Gio.Settings, key: string, title: string, subtitle: string): Adw.SwitchRow {
    const row = new Adw.SwitchRow({title, subtitle});
    settings.bind(key, row, 'active', Gio.SettingsBindFlags.DEFAULT);
    return row;
}

function choiceRow(settings: Gio.Settings, key: string, title: string, choices: [string, string][]): Adw.ComboRow {
    const row = new Adw.ComboRow({title, model: Gtk.StringList.new(choices.map(([, label]) => label))});
    const sync = () => {
        row.selected = Math.max(0, choices.findIndex(([nick]) => nick === settings.get_string(key)));
    };
    sync();
    settings.connect(`changed::${key}`, sync);
    row.connect('notify::selected', () => settings.set_string(key, choices[row.selected]?.[0] ?? settings.get_string(key)));
    return row;
}

function delayRow(settings: Gio.Settings): Adw.SpinRow {
    const row = Adw.SpinRow.new_with_range(0.3, 10, 0.1);
    row.title = 'Tempo até a tradução (segundos)';
    row.digits = 1;
    row.value = settings.get_uint('hover-delay-ms') / 1000;
    row.connect('notify::value', () => settings.set_uint('hover-delay-ms', Math.round(row.value * 1000)));
    return row;
}

function group(title: string, rows: Gtk.Widget[]): Adw.PreferencesGroup {
    const preferencesGroup = new Adw.PreferencesGroup({title});
    rows.forEach(row => { preferencesGroup.add(row); });
    return preferencesGroup;
}

export default class FuriganaPreferences extends ExtensionPreferences {
    // Shell 46 calls this synchronously (async support arrived in 47).
    override fillPreferencesWindow(window: Adw.PreferencesWindow): void {
        const settings = this.getSettings();
        const page = new Adw.PreferencesPage({title: 'Furigana', icon_name: 'accessories-dictionary-symbolic'});
        page.add(group('Furigana', [
            choiceRow(settings, 'annotate', 'Anotar palavras', [['all', 'Todas com kanji'], ['off', 'Nenhuma']]),
            choiceRow(settings, 'display-form', 'Forma', [['ruby', 'Leitura acima do kanji'], ['inline', '漢字(かんじ)']]),
        ]));
        page.add(group('Tradução pt-BR', [
            switchRow(settings, 'hover-enabled', 'Traduzir ao parar o mouse', 'Cartão com leitura e significado'),
            delayRow(settings),
            choiceRow(settings, 'hover-scope', 'Alvo', [['word', 'Palavra sob o cursor'], ['label', 'Rótulo inteiro']]),
        ]));
        window.add(page);
    }
}
