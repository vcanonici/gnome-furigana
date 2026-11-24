/** "ふりがな" toggle in Quick Settings: on/off, display form, hover and a link to the settings panel. */

import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import {QuickMenuToggle, SystemIndicator} from 'resource:///org/gnome/shell/ui/quickSettings.js';

import {readPreferences} from './furiganaSettings.js';

const SETTINGS_PANEL = 'furigana';

const FuriganaMenuToggle = GObject.registerClass(
class FuriganaMenuToggle extends QuickMenuToggle {
    private readonly settings: Gio.Settings;
    private readonly formItems: Record<'ruby' | 'inline', PopupMenu.PopupMenuItem>;
    private readonly hoverItem: PopupMenu.PopupSwitchMenuItem;
    private readonly signals: number[] = [];

    constructor(settings: Gio.Settings, openPreferences: () => void) {
        super({title: 'ふりがな', iconName: 'accessories-dictionary-symbolic', toggleMode: true});
        this.settings = settings;
        this.menu.setHeader('accessories-dictionary-symbolic', 'Furigana', 'Leituras e traduções pt-BR');
        this.formItems = {
            ruby: this.addFormItem('Leitura acima do kanji', 'ruby'),
            inline: this.addFormItem('Leitura entre parênteses', 'inline'),
        };
        this.hoverItem = new PopupMenu.PopupSwitchMenuItem('Traduzir ao parar o mouse', true);
        this.hoverItem.connect('toggled', (_item, state: boolean) => this.settings.set_boolean('hover-enabled', state));
        this.menu.addMenuItem(this.hoverItem);
        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        this.menu.addAction('Configurações de Furigana', () => { openSettingsPanel(openPreferences); });
        this.connect('clicked', () => this.settings.set_string('annotate', this.checked ? 'all' : 'off'));
        this.signals.push(this.settings.connect('changed', () => { this.sync(); }));
        this.connect('destroy', () => { this.signals.forEach(id => { this.settings.disconnect(id); }); });
        this.sync();
    }

    private addFormItem(label: string, form: 'ruby' | 'inline'): PopupMenu.PopupMenuItem {
        const item = new PopupMenu.PopupMenuItem(label);
        item.connect('activate', () => this.settings.set_string('display-form', form));
        this.menu.addMenuItem(item);
        return item;
    }

    private sync(): void {
        const preferences = readPreferences(this.settings);
        this.checked = preferences.annotate;
        this.subtitle = preferences.displayForm === 'ruby' ? 'acima' : 'parênteses';
        for (const form of ['ruby', 'inline'] as const)
            this.formItems[form].setOrnament(form === preferences.displayForm ? PopupMenu.Ornament.DOT : PopupMenu.Ornament.NO_DOT);
        this.hoverItem.setToggleState(preferences.hoverEnabled);
    }
});

/** Opens the Furigana panel of the patched Settings app; falls back to the extension preferences. */
function openSettingsPanel(openPreferences: () => void): void {
    try {
        const [, argv] = GLib.shell_parse_argv(`gnome-control-center ${SETTINGS_PANEL}`);
        if (!hasFuriganaPanel())
            throw new Error('Settings fork not installed');
        GLib.spawn_async(null, argv ?? [], null, GLib.SpawnFlags.SEARCH_PATH, null);
    } catch {
        openPreferences();
    }
}

function hasFuriganaPanel(): boolean {
    const marker = GLib.build_filenamev([GLib.get_user_data_dir(), 'gnome-furigana', 'settings', 'VERSION']);
    return GLib.file_test(marker, GLib.FileTest.EXISTS);
}

/** System indicator holding the toggle; add with `quickSettings.addExternalIndicator`. */
export const FuriganaIndicator = GObject.registerClass(
class FuriganaIndicator extends SystemIndicator {
    constructor(settings: Gio.Settings, openPreferences: () => void) {
        super();
        this.quickSettingsItems.push(new FuriganaMenuToggle(settings, openPreferences));
    }

    override destroy(): void {
        this.quickSettingsItems.forEach(item => { item.destroy(); });
        super.destroy();
    }
});
