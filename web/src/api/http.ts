import { LedId, ButtonId, Capabilities, OnLedState, OnButtonState } from './types.ts';

export { LedId, ButtonId };
export type { Capabilities, OnLedState, OnButtonState };

const root = ""; //process.env.API_ROOT

/// Get capabilities
export async function capabilities(): Promise<Capabilities> {
    const res = await fetch(`${root}/capabilities`);
    if (res.status != 200) {
        throw `Invalid status: ${res.status}`;
    }
    return await res.json();
}

/// Get LED state
export async function led_state(id: LedId): Promise<boolean> {
    const res = await fetch(`${root}/leds/${id}/state`, {
        method: 'POST',
    });
    if (res.status != 200) {
        throw `Invalid status: ${res.status}`;
    }
    return await res.json();
}

/// Get button state
export async function button_state(id: ButtonId): Promise<boolean> {
    const res = await fetch(`${root}/buttons/${id}/state`, {
        method: 'POST',
    });
    if (res.status != 200) {
        throw `Invalid status: ${res.status}`;
    }
    return await res.json();
}

/// Set button state
export async function set_button_state(id: ButtonId, state: boolean) {
    const res = await fetch(`${root}/buttons/${id}/state`, {
        method: 'PUT',
        headers: {
            'Content-Type': 'application/json;charset=utf-8'
        },
        body: JSON.stringify(state)
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
    ButtonRelease = "button-release",
}

export interface EventHandlers {
    led_status?: (id: LedId, status: boolean) => void;
    button_press?: (id: ButtonId) => void;
}

let event_source: EventSource | undefined;

const led_state_handlers: OnLedState[] = [];
const button_state_handlers: OnButtonState[] = [];

function make_event_source() {
    if (!event_source) {
        event_source = new EventSource(`${root}/events`);

        event_source.addEventListener(EventId.LedOn, (event: Event) => {
            for (const handler of led_state_handlers) {
                handler(event.data, true);
            }
        });

        event_source.addEventListener(EventId.LedOff, (event: Event) => {
            for (const handler of led_state_handlers) {
                handler(event.data, false);
            }
        });

        event_source.addEventListener(EventId.ButtonPress, (event: Event) => {
            for (const handler of button_state_handlers) {
                handler(event.data, true);
            }
        });

        event_source.addEventListener(EventId.ButtonRelease, (event: Event) => {
            for (const handler of button_state_handlers) {
                handler(event.data, false);
            }
        });
    }
}

/// Add LED state handler
export function on_led_state(handler: OnLedState) {
    make_event_source();

    led_state_handlers.push(handler);
}

/// Add button press handler
export function on_button_state(handler: OnButtonState) {
    make_event_source();

    button_state_handlers.push(handler);
}
