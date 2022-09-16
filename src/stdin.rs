/*
 * Copyright (C) 2022   Steffen Nuessle
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use std::io;
use std::os::raw::c_int;
use std::os::unix::io::AsRawFd;

extern "C" {
    #[cfg(target_family = "unix")]
    fn isatty(fd: c_int) -> c_int;
}

pub trait Term {
    fn isatty(&self) -> bool;
}

impl Term for io::Stdin {
    #[cfg(target_family = "unix")]
    fn isatty(&self) -> bool {
        unsafe { isatty(self.as_raw_fd()) == 1 }
    }
}
