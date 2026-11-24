/**
 * One Shell label under annotation. The original `St.Label` is never reparented and
 * stays reactive as before; furigana is drawn by separate non-reactive actors in an
 * overlay layer, so clicks, focus and accessibility are untouched (spec LAY-02).
 * The label's text changes only for the inline form, or to drop readings it already
 * carried when ruby is drawn instead; `destroy()` restores it.
 */

import type Clutter from 'gi://Clutter';
import St from 'gi://St';

import type {Analysis, Token} from './core/protocol.js';
import {decideFit, type FitDecision} from './core/layoutFit.js';
import {codePointLength, inlineToOriginalPositions, rubySpans, tokenAt, type RubySpan} from './core/textOffsets.js';
import type {FuriganaPreferences} from './furiganaSettings.js';
import {canHostRoom, foregroundCss, layoutHost, positionAt, scaledFontSize, spaceAbove, spanRect,
    type StageRect} from './labelGeometry.js';

const RUBY_FONT_FACTOR = 0.55;
// Ruby glyph boxes include leading; lower them so the reading sits close to the kanji.
const RUBY_DROP = 0.18;
/** Style class adding `padding-top` to the layout host; must match stylesheet.css. */
const ROOM_CLASS = 'furigana-room';
const ROOM_PX = 10;

/** A word found under the pointer. */
export interface WordHit {
    key: string;
    token: Token;
    phrase: Analysis['phrase'];
    rect: StageRect;
}

let nextLabelId = 1;

export class AnnotatedLabel {
    readonly id = nextLabelId++;
    private originalText: string;
    private analysis: Analysis | null = null;
    private decision: FitDecision | null = null;
    private rubyActors: {span: RubySpan; actor: St.Label}[] = [];
    private isApplyingText = false;
    private isTextReplaced = false;
    private readonly textSignal: number;
    private roomHost: St.Widget | null = null;
    private mappedSignal: number | null = null;

    constructor(readonly label: St.Label, private readonly overlay: Clutter.Actor,
        private readonly onTextChanged: (label: AnnotatedLabel) => void) {
        this.originalText = label.text;
        this.textSignal = label.connect('notify::text', () => {
            if (this.isApplyingText)
                return;
            // The Shell replaced the text: our decorations describe the old one.
            this.removeRuby();
            this.decision = null;
            this.isTextReplaced = false;
            this.originalText = label.text;
            this.onTextChanged(this);
        });
    }

    /** Text the Shell set, regardless of any inline annotation shown. */
    get sourceText(): string {
        return this.originalText;
    }

    private get isInline(): boolean {
        return this.decision?.form === 'inline';
    }

    /** Applies an analysis with the current preferences; `null` removes annotations. */
    apply(analysis: Analysis | null, preferences: FuriganaPreferences): void {
        this.clear();
        this.analysis = analysis;
        if (!analysis || analysis.tokens.length === 0 || !preferences.annotate)
            return;
        // Geometry is meaningless before the label is shown (e.g. during Shell startup).
        if (!this.label.mapped) {
            this.applyWhenMapped(analysis, preferences);
            return;
        }
        this.decision = this.measureFit(preferences);
        if (this.decision.form === 'inline') {
            this.setLabelText(analysis.display_inline);
            return;
        }
        if (this.decision.needsRoom)
            this.addRoom();
        // Readings already in the text (from ja@furigana catalogs) move above the kanji.
        if (analysis.plain_text !== this.originalText)
            this.setLabelText(analysis.plain_text);
        this.createRuby(analysis, this.decision.scale);
    }

    /** Moves ruby actors to follow the label (menus animate and re-layout). */
    syncPosition(): void {
        const isVisible = this.label.mapped && this.label.get_paint_opacity() > 0;
        for (const {span, actor} of this.rubyActors) {
            const rect = isVisible ? spanRect(this.label, span.start, span.end) : null;
            actor.visible = rect !== null;
            if (!rect)
                continue;
            // Follow the label's own fade (menus and the Overview animate opacity).
            actor.opacity = this.label.get_paint_opacity();
            const [, , width, height] = actor.get_preferred_size();
            actor.set_position(Math.round(rect.x + (rect.width - width) / 2), Math.round(rect.y - height * (1 - RUBY_DROP)));
        }
    }

