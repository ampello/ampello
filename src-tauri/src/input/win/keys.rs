// SPDX-License-Identifier: GPL-3.0-or-later
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

pub fn is_reset_key(vk: u16) -> bool {
    matches!(
        vk,
        VK_LEFT
            | VK_RIGHT
            | VK_UP
            | VK_DOWN
            | VK_HOME
            | VK_END
            | VK_PRIOR
            | VK_NEXT
            | VK_DELETE
            | VK_INSERT
            | VK_ESCAPE
    ) || (VK_F1..=VK_F24).contains(&vk)
}

pub fn is_modifier(vk: u16) -> bool {
    matches!(
        vk,
        VK_SHIFT
            | VK_CONTROL
            | VK_MENU
            | VK_LSHIFT
            | VK_RSHIFT
            | VK_LCONTROL
            | VK_RCONTROL
            | VK_LMENU
            | VK_RMENU
            | VK_LWIN
            | VK_RWIN
            | VK_CAPITAL
            | VK_NUMLOCK
            | VK_SCROLL
    )
}

const DOWN: u8 = 0x80;
const TOGGLED: u8 = 0x01;

pub fn update_state(state: &mut [u8; 256], vk: u16, down: bool) {
    let index = vk as usize;
    if index >= state.len() {
        return;
    }

    if down {
        state[index] |= DOWN;
    } else {
        state[index] &= !DOWN;
    }

    if down && matches!(vk, VK_CAPITAL | VK_NUMLOCK | VK_SCROLL) {
        state[index] ^= TOGGLED;
    }

    sync_pair(state, VK_SHIFT, VK_LSHIFT, VK_RSHIFT);
    sync_pair(state, VK_CONTROL, VK_LCONTROL, VK_RCONTROL);
    sync_pair(state, VK_MENU, VK_LMENU, VK_RMENU);
}

fn sync_pair(state: &mut [u8; 256], generic: u16, left: u16, right: u16) {
    let held = state[left as usize] & DOWN != 0 || state[right as usize] & DOWN != 0;
    if held {
        state[generic as usize] |= DOWN;
    } else {
        state[generic as usize] &= !DOWN;
    }
}

pub fn is_down(state: &[u8; 256], vk: u16) -> bool {
    state[vk as usize] & DOWN != 0
}

pub fn is_shortcut(state: &[u8; 256]) -> bool {
    let ctrl = is_down(state, VK_CONTROL);
    let alt = is_down(state, VK_MENU);
    let win = is_down(state, VK_LWIN) || is_down(state, VK_RWIN);
    win || (ctrl && !alt)
}
