import { LedId, ButtonId, Capabilities, OnLedState, OnButtonState } from './types.ts';

export { LedId, ButtonId };
export type { Capabilities, OnLedState, OnButtonState };

function random_range(min: number, max: number): number {
    return Math.random() * (max - min) + min;
}

type Delay = number | [number, number];

function random_delay(delay: Delay): number {
    return typeof delay == 'number' ? delay : random_range(delay[0], delay[1]);
}

function sleep(delay: Delay = 100): Promise<void> {
    return new Promise(resolve => {
        setTimeout(() => { resolve(); }, random_delay(delay));
    });
}

/// Get capabilities
export async function capabilities(): Promise<Capabilities> {
    await sleep([100, 500]);
    return {
        buttons: [ButtonId.Power, ButtonId.Reset, ButtonId.Clear],
        leds: [LedId.Power, LedId.Disk, LedId.Ether],
    };
}

function activity<Config>(config: Config, on: (config: Config) => number): (config: Config) => void {
    let timer: number | undefined;

    function restart() {
        const delay = on(config);
        if (delay != 0 && timer === undefined) {
            timer = setTimeout(() => {
                timer = undefined;
                restart();
            }, delay);
        }
        if (delay == 0 && timer !== undefined) {
            clearTimeout(timer);
            timer = undefined;
        }
    }

    return (config_: Config) => {
        config = config_;
        restart();
    };
}

const led_state_handlers: OnLedState[] = [];

/// Add LED status handler
export function on_led_state(handler: OnLedState) {
    led_state_handlers.push(handler);
}

function handle_led_state(id: LedId, status: boolean) {
    for (const handler of led_state_handlers) {
        handler(id, status);
    }
}

const button_states: { [id: string]: boolean } = {
    [ButtonId.Power]: false,
    [ButtonId.Reset]: false,
    [ButtonId.Clear]: false,
};
const button_state_handlers: OnButtonState[] = [];

function handle_button_state(id: ButtonId, state: boolean) {
    button_states[id] = state;
    for (const handler of button_state_handlers) {
        handler(id, state);
    }
}

/// Add button state handler
export function on_button_state(handler: OnButtonState) {
    button_state_handlers.push(handler);
}

interface LedActivityRandom {
    on: Delay,
    off: Delay,
}

type LedActivity = LedActivityRandom | false;

const led_activity_off: LedActivity = false;
const led_activity_high: LedActivity = {
    on: 100,
    off: [100, 200],
};
const led_activity_idle: LedActivity = {
    on: 100,
    off: [1000, 2000],
};

let disk_led_state = false;
const disk_led_activity = activity<LedActivity>(led_activity_off, config => {
    if (config) {
        disk_led_state = !disk_led_state;
    } else {
        disk_led_state = false;
    }
    handle_led_state(LedId.Disk, disk_led_state);
    if (config) {
        return random_delay(config[disk_led_state ? 'on' : 'off'] as Delay);
    }
    return 0;
});

let ether_led_state = false;
const ether_led_activity = activity<LedActivity>(led_activity_off, config => {
    if (config) {
        ether_led_state = !ether_led_state;
    } else {
        ether_led_state = false;
    }
    handle_led_state(LedId.Ether, ether_led_state);
    if (config) {
        return random_delay(config[ether_led_state ? 'on' : 'off'] as Delay);
    }
    return 0;
});

let power_state = false;

async function power_on() {
    power_state = true;
    handle_led_state(LedId.Power, power_state);
    disk_led_activity(led_activity_high);
    ether_led_activity(led_activity_high);
    await sleep([1500, 2000]);
    disk_led_activity(led_activity_idle);
    ether_led_activity(led_activity_idle);
}

async function power_off() {
    disk_led_activity(led_activity_high);
    ether_led_activity(led_activity_high);
    await sleep([500, 700]);
    disk_led_activity(led_activity_off);
    ether_led_activity(led_activity_off);
    power_state = false;
    handle_led_state(LedId.Power, power_state);
}

/// Get LED state
export async function led_state(id: LedId): Promise<boolean> {
    return id == LedId.Power ? power_state : id == LedId.Disk ? disk_led_state : ether_led_state;
}

/// Get button state
export async function button_state(id: ButtonId): Promise<boolean> {
    return button_states[id];
}

/// Set button state
export async function set_button_state(id: ButtonId, state: boolean) {
    handle_button_state(id, state);
    switch (id) {
        case ButtonId.Power:
            power_state = !power_state;
            if (power_state) {
                await power_on();
            } else {
                await power_off();
            }
            break;
        case ButtonId.Reset:
            if (power_state) {
                await power_off();
                await power_on();
            }
            break;
    }
}
