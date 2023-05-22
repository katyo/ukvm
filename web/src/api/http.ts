import { LedId, ButtonId, KeyboardKey, KeyboardLed, MouseButton, MousePointer, MouseWheel, InputApi, OutputApi, State } from './types.ts';

export { LedId, ButtonId };

const root = ""; //process.env.API_ROOT

export function api(handler: OutputApi): InputApi {
    const socket = new WebSocket(`${root}/socket`);

    socket.onopen = function() {
        handler.connection(true);
    };
    socket.onmessage = function(evt) {
        if (typeof evt.data == 'string') {
            const msg = JSON.parse(evt.data);
            switch (msg.$) {
                case 's': handler.state(msg); break;
                case 'l': handler.led(msg.l, msg.s); break;
                case 'b': handler.button(msg.b, msg.s); break;
                case 'k': handler.keyboardKey(msg.k, msg.s); break;
                case 'i': handler.keyboardLed(msg.l, msg.s); break;
                case 'm': handler.mouseButton(msg.b, msg.s); break;
                case 'p': handler.mousePointer(msg.p); break;
                case 'w': handler.mousePointer(msg.w); break;
            }
        }
    };
    socket.onclose = function() {
        handler.connection(false);
    };

    function send(msg: { $: string, [key: string]: string | number | boolean | [number, number] | State }) {
        socket.send(JSON.stringify(msg));
    }

    return {
        button(id: ButtonId, state: boolean) {
            send({
                $: 'b',
                b: id,
                s: state,
            });
        },
        keyboardKey(key: KeyboardKey, state: boolean) {
            send({
                $: 'k',
                k: key,
                s: state,
            })
        },
        mouseButton(button: MouseButton, state: boolean) {
            send({
                $: 'm',
                b: button,
                s: state,
            })
        },
        mousePointer(pointer: MousePointer) {
            send({
                $: 'p',
                p: pointer,
            })
        },
        mouseWheel(wheel: MouseWheel) {
            send({
                $: 'w',
                w: wheel,
            })
        },
    };
}
