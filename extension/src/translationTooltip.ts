/** The translation card shown after the hover delay; never reactive, so it cannot eat clicks. */

import Clutter from 'gi://Clutter';
import Mtk from 'gi://Mtk';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import type {TooltipContent} from './core/tooltipContent.js';
import type {StageRect} from './labelGeometry.js';

const GAP = 6;
const FADE_MS = 120;

export class TranslationTooltip {
    private readonly box = new St.BoxLayout({vertical: true, style_class: 'furigana-tooltip', reactive: false,
        visible: false, opacity: 0});
    private readonly heading = new St.Label({style_class: 'furigana-tooltip-heading'});
    private readonly senses = new St.Label({style_class: 'furigana-tooltip-senses'});
    private readonly details = new St.Label({style_class: 'furigana-tooltip-details'});
    private readonly phrase = new St.Label({style_class: 'furigana-tooltip-phrase'});

    constructor() {
        for (const label of [this.heading, this.senses, this.details, this.phrase])
            this.box.add_child(label);
        Main.layoutManager.uiGroup.add_child(this.box);
    }

    /** Shows the card under the word, or above it when there is no room below. */
    show(content: TooltipContent, anchor: StageRect): void {
        this.heading.text = content.heading;
        this.senses.text = content.senses;
        this.details.text = content.details;
        this.phrase.text = content.phrase ?? '';
        this.phrase.visible = content.phrase !== null;
        Main.layoutManager.uiGroup.set_child_above_sibling(this.box, null);
        this.box.show();
        this.place(anchor);
        this.box.remove_all_transitions();
        this.box.save_easing_state();
        this.box.set_easing_duration(FADE_MS);
        this.box.set_easing_mode(Clutter.AnimationMode.EASE_OUT_QUAD);
        this.box.opacity = 255;
        this.box.restore_easing_state();
    }

    hide(): void {
        this.box.remove_all_transitions();
        this.box.opacity = 0;
        this.box.hide();
    }

    destroy(): void {
        this.box.destroy();
    }

    private place(anchor: StageRect): void {
        const monitor = global.display.get_monitor_index_for_rect(new Mtk.Rectangle({
            x: Math.round(anchor.x), y: Math.round(anchor.y),
            width: Math.max(1, Math.round(anchor.width)), height: Math.max(1, Math.round(anchor.height)),
        }));
        const area = Main.layoutManager.getWorkAreaForMonitor(monitor);
        const [, , width, height] = this.box.get_preferred_size();
        const below = anchor.y + anchor.height + GAP;
        const y = below + height <= area.y + area.height ? below : anchor.y - height - GAP;
        const x = anchor.x + anchor.width / 2 - width / 2;
        this.box.set_position(
            Math.round(Math.min(Math.max(x, area.x), area.x + area.width - width)),
            Math.round(Math.max(y, area.y)));
    }
}
