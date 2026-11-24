/**
 * gnome-furigana: furigana above kanji (or inline when there is no room) and pt-BR
 * translation on hover for the whole Japanese GNOME Shell 46 interface. Linguistic
 * work runs in a separate service (D-Bus); this process only discovers labels, draws
 * and tracks the pointer. GTK applications get their readings from the `ja@furigana`
 * gettext catalogs instead.
 */

import type Gio from 'gi://Gio';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

import {readPreferences, watchPreferences} from './furiganaSettings.js';
import {addToQuickSettings, FuriganaIndicator} from './furiganaToggle.js';
import {HoverController} from './hoverController.js';
import {LinguisticClient} from './linguisticClient.js';
import {ShellLabelAnnotator} from './shellLabelAnnotator.js';

interface Session {
    settings: Gio.Settings;
    signals: number[];
    client: LinguisticClient;
    hover: HoverController;
    annotator: ShellLabelAnnotator;
    indicator: InstanceType<typeof FuriganaIndicator>;
}

export default class FuriganaExtension extends Extension {
    private session: Session | null = null;

    override enable(): void {
        const settings = this.getSettings();
        const preferences = readPreferences(settings);
        const client = new LinguisticClient();
        const hover = new HoverController(preferences);
        const indicator = new FuriganaIndicator(settings, () => { this.openPreferences(); });
        if (!addToQuickSettings(indicator))
            console.warn('[gnome-furigana] Quick Settings layout not recognised; toggle not added');
        const annotator = new ShellLabelAnnotator(client, hover, preferences);
        const signals = watchPreferences(settings, next => {
            hover.update(next);
            annotator.update(next);
        });
        this.session = {settings, signals, client, hover, annotator, indicator};
    }

    override disable(): void {
        const session = this.session;
        this.session = null;
        if (!session)
            return;
        session.signals.forEach(id => { session.settings.disconnect(id); });
        session.annotator.destroy();
        session.indicator.destroy();
        session.hover.destroy();
        session.client.destroy();
    }
}
