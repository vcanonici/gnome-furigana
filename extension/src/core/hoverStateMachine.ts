/**
 * Hover → tooltip state machine (spec §7.1), independent of GNOME APIs.
 *
 * IDLE ──enter(word)──► TRACKING ──delay elapsed──► SHOWING
 *   ▲                     │  enter(other word): restart timer
 *   └──── leave / reset ──┴── SHOWING: enter(other word) hides and restarts
 *
 * Repeated `enter` calls with the same key (small cursor moves inside one word)
 * do not restart the timer.
 */

/** Opaque handle returned by a scheduler. */
export type TimerHandle = number;

/** Timer source; GLib in the Shell, fake clock in tests. */
export interface Scheduler {
    schedule(delayMs: number, callback: () => void): TimerHandle;
    cancel(handle: TimerHandle): void;
}

/** Callbacks driven by the state machine. */
export interface HoverListener<T> {
    show(target: T): void;
    hide(): void;
}

type State<T> =
    | {kind: 'idle'}
    | {kind: 'tracking'; key: string; target: T; timer: TimerHandle}
    | {kind: 'showing'; key: string};

/** Debounces hover over words; `T` is whatever the tooltip needs to render. */
export class HoverStateMachine<T> {
    private state: State<T> = {kind: 'idle'};

    constructor(
        private readonly scheduler: Scheduler,
        private readonly listener: HoverListener<T>,
        private delayMs: number
    ) {}

    /** Changes the dwell time used for the next hover. */
    setDelay(delayMs: number): void {
        this.delayMs = delayMs;
    }

    /** The pointer is over the word identified by `key`. */
    enter(key: string, target: T): void {
        if (this.state.kind !== 'idle' && this.state.key === key)
            return;
        this.reset();
        const timer = this.scheduler.schedule(this.delayMs, () => { this.elapse(key); });
        this.state = {kind: 'tracking', key, target, timer};
    }

    /** The pointer left every word, the menu closed or the label disappeared. */
    reset(): void {
        if (this.state.kind === 'tracking')
            this.scheduler.cancel(this.state.timer);
        else if (this.state.kind === 'showing')
            this.listener.hide();
        this.state = {kind: 'idle'};
    }

    /** Resets only when the current word's key matches (e.g. its label was destroyed). */
    resetIf(isAffected: (key: string) => boolean): void {
        if (this.state.kind !== 'idle' && isAffected(this.state.key))
            this.reset();
    }

    /** True while a tooltip is visible. */
    get isShowing(): boolean {
        return this.state.kind === 'showing';
    }

    private elapse(key: string): void {
        if (this.state.kind !== 'tracking' || this.state.key !== key)
            return;
        const {target} = this.state;
        this.state = {kind: 'showing', key};
        this.listener.show(target);
    }
}
