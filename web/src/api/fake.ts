import { LedId, ButtonId, Capabilities, OnLedStatus, OnButtonPress } from './types.ts';

export { LedId, ButtonId };
export type { Capabilities, OnLedStatus, OnButtonPress };

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

const led_status_handlers: OnLedStatus[] = [];

/// Add LED status handler
export function on_led_status(handler: OnLedStatus) {
    led_status_handlers.push(handler);
}

function handle_led_status(id: LedId, status: boolean) {
    for (const handler of led_status_handlers) {
        handler(id, status);
    }
}

const button_press_handlers: OnButtonPress[] = [];

function handle_button_press(id: ButtonId) {
    for (const handler of button_press_handlers) {
        handler(id);
    }
}

/// Add button press handler
export function on_button_press(handler: OnButtonPress) {
    button_press_handlers.push(handler);
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

let disk_led_status = false;
const disk_led_activity = activity<LedActivity>(led_activity_off, config => {
    if (config) {
        disk_led_status = !disk_led_status;
    } else {
        disk_led_status = false;
    }
    handle_led_status(LedId.Disk, disk_led_status);
    if (config) {
        return random_delay(config[disk_led_status ? 'on' : 'off'] as Delay);
    }
    return 0;
});

let ether_led_status = false;
const ether_led_activity = activity<LedActivity>(led_activity_off, config => {
    if (config) {
        ether_led_status = !ether_led_status;
    } else {
        ether_led_status = false;
    }
    handle_led_status(LedId.Ether, ether_led_status);
    if (config) {
        return random_delay(config[ether_led_status ? 'on' : 'off'] as Delay);
    }
    return 0;
});

let power_state = false;

async function power_on() {
    power_state = true;
    handle_led_status(LedId.Power, power_state);
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
    handle_led_status(LedId.Power, power_state);
}

/// Press button
export async function button_press(id: ButtonId) {
    handle_button_press(id);
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
