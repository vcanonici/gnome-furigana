/** Measurements of an `St.Label` in stage coordinates, built on `Clutter.Text`. */

import type Clutter from 'gi://Clutter';
import Pango from 'gi://Pango';
import St from 'gi://St';

/** Axis-aligned rectangle in stage coordinates. */
export interface StageRect {
    x: number;
    y: number;
    width: number;
    height: number;
}

/** Stage rectangle of the characters `[start, end)` of a single-line label, or null if clipped. */
export function spanRect(label: St.Label, start: number, end: number): StageRect | null {
    const text = label.clutter_text;
    const [okStart, x1, y1, lineHeight] = text.position_to_coords(start);
    const [okEnd, x2] = text.position_to_coords(end);
    if (!okStart || !okEnd || x2 > text.width + 0.5)
        return null;
    const [stageX, stageY] = text.get_transformed_position();
    return {x: stageX + Math.min(x1, x2), y: stageY + y1, width: Math.abs(x2 - x1), height: lineHeight};
}

/** Character position (code points) under a stage point, or null when outside the text. */
export function positionAt(label: St.Label, stageX: number, stageY: number): number | null {
    const text = label.clutter_text;
    const [inside, x, y] = text.transform_stage_point(stageX, stageY);
    if (!inside || x < 0 || y < 0 || x > text.width || y > text.height)
        return null;
    const position = text.coords_to_position(x, y);
    return position >= 0 ? position : null;
}

/** CSS font size of the label, e.g. `11pt` or `14px`, scaled by `factor`. */
export function scaledFontSize(label: St.Label, factor: number): string {
    const font: Pango.FontDescription = label.get_theme_node().get_font();
    const size = (font.get_size() / Pango.SCALE) * factor;
    return font.get_size_is_absolute() ? `${size.toFixed(1)}px` : `${size.toFixed(1)}pt`;
}

/** CSS `rgba()` of the label's foreground, so ruby follows light and dark themes. */
export function foregroundCss(label: St.Label): string {
    const color = label.get_theme_node().get_foreground_color();
    return `rgba(${color.red},${color.green},${color.blue},${(color.alpha / 255).toFixed(3)})`;
}

// Style classes of the Shell 46 widgets that may grow to make room for furigana.
const ROOM_HOST_CLASSES = ['quick-toggle', 'popup-menu-item'];

function isLayoutHost(actor: Clutter.Actor): actor is St.Widget {
    return actor instanceof St.Button ||
        (actor instanceof St.Widget && actor.has_style_class_name('popup-menu-item'));
}

/** The widget that frames the label: a button (toggle, panel button) or a menu item. */
export function layoutHost(label: St.Label): St.Widget | null {
    for (let actor = label.get_parent(); actor; actor = actor.get_parent()) {
        if (isLayoutHost(actor))
            return actor;
    }
    return null;
}

/** True when the host may gain top padding (menus grow; the top bar and app grid do not). */
export function canHostRoom(host: St.Widget | null): boolean {
    return host !== null && ROOM_HOST_CLASSES.some(name => host.has_style_class_name(name));
}

function stageBottom(actor: Clutter.Actor): number {
    const [, y] = actor.get_transformed_position();
    return y + actor.get_transformed_size()[1];
}

/**
 * Free space above the label: up to the bottom of the previous visible sibling (e.g. a
 * toggle title above its subtitle), else up to the top of its layout host.
 */
export function spaceAbove(label: St.Label): number {
    const [, labelTop] = label.clutter_text.get_transformed_position();
    let limit = -Infinity;
    const previous = label.get_previous_sibling();
    if (previous?.visible && previous.mapped)
        limit = Math.max(limit, stageBottom(previous));
    const host = layoutHost(label);
    if (host)
        limit = Math.max(limit, host.get_transformed_position()[1]);
    return Number.isFinite(limit) ? Math.max(0, labelTop - limit) : 0;
}
