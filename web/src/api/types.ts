import type { Key as KeyboardKey, Led as KeyboardLed, Button as MouseButton, Pointer as MousePointer, Wheel as MouseWheel } from './hid.ts';

export type { KeyboardKey, KeyboardLed, MouseButton, MousePointer, MouseWheel };

/// LED identifier
export const enum LedId {
    Power = "power",
    Disk = "disk",
    Ether = "ether",
}

/// Button identifier
export const enum ButtonId {
    Power = "power",
    Reset = "reset",
    Clear = "clear",
}

/// Keyboard state key
export const enum KeyboardStateKey {
    /// Pressed keys
    Keys = "k",
    /// Lit LEDs
    Leds = "l",
}

/// Keyboard state
export interface KeyboardState {
    [KeyboardStateKey.Leds]: KeyboardLed[],
    [KeyboardStateKey.Keys]: KeyboardKey[],
}

/// Mouse state key
export const enum MouseStateKey {
    /// Pressed buttons
    Buttons = "b",
    /// Pointer position
    Pointer = "p",
    /// Wheel position
    Wheel = "w",
}

/// Mouse state
export interface MouseState {
    [MouseStateKey.Buttons]: MouseButton[],
    [MouseStateKey.Pointer]: MousePointer,
    [MouseStateKey.Wheel]: MouseWheel,
}

/// State key
export const enum StateKey {
    Leds = "l",
    Buttons = "b",
    Keyboard = "k",
    Mouse = "m",
}

/// State
export interface State {
    [StateKey.Leds]: { [id: string]: boolean },
    [StateKey.Buttons]: { [id: string]: boolean },
    [StateKey.Keyboard]?: KeyboardState,
    [StateKey.Mouse]?: MouseState,
}

/// Output messages handler
export interface OutputApi {
    /// Connection state changed
    connection(state: boolean): void;

    /// Initial state
    state(state: State): void;

    /// LED state changed
    led(id: LedId, state: boolean): void;

    /// Button state changed
    button(id: ButtonId, state: boolean): void;

    /// Keyboard key state changed
    keyboardKey(key: KeyboardKey, state: boolean): void;

    /// Keyboard LED state changed
    keyboardLed(led: KeyboardLed, state: boolean): void;

    /// Mouse button state changed
    mouseButton(button: MouseButton, state: boolean): void;

    /// Mouse pointer value changed
    mousePointer(pointer: MousePointer): void;

    /// Mouse wheel value changed
    mouseWheel(wheel: MouseWheel): void;
}

/// Input messages handler
export interface InputApi {
    /// Change button state
    button(id: ButtonId, state: boolean): void;

    /// Change keyboard key state
    keyboardKey(key: KeyboardKey, state: boolean): void;

    /// Change mouse button state
    mouseButton(button: MouseButton, state: boolean): void;

    /// Change pointer value
    mousePointer(pointer: MousePointer): void;

    /// Change wheel value
    mouseWheel(wheel: MouseWheel): void;
}
