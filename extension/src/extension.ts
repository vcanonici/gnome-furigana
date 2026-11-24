/**
 * gnome-furigana: furigana above kanji and pt-BR translation on hover for the
 * Japanese GNOME Shell 46 interface. Linguistic work runs in a separate service
 * (D-Bus); this process only discovers labels, draws and tracks the pointer.
 */

import type Gio from 'gi://Gio';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

import {type FuriganaPreferences, readPreferences, watchPreferences} from './furiganaSettings.js';
import {FuriganaIndicator} from './furiganaToggle.js';
import {HoverController} from './hoverController.js';
import {LinguisticClient} from './linguisticClient.js';
import {PanelComponent, QuickSettingsComponent, quickSettingsButton, type ShellComponent} from './shellComponents.js';

interface Session {
    settings: Gio.Settings;
    preferences: FuriganaPreferences;
    signals: number[];
    client: LinguisticClient;
    hover: HoverController;
    components: ShellComponent[];
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
        quickSettingsButton()?.addExternalIndicator(indicator);
        const session: Session = {settings, preferences, client, hover, indicator, signals: [],
            components: this.createComponents(client, hover, preferences)};
        session.signals = watchPreferences(settings, next => { this.applyPreferences(next); });
        this.session = session;
    }

    override disable(): void {
        const session = this.session;
        this.session = null;
        if (!session)
            return;
        session.signals.forEach(id => { session.settings.disconnect(id); });
        session.components.forEach(component => { component.destroy(); });
        session.indicator.destroy();
        session.hover.destroy();
        session.client.destroy();
    }

    private createComponents(client: LinguisticClient, hover: HoverController,
        preferences: FuriganaPreferences): ShellComponent[] {
        const components: ShellComponent[] = [];
        if (preferences.quickSettings) {
            const quickSettings = QuickSettingsComponent.create(client, hover, preferences);
            if (quickSettings)
                components.push(quickSettings);
            else
                console.warn('[gnome-furigana] Quick Settings layout not recognised; component skipped');
        }
        if (preferences.panel)
            components.push(new PanelComponent(client, hover, preferences));
        return components;
    }

    private applyPreferences(preferences: FuriganaPreferences): void {
        const session = this.session;
        if (!session)
            return;
        const previous = session.preferences;
        session.preferences = preferences;
        session.hover.update(preferences);
        if (previous.quickSettings === preferences.quickSettings && previous.panel === preferences.panel) {
            session.components.forEach(component => { component.update(preferences); });
            return;
        }
        // Component switches change which actors we own: rebuild them cleanly.
        session.components.forEach(component => { component.destroy(); });
        session.components = this.createComponents(session.client, session.hover, preferences);
    }
}
