// SPDX-License-Identifier: GPL-3.0-or-later
// Release builds have no console window; debug builds keep one.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    ampello_lib::run()
}
