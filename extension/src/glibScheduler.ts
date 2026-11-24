/** `Scheduler` backed by GLib main-loop timeouts. */

import GLib from 'gi://GLib';

import type {Scheduler, TimerHandle} from './core/hoverStateMachine.js';

export class GLibScheduler implements Scheduler {
    private readonly pending = new Set<TimerHandle>();

    schedule(delayMs: number, callback: () => void): TimerHandle {
        const handle = GLib.timeout_add(GLib.PRIORITY_DEFAULT, delayMs, () => {
            this.pending.delete(handle);
            callback();
            return GLib.SOURCE_REMOVE;
        });
        this.pending.add(handle);
        return handle;
    }

    cancel(handle: TimerHandle): void {
        // Removing an already-fired source logs a critical warning; track what is live.
        if (this.pending.delete(handle))
            GLib.source_remove(handle);
    }

    /** Removes every pending timeout (extension disable). */
    cancelAll(): void {
        for (const handle of this.pending)
            GLib.source_remove(handle);
        this.pending.clear();
    }
}
