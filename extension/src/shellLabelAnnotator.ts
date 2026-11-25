/**
 * Annotates every `St.Label` of the GNOME Shell UI (`Main.layoutManager.uiGroup`):
 * top bar, all menus and Quick Settings, the Overview (app grid, dash, search, window
 * captions), dialogs, notifications and the calendar. Only labels are touched;
 * editable text (`St.Entry`) and our own actors are skipped.
 *
 * Cost per frame is kept near zero: labels are found by a sliced walk (`LabelScanner`),
 * only labels drawing ruby are repositioned, and the overlay is raised only when a new
 * actor joins the `uiGroup`.
 */

import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import {AnnotatedLabel, type WordHit} from './annotatedLabel.js';
import type {FuriganaPreferences} from './furiganaSettings.js';
import type {HoverController} from './hoverController.js';
import {LabelScanner} from './labelScanner.js';
import type {LinguisticClient} from './linguisticClient.js';

// CJK ideographs incl. extension A, compatibility and 々; cheap pre-filter before D-Bus.
const KANJI = /[㐀-䶿一-鿿豈-﫿々]|[\ud840-\ud87f][\udc00-\udfff]/u;
// Menus and the Overview show labels lazily; a periodic sliced walk catches them.
const RESCAN_INTERVAL_MS = 1000;
/** Name prefix of our own actors (overlay, tooltip), which must never be annotated. */
export const OWN_ACTOR_PREFIX = 'furigana-';
const SERVICE_CONTEXT = 'shell';

// `Main.overview` is typed `any` upstream; these are the only calls we make on it.
interface OverviewEvents {
    connect(signal: 'showing', callback: () => void): number;
    disconnect(id: number): void;
}
const overview = (): OverviewEvents => Main.overview as OverviewEvents;

export class ShellLabelAnnotator {
    private readonly overlay = new St.Widget({name: `${OWN_ACTOR_PREFIX}overlay`, reactive: false});
    private readonly labels = new Map<St.Label, AnnotatedLabel>();
    private readonly destroySignals = new Map<St.Label, number>();
    private readonly eventSignal: number;
    private readonly cursorTracker = Meta.CursorTracker.get_for_display(global.display);
    private readonly cursorSignal: number;
    private readonly overviewSignal: number;
    private readonly rescanSource: number;
    private readonly laterId: number;
    private readonly childAddedSignal: number;
    private readonly scanner: LabelScanner;

    constructor(private readonly client: LinguisticClient, private readonly hover: HoverController,
        private preferences: FuriganaPreferences) {
        const uiGroup = Main.layoutManager.uiGroup;
        uiGroup.add_child(this.overlay);
        this.scanner = new LabelScanner(uiGroup, actor => this.isExcluded(actor), label => { this.track(label); });
        // Menus and dialogs created after us are stacked above; furigana must stay on top.
        // Raising on every frame instead fights other actors and redraws forever.
        this.childAddedSignal = uiGroup.connect('child-added', () => { this.raiseOverlay(); });
        // Pointer motion comes from the cursor tracker: an open menu grabs input, and
        // during a grab the stage receives no events, so stage signals would miss hovers.
        this.cursorSignal = this.cursorTracker.connect('position-invalidated', () => { this.handlePointerMoved(); });
        this.eventSignal = global.stage.connect('captured-event', (_actor, event: Clutter.Event) => {
            this.handleEvent(event);
            return Clutter.EVENT_PROPAGATE;
        });
        this.overviewSignal = overview().connect('showing', () => { this.scanner.request(); });
        // Default priority: a low-priority source can starve while the Shell keeps redrawing.
        this.rescanSource = GLib.timeout_add(GLib.PRIORITY_DEFAULT, RESCAN_INTERVAL_MS, () => {
            this.scanner.request();
            return GLib.SOURCE_CONTINUE;
        });
        this.laterId = global.compositor.get_laters().add(Meta.LaterType.BEFORE_REDRAW, () => {
            this.syncPositions();
            return GLib.SOURCE_CONTINUE;
        });
        this.scanner.request();
    }

    /** Re-renders every label with new preferences. */
    update(preferences: FuriganaPreferences): void {
        this.preferences = preferences;
        for (const annotated of this.labels.values())
            void this.annotate(annotated);
    }

    /** Removes all decorations, restores texts and disconnects (spec CA-F07). */
    destroy(): void {
        GLib.source_remove(this.rescanSource);
        this.scanner.cancel();
        Main.layoutManager.uiGroup.disconnect(this.childAddedSignal);
        global.compositor.get_laters().remove(this.laterId);
        global.stage.disconnect(this.eventSignal);
        this.cursorTracker.disconnect(this.cursorSignal);
        overview().disconnect(this.overviewSignal);
        for (const [label, annotated] of this.labels) {
            annotated.destroy();
            label.disconnect(this.destroySignals.get(label) ?? 0);
        }
        this.labels.clear();
        this.destroySignals.clear();
        this.overlay.destroy();
    }

    private isExcluded(actor: Clutter.Actor): boolean {
        // Typed as string upstream, but null for actors that were never named.
        const name = actor.name as string | null;
        return actor === global.window_group || (name?.startsWith(OWN_ACTOR_PREFIX) ?? false);
    }

    private track(label: St.Label): void {
        if (this.labels.has(label) || !KANJI.test(label.text))
            return;
        const annotated = new AnnotatedLabel(label, this.overlay, changed => void this.annotate(changed));
        this.labels.set(label, annotated);
        this.destroySignals.set(label, label.connect('destroy', () => { this.forget(label); }));
        void this.annotate(annotated);
    }

    private forget(label: St.Label): void {
        const annotated = this.labels.get(label);
        if (annotated) {
            // Other labels disappear all the time (notifications expire): keep their hover.
            this.hover.forgetLabel(annotated.id);
            annotated.destroy();
        }
        this.labels.delete(label);
        this.destroySignals.delete(label);
    }

    private async annotate(annotated: AnnotatedLabel): Promise<void> {
        const text = annotated.sourceText;
        const analysis = KANJI.test(text) ? await this.client.analyze(text, SERVICE_CONTEXT) : null;
        // The label may have changed or been destroyed while the call was in flight.
        if (this.labels.get(annotated.label) === annotated && annotated.sourceText === text)
            annotated.apply(analysis, this.preferences);
    }

    private raiseOverlay(): void {
        const uiGroup = Main.layoutManager.uiGroup;
        if (uiGroup.get_last_child() !== this.overlay)
            uiGroup.set_child_above_sibling(this.overlay, null);
    }

    /** Runs before every redraw: touches only labels that draw ruby. */
    private syncPositions(): void {
        for (const annotated of this.labels.values()) {
            if (annotated.hasRuby)
                annotated.syncPosition();
        }
    }

    private handlePointerMoved(): void {
        const [x, y] = global.get_pointer();
        this.hover.point(this.hitTest(x, y));
    }

    /** Clicks and key presses dismiss the card (when no grab hides them from the stage). */
    private handleEvent(event: Clutter.Event): void {
        const type = event.type();
        if (type === Clutter.EventType.BUTTON_PRESS || type === Clutter.EventType.KEY_PRESS)
            this.hover.reset();
    }

    private hitTest(stageX: number, stageY: number): WordHit | null {
        for (const annotated of this.labels.values()) {
            const hit = annotated.label.mapped ? annotated.hitTest(stageX, stageY, this.preferences.hoverScope) : null;
            if (hit)
                return hit;
        }
        return null;
    }
}
