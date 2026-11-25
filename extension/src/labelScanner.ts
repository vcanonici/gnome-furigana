/**
 * Finds the mapped `St.Label`s of a Shell subtree without blocking a frame: the walk
 * runs in small slices on the main loop, and a new request while one is running is
 * merged into it. Unmapped subtrees (hidden UI), application windows, our own actors
 * and editable text are skipped; of an entry only its placeholder is reported.
 */

import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import St from 'gi://St';

/** Actors visited per main-loop iteration; a slice stays well under a millisecond. */
const SLICE_SIZE = 150;

export class LabelScanner {
    private pending: Clutter.Actor[] = [];
    private source: number | null = null;

    /**
     * @param root subtree to walk (the Shell's `uiGroup`).
     * @param isExcluded actors whose whole subtree is skipped (our overlay, windows).
     * @param onLabel called once per mapped label found by a walk.
     */
    constructor(private readonly root: Clutter.Actor, private readonly isExcluded: (actor: Clutter.Actor) => boolean,
        private readonly onLabel: (label: St.Label) => void) {}

    /** Starts a walk unless one is already in progress. */
    request(): void {
        if (this.source !== null)
            return;
        this.pending = [this.root];
        // Default priority: an idle or low-priority source starves while the Shell redraws.
        this.source = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 0, () => this.step());
    }

    /** Stops any walk in progress (extension disable). */
    cancel(): void {
        if (this.source !== null)
            GLib.source_remove(this.source);
        this.source = null;
        this.pending = [];
    }

    private step(): boolean {
        for (let budget = SLICE_SIZE; budget > 0; budget--) {
            const actor = this.pending.pop();
            if (!actor) {
                this.source = null;
                return GLib.SOURCE_REMOVE;
            }
            this.visit(actor);
        }
        return GLib.SOURCE_CONTINUE;
    }

    private visit(actor: Clutter.Actor): void {
        if (!actor.mapped || this.isExcluded(actor))
            return;
        if (actor instanceof St.Entry) {
            const hint = actor.get_hint_actor();
            if (hint instanceof St.Label && hint.mapped)
                this.onLabel(hint);
            return;
        }
        if (actor instanceof St.Label)
            this.onLabel(actor);
        this.pending.push(...actor.get_children());
    }
}
