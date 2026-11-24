// Evaluated inside the headless GNOME Shell by run-acceptance.sh through
// org.gnome.Shell.Eval, which runs `JSON.stringify(await eval(code))` in the scope of
// shellDBus.js (so `Main` is in scope). Defines `globalThis.furiganaProbe`; every probe
// returns a plain value that Eval serialises for the script to assert on.
/* global global, Main, imports */
(() => {
    const {Clutter, St} = imports.gi;
    const seat = Clutter.get_default_backend().get_default_seat();
    const pointer = seat.create_virtual_device(Clutter.InputDeviceType.POINTER_DEVICE);
    const KANJI = /[\u4e00-\u9fff]/u;

    const quickSettingsMenu = () => Main.panel.statusArea.quickSettings.menu;

    function descendants(root, predicate) {
        const found = [];
        const pending = [root];
        for (let actor = pending.pop(); actor; actor = pending.pop()) {
            if (predicate(actor))
                found.push(actor);
            pending.push(...actor.get_children());
        }
        return found;
    }

    const rubyActors = () => descendants(Main.layoutManager.uiGroup,
        actor => actor instanceof St.Label && actor.has_style_class_name('furigana-ruby'));
    const tooltip = () => descendants(Main.layoutManager.uiGroup,
        actor => actor instanceof St.BoxLayout && actor.has_style_class_name('furigana-tooltip'))[0] ?? null;
    const kanjiLabels = () => descendants(quickSettingsMenu().box,
        actor => actor instanceof St.Label && actor.mapped && KANJI.test(actor.text));

    function movePointer(x, y) {
        pointer.notify_absolute_motion(global.get_current_time() * 1000, x, y);
    }

    /** Center of the first kanji character of the label whose text includes `needle`. */
    function kanjiPoint(needle) {
        const label = kanjiLabels().find(candidate => candidate.text.includes(needle));
        if (!label)
            return null;
        const index = [...label.text].findIndex(character => KANJI.test(character));
        const text = label.clutter_text;
        const [, x1, y1, lineHeight] = text.position_to_coords(index);
        const [, x2] = text.position_to_coords(index + 1);
        const [stageX, stageY] = text.get_transformed_position();
        return {label: label.text, x: stageX + (x1 + x2) / 2, y: stageY + y1 + lineHeight / 2};
    }

    globalThis.furiganaProbe = {
        openQuickSettings: () => { quickSettingsMenu().open(false); return 'opened'; },
        closeQuickSettings: () => { quickSettingsMenu().close(false); return 'closed'; },
        state: () => ({
            menuOpen: quickSettingsMenu().isOpen,
            labels: kanjiLabels().map(label => label.text),
            panelLabels: descendants(Main.panel, actor => actor instanceof St.Label && actor.mapped &&
                KANJI.test(actor.text)).map(label => label.text),
            ruby: rubyActors().filter(actor => actor.mapped && actor.get_paint_opacity() > 0).map(actor => actor.text),
            rubyTotal: rubyActors().length,
            tooltipVisible: tooltip()?.visible === true && tooltip().opacity > 0,
            tooltipText: tooltip() ? tooltip().get_children().map(label => label.text) : [],
        }),
        hover: needle => {
            const point = kanjiPoint(needle);
            if (point)
                movePointer(point.x, point.y);
            return point;
        },
        nudge: (dx, dy) => {
            const [x, y] = global.get_pointer();
            movePointer(x + dx, y + dy);
            return {x: x + dx, y: y + dy};
        },
        leave: () => { movePointer(5, global.stage.height - 5); return 'left'; },
    };
    return 'probe-installed';
})();
