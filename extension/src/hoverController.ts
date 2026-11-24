/** Connects pointer hits to the hover state machine and the translation card. */

import {HoverStateMachine} from './core/hoverStateMachine.js';
import {buildTooltipContent} from './core/tooltipContent.js';
import type {WordHit} from './annotatedLabel.js';
import type {FuriganaPreferences} from './furiganaSettings.js';
import {GLibScheduler} from './glibScheduler.js';
import {TranslationTooltip} from './translationTooltip.js';

export class HoverController {
    private readonly scheduler = new GLibScheduler();
    private readonly tooltip = new TranslationTooltip();
    private readonly machine: HoverStateMachine<WordHit>;
    private isEnabled: boolean;

    constructor(preferences: FuriganaPreferences) {
        this.isEnabled = preferences.hoverEnabled;
        this.machine = new HoverStateMachine<WordHit>(this.scheduler, {
            show: hit => { this.tooltip.show(buildTooltipContent(hit.token, hit.phrase), hit.rect); },
            hide: () => { this.tooltip.hide(); },
        }, preferences.hoverDelayMs);
    }

    /** Applies new preferences; disabling hover hides any visible card. */
    update(preferences: FuriganaPreferences): void {
        this.isEnabled = preferences.hoverEnabled;
        this.machine.setDelay(preferences.hoverDelayMs);
        if (!this.isEnabled)
            this.machine.reset();
    }

    /** Called on every pointer move with the word under the pointer (or null). */
    point(hit: WordHit | null): void {
        if (this.isEnabled && hit)
            this.machine.enter(hit.key, hit);
        else
            this.machine.reset();
    }

    /** Pointer left the component or a menu closed. */
    reset(): void {
        this.machine.reset();
    }

    /** A label went away: cancel only if the word under the pointer was in it. */
    forgetLabel(labelId: number): void {
        this.machine.resetIf(key => key.startsWith(`${labelId}:`));
    }

    destroy(): void {
        this.machine.reset();
        this.scheduler.cancelAll();
        this.tooltip.destroy();
    }
}
