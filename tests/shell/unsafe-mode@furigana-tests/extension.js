// TEST ONLY — used inside a throwaway headless Shell by tests/shell/run-acceptance.sh.
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

export default class UnsafeModeForTests extends Extension {
    enable() {
        global.context.unsafe_mode = true;
    }

    disable() {
        global.context.unsafe_mode = false;
    }
}
