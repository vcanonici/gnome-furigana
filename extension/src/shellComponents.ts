/**
 * Adapters binding `LabelTreeAnnotator` to specific Shell 46 components. Each adapter
 * only knows how to find its root actor and when the component is visible.
 */

import type Clutter from 'gi://Clutter';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import type {PopupMenu} from 'resource:///org/gnome/shell/ui/popupMenu.js';

import type {FuriganaPreferences} from './furiganaSettings.js';
import type {HoverController} from './hoverController.js';
import {LabelTreeAnnotator} from './labelTreeAnnotator.js';
import type {LinguisticClient} from './linguisticClient.js';

/** Lifecycle shared by every component adapter. */
export interface ShellComponent {
    update(preferences: FuriganaPreferences): void;
    destroy(): void;
}

// Upstream typings declare no PopupMenu signals; this is the one we use.
type MenuSignals = {'open-state-changed': [isOpen: boolean]};

// `Main.panel.statusArea` is typed `any` upstream; this is the part we rely on.
interface QuickSettingsButton {
    menu: PopupMenu<MenuSignals> & {box: Clutter.Actor; actor: Clutter.Actor};
    addExternalIndicator(indicator: Clutter.Actor): void;
}

function isQuickSettingsButton(value: unknown): value is QuickSettingsButton {
    if (typeof value !== 'object' || value === null || !('menu' in value) || !('addExternalIndicator' in value))
        return false;
    const menu: unknown = value.menu;
    return typeof menu === 'object' && menu !== null && 'box' in menu && 'actor' in menu;
}

/** The Quick Settings panel button, or null when its Shell 46 shape is not found. */
export function quickSettingsButton(): QuickSettingsButton | null {
    const candidate: unknown = Main.panel.statusArea.quickSettings;
    return isQuickSettingsButton(candidate) ? candidate : null;
}

/** Quick Settings menu: annotated only while open. */
export class QuickSettingsComponent implements ShellComponent {
    private readonly annotator: LabelTreeAnnotator;
    private readonly menu: QuickSettingsButton['menu'];
    private readonly openSignal: number;

    private constructor(button: QuickSettingsButton, client: LinguisticClient, hover: HoverController,
        preferences: FuriganaPreferences) {
        this.menu = button.menu;
        this.annotator = new LabelTreeAnnotator({context: 'quick-settings', root: this.menu.box,
            eventSource: this.menu.actor, layoutPolicy: {canGrowWidth: false, canAddRoom: true}},
        client, hover, preferences);
        this.openSignal = this.menu.connect('open-state-changed', (_menu, isOpen) => {
            this.onOpenStateChanged(isOpen);
            // Shell signal handlers must return undefined (true would stop emission).
            return undefined;
        });
        this.onOpenStateChanged(this.menu.isOpen);
    }

    private onOpenStateChanged(isOpen: boolean): void {
        if (isOpen)
            this.annotator.activate();
        else
            this.annotator.deactivate();
    }

    /** Returns null when the Shell layout differs from what this adapter supports. */
    static create(client: LinguisticClient, hover: HoverController, preferences: FuriganaPreferences): QuickSettingsComponent | null {
        const button = quickSettingsButton();
        return button ? new QuickSettingsComponent(button, client, hover, preferences) : null;
    }

    update(preferences: FuriganaPreferences): void {
        this.annotator.update(preferences);
    }

    destroy(): void {
        this.menu.disconnect(this.openSignal);
        this.annotator.destroy();
    }
}

/** Top bar (clock, app menu, indicators): always visible. */
export class PanelComponent implements ShellComponent {
    private readonly annotator: LabelTreeAnnotator;

    constructor(client: LinguisticClient, hover: HoverController, preferences: FuriganaPreferences) {
        // The top bar is as tall as the theme makes it: no room above, but labels may widen.
        this.annotator = new LabelTreeAnnotator({context: 'panel', root: Main.panel, eventSource: Main.panel,
            layoutPolicy: {canGrowWidth: true, canAddRoom: false}}, client, hover, preferences);
        this.annotator.activate();
    }

    update(preferences: FuriganaPreferences): void {
        this.annotator.update(preferences);
    }

    destroy(): void {
        this.annotator.destroy();
    }
}