    /** Word (or whole label, for `label` scope) under a stage point. */
    hitTest(stageX: number, stageY: number, scope: FuriganaPreferences['hoverScope']): WordHit | null {
        const analysis = this.analysis;
        const displayed = analysis ? positionAt(this.label, stageX, stageY) : null;
        if (!analysis || displayed === null)
            return null;
        const position = this.isInline ? inlineToOriginalPositions(analysis.plain_text, analysis.tokens)[displayed] : displayed;
        const token = position === undefined ? null : tokenAt(analysis.tokens, position);
        if (!token)
            return null;
        if (scope === 'label')
            return this.labelHit(analysis, token);
        const rect = this.isInline ? null : spanRect(this.label, token.start, token.end);
        return {key: `${this.id}:${token.start}`, token, phrase: analysis.phrase, rect: rect ?? this.labelRect()};
    }

    /** Removes every decoration and restores the original text (spec CA-F07). */
    destroy(): void {
        this.clear();
        this.label.disconnect(this.textSignal);
    }

    private applyWhenMapped(analysis: Analysis, preferences: FuriganaPreferences): void {
        this.mappedSignal = this.label.connect('notify::mapped', () => {
            if (this.label.mapped)
                this.apply(analysis, preferences);
        });
    }

    private labelHit(analysis: Analysis, token: Token): WordHit {
        const whole: Token = {...token, surface: analysis.plain_text, start: 0, end: codePointLength(analysis.plain_text),
            lemma: analysis.plain_text, meaning: analysis.phrase ?? token.meaning};
        return {key: `${this.id}:label`, token: whole, phrase: null, rect: this.labelRect()};
    }

    private labelRect(): StageRect {
        const [x, y] = this.label.get_transformed_position();
        const [width, height] = this.label.get_transformed_size();
        return {x, y, width, height};
    }

    private measureFit(preferences: FuriganaPreferences): FitDecision {
        const probe = this.createRubyLabel('あ', 1);
        const [, rubyHeight] = probe.get_preferred_height(-1);
        probe.destroy();
        return decideFit({
            spaceAbove: spaceAbove(this.label),
            extraRoom: canHostRoom(layoutHost(this.label)) ? ROOM_PX : 0,
            rubyHeight: rubyHeight * (1 - RUBY_DROP),
            minScale: preferences.minScale,
            preferredForm: preferences.displayForm,
        });
    }

    private createRuby(analysis: Analysis, scale: number): void {
        this.rubyActors = rubySpans(analysis.tokens).map(span => ({span, actor: this.createRubyLabel(span.reading, scale)}));
        this.syncPosition();
    }

    private createRubyLabel(reading: string, scale: number): St.Label {
        const actor = new St.Label({
            text: reading,
            style_class: 'furigana-ruby',
            style: `font-size: ${scaledFontSize(this.label, RUBY_FONT_FACTOR * scale)}; color: ${foregroundCss(this.label)};`,
            reactive: false,
        });
        this.overlay.add_child(actor);
        return actor;
    }

    private setLabelText(text: string): void {
        this.isApplyingText = true;
        this.label.text = text;
        this.isApplyingText = false;
        this.isTextReplaced = text !== this.originalText;
    }

    private removeRuby(): void {
        for (const {actor} of this.rubyActors)
            actor.destroy();
        this.rubyActors = [];
    }

    private addRoom(): void {
        this.roomHost = layoutHost(this.label);
        this.roomHost?.add_style_class_name(ROOM_CLASS);
    }

    private clear(): void {
        if (this.mappedSignal !== null)
            this.label.disconnect(this.mappedSignal);
        this.mappedSignal = null;
        this.removeRuby();
        this.roomHost?.remove_style_class_name(ROOM_CLASS);
        this.roomHost = null;
        if (this.isTextReplaced)
            this.setLabelText(this.originalText);
        this.decision = null;
    }
}
