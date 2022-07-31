import { LedId, ButtonId, Capabilities, OnLedStatus, OnButtonPress } from './types.ts';

export { LedId, ButtonId };
export type { Capabilities, OnLedStatus, OnButtonPress };

const root = ""; //process.env.API_ROOT

/// Get capabilities
export async function capabilities(): Promise<Capabilities> {
    const res = await fetch(`${root}/capabilities`);
    if (res.status != 200) {
        throw `Invalid status: ${res.status}`;
    }
    return await res.json();
}

/// Press button
export async function button_press(id: ButtonId) {
    const res = await fetch(`${root}/buttons/${id}/press`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json;charset=utf-8'
        }
    });
    if (res.status != 200) {
        throw `Invalid status: ${res.status}`;
    }
}

/// Event identifier
const enum EventId {
    LedOn = "led-on",
    LedOff = "led-off",
    ButtonPress = "button-press",
}

export interface EventHandlers {
    led_status?: (id: LedId, status: boolean) => void;
    button_press?: (id: ButtonId) => void;
}

let event_source: EventSource | undefined;

const led_status_handlers: OnLedStatus[] = [];
const button_press_handlers: OnButtonPress[] = [];

function make_event_source() {
    if (!event_source) {
        event_source = new EventSource(`${root}/events`);

        event_source.addEventListener(EventId.LedOn, (event: Event) => {
            for (const handler of led_status_handlers) {
                handler(event.data, true);
            }
        });

        event_source.addEventListener(EventId.LedOff, (event: Event) => {
            for (const handler of led_status_handlers) {
                handler(event.data, false);
            }
        });

        event_source.addEventListener(EventId.ButtonPress, (event: Event) => {
            for (const handler of button_press_handlers) {
                handler(event.data);
            }
        });
    }
}

/// Add LED status handler
export function on_led_status(handler: OnLedStatus) {
    make_event_source();

    led_status_handlers.push(handler);
}

/// Add button press handler
export function on_button_press(handler: OnButtonPress) {
    make_event_source();

    button_press_handlers.push(handler);
}
