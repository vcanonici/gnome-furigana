/**
 * Annotates the `St.Label`s under one root actor of a Shell component (the Quick
 * Settings menu, the top bar). Only labels are touched; editable text and other
 * actors are ignored by construction. Work happens only while the component is
 * active (e.g. its menu is open).
 */

import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import {AnnotatedLabel, type LabelLayoutPolicy, type WordHit} from './annotatedLabel.js';
import type {FuriganaPreferences} from './furiganaSettings.js';
import type {HoverController} from './hoverController.js';
import type {LinguisticClient} from './linguisticClient.js';

// CJK ideographs incl. extension A, compatibility and 々; cheap pre-filter before D-Bus.
const KANJI = /[㐀-䶿一-鿿豈-﫿々]|[\ud840-\ud87f][\udc00-\udfff]/u;
// Menus add rows lazily (Wi-Fi networks, submenus); rescan while active.
const RESCAN_INTERVAL_MS = 750;

/** Static description of one annotated Shell component. */
export interface ComponentTarget {
    /** Service context name, e.g. `quick-settings`. */
    context: string;
    /** Subtree whose labels are annotated. */
    root: Clutter.Actor;
    /** Actor whose captured events reveal pointer motion over the subtree. */
    eventSource: Clutter.Actor;
    /** How labels of this component may change their surroundings. */
    layoutPolicy: LabelLayoutPolicy;
}

export class LabelTreeAnnotator {
    private readonly overlay = new St.Widget({reactive: false, style_class: 'furigana-overlay'});
    private readonly labels = new Map<St.Label, AnnotatedLabel>();
    private readonly destroySignals = new Map<St.Label, number>();
    private readonly eventSignal: number;
    private rescanSource: number | null = null;
    private laterId: number | null = null;
    private isActive = false;

    constructor(private readonly target: ComponentTarget, private readonly client: LinguisticClient,
        private readonly hover: HoverController, private preferences: FuriganaPreferences) {
        Main.layoutManager.uiGroup.add_child(this.overlay);
        this.eventSignal = target.eventSource.connect('captured-event', (_actor, event: Clutter.Event) => {
            this.handleEvent(event);
            return Clutter.EVENT_PROPAGATE;
        });
    }

    /** Starts annotating (component became visible). */
    activate(): void {
        this.isActive = true;
        Main.layoutManager.uiGroup.set_child_above_sibling(this.overlay, null);
        this.overlay.show();
        this.scan();
        this.rescanSource ??= GLib.timeout_add(GLib.PRIORITY_LOW, RESCAN_INTERVAL_MS, () => {
            this.scan();
            return GLib.SOURCE_CONTINUE;
        });
        this.laterId ??= global.compositor.get_laters().add(Meta.LaterType.BEFORE_REDRAW, () => {
            this.syncPositions();
            return GLib.SOURCE_CONTINUE;
        });
    }

    /** Stops work while the component is hidden; decorations are kept for the next open. */
    deactivate(): void {
        this.isActive = false;
        this.hover.reset();
        this.overlay.hide();
        this.stopTimers();
    }

    /** Re-renders every label with new preferences. */
    update(preferences: FuriganaPreferences): void {
        this.preferences = preferences;
        for (const annotated of this.labels.values())
            void this.annotate(annotated);
    }

    /** Removes all decorations, restores texts and disconnects (spec CA-F07). */
    destroy(): void {
        this.stopTimers();
        this.target.eventSource.disconnect(this.eventSignal);
        for (const [label, annotated] of this.labels) {
            annotated.destroy();
            label.disconnect(this.destroySignals.get(label) ?? 0);
        }
        this.labels.clear();
        this.destroySignals.clear();
        this.overlay.destroy();
    }

    private stopTimers(): void {
        if (this.rescanSource !== null)
            GLib.source_remove(this.rescanSource);
        if (this.laterId !== null)
            global.compositor.get_laters().remove(this.laterId);
        this.rescanSource = null;
        this.laterId = null;
    }

    private scan(): void {
        for (const label of findLabels(this.target.root)) {
            if (this.labels.has(label) || !KANJI.test(label.text))
                continue;
            const annotated = new AnnotatedLabel(label, this.overlay, this.target.layoutPolicy,
                changed => void this.annotate(changed));
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
        const analysis = KANJI.test(text) ? await this.client.analyze(text, this.target.context) : null;
        // The label may have changed or been destroyed while the call was in flight.
        if (this.labels.get(annotated.label) === annotated && annotated.sourceText === text)
            annotated.apply(analysis, this.preferences);
    }

    private syncPositions(): void {
        this.overlay.opacity = this.target.root.get_paint_opacity();
        for (const annotated of this.labels.values())
            annotated.syncPosition();
    }

    private handleEvent(event: Clutter.Event): void {
        if (!this.isActive)
            return;
        const type = event.type();
        if (type === Clutter.EventType.LEAVE) {
            // Typed non-null upstream, but null when the pointer leaves the stage.
            const related = event.get_related() as Clutter.Actor | null;
            if (related === null || !this.target.eventSource.contains(related))
                this.hover.reset();
        } else if (type === Clutter.EventType.MOTION) {
            const [x, y] = event.get_coords();
            this.hover.point(this.hitTest(x, y));
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

/** Depth-first list of visible `St.Label`s; skips `St.Entry` subtrees (editable text). */
function findLabels(root: Clutter.Actor): St.Label[] {
    const found: St.Label[] = [];
    const pending: Clutter.Actor[] = [root];
    for (let actor = pending.pop(); actor; actor = pending.pop()) {
        if (actor instanceof St.Entry || !actor.visible)
            continue;
        if (actor instanceof St.Label)
            found.push(actor);
        pending.push(...actor.get_children());
    }
    return found;
}
