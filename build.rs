/*  cinit: process initialisation program for containers
 *  Copyright (C) 2019 The cinit developers
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
extern crate vergen;

use vergen::generate_cargo_keys;
use vergen::ConstantsFlags;

fn main() {
    println!("cargo:rustc-link-search=/usr/lib");
    println!("cargo:rustc-link-lib=static=cap");

    let mut flags = ConstantsFlags::all();
    flags.toggle(ConstantsFlags::REBUILD_ON_HEAD_CHANGE);

    generate_cargo_keys(flags).expect("vergen could not generate flags");
}
