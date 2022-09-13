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

/// Capabilities
export interface Capabilities {
    buttons: ButtonId[],
    leds: LedId[],
}

export interface OnLedState {
    (id: LedId, state: boolean): void;
}

export interface OnButtonState {
    (id: ButtonId, state: boolean): void;
}
