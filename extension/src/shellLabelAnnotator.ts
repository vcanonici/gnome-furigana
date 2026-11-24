/**
 * Annotates every `St.Label` of the GNOME Shell UI (`Main.layoutManager.uiGroup`):
 * top bar, all menus and Quick Settings, the Overview (app grid, dash, search, window
 * captions), dialogs, notifications and the calendar. Only labels are touched;
 * editable text (`St.Entry`) and our own actors are skipped. Unmapped subtrees are not
 * walked, so hidden UI costs nothing.
 */

import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import {AnnotatedLabel, type WordHit} from './annotatedLabel.js';
import type {FuriganaPreferences} from './furiganaSettings.js';
import type {HoverController} from './hoverController.js';
import type {LinguisticClient} from './linguisticClient.js';

// CJK ideographs incl. extension A, compatibility and 々; cheap pre-filter before D-Bus.
const KANJI = /[㐀-䶿一-鿿豈-﫿々]|[\ud840-\ud87f][\udc00-\udfff]/u;
// Menus and the Overview add labels lazily; a periodic walk of mapped actors catches them.
const RESCAN_INTERVAL_MS = 750;
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
    private readonly overviewSignal: number;
    private readonly rescanSource: number;
    private readonly laterId: number;

    constructor(private readonly client: LinguisticClient, private readonly hover: HoverController,
        private preferences: FuriganaPreferences) {
        Main.layoutManager.uiGroup.add_child(this.overlay);
        this.eventSignal = global.stage.connect('captured-event', (_actor, event: Clutter.Event) => {
            this.handleEvent(event);
            return Clutter.EVENT_PROPAGATE;
        });
        this.overviewSignal = overview().connect('showing', () => { this.scan(); });
        this.rescanSource = GLib.timeout_add(GLib.PRIORITY_LOW, RESCAN_INTERVAL_MS, () => {
            this.scan();
            return GLib.SOURCE_CONTINUE;
        });
        this.laterId = global.compositor.get_laters().add(Meta.LaterType.BEFORE_REDRAW, () => {
            this.syncPositions();
            return GLib.SOURCE_CONTINUE;
        });
        this.scan();
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
        global.compositor.get_laters().remove(this.laterId);
        global.stage.disconnect(this.eventSignal);
        overview().disconnect(this.overviewSignal);
        for (const [label, annotated] of this.labels) {
            annotated.destroy();
            label.disconnect(this.destroySignals.get(label) ?? 0);
        }
        this.labels.clear();
        this.destroySignals.clear();
        this.overlay.destroy();
    }

    private scan(): void {
        for (const label of findMappedLabels(Main.layoutManager.uiGroup)) {
            if (this.labels.has(label) || !KANJI.test(label.text))
                continue;
            const annotated = new AnnotatedLabel(label, this.overlay, changed => void this.annotate(changed));
            this.labels.set(label, annotated);
            this.destroySignals.set(label, label.connect('destroy', () => { this.forget(label); }));
            void this.annotate(annotated);
        }
    }

    private forget(label: St.Label): void {
        this.labels.get(label)?.destroy();
        this.labels.delete(label);
        this.destroySignals.delete(label);
        this.hover.reset();
    }

    private async annotate(annotated: AnnotatedLabel): Promise<void> {
        const text = annotated.sourceText;
        const analysis = KANJI.test(text) ? await this.client.analyze(text, SERVICE_CONTEXT) : null;
        // The label may have changed or been destroyed while the call was in flight.
        if (this.labels.get(annotated.label) === annotated && annotated.sourceText === text)
            annotated.apply(analysis, this.preferences);
    }

    private syncPositions(): void {
        // Menus and dialogs created after us are stacked above; furigana must stay on top.
        const uiGroup = Main.layoutManager.uiGroup;
        if (uiGroup.get_last_child() !== this.overlay)
            uiGroup.set_child_above_sibling(this.overlay, null);
        for (const annotated of this.labels.values())
            annotated.syncPosition();
    }

    private handleEvent(event: Clutter.Event): void {
        const type = event.type();
        if (type === Clutter.EventType.MOTION) {
            const [x, y] = event.get_coords();
            this.hover.point(this.hitTest(x, y));
        } else if (type === Clutter.EventType.LEAVE) {
            // Typed non-null upstream, but null when the pointer leaves the stage.
            const related = event.get_related() as Clutter.Actor | null;
            if (related === null)
                this.hover.reset();
        } else if (type === Clutter.EventType.BUTTON_PRESS || type === Clutter.EventType.KEY_PRESS) {
            this.hover.reset();
        }
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

/**
 * Depth-first list of mapped `St.Label`s. Skips unmapped subtrees (hidden UI), editable
 * text and our own actors.
 */
function findMappedLabels(root: Clutter.Actor): St.Label[] {
    const found: St.Label[] = [];
    const pending: Clutter.Actor[] = [root];
    for (let actor = pending.pop(); actor; actor = pending.pop()) {
        // Typed as string upstream, but null for actors that were never named.
        const name = actor.name as string | null;
        if (!actor.mapped || actor instanceof St.Entry || name?.startsWith(OWN_ACTOR_PREFIX))
            continue;
        if (actor instanceof St.Label)
            found.push(actor);
        pending.push(...actor.get_children());
    }
    return found;
}
