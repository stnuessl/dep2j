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

use std::hash::Hasher;
use std::mem;

/*
 * This application does not need a cryptographic secure hash function.
 * All we care about is high performance and good collision avoidance.
 */
pub struct PathHasher {
    hash: usize,
}

impl Default for PathHasher {
    #[inline]
    fn default() -> Self {
        Self { hash: 1 }
    }
}

impl PathHasher {
    fn add(&mut self, num: usize) {
        const VALUE: u32 = (4 * mem::size_of::<usize>()) as u32;

        self.hash = self.hash.rotate_left(VALUE);
        self.hash ^= num;
    }
}

impl Hasher for PathHasher {
    fn write(&mut self, bytes: &[u8]) {
        let mut view = bytes;
        let chunk_size = mem::size_of::<usize>();

        while view.len() >= chunk_size {
            let data =
                unsafe { view[..chunk_size].try_into().unwrap_unchecked() };

            self.add(usize::from_ne_bytes(data));
            view = &view[chunk_size..];
        }

        if chunk_size > 4 && view.len() >= 4 {
            let data = unsafe { view[..4].try_into().unwrap_unchecked() };

            self.add(u32::from_ne_bytes(data) as usize);
            view = &view[4..];
        }

        if chunk_size > 2 && view.len() >= 2 {
            let data = unsafe { view[..2].try_into().unwrap_unchecked() };

            self.add(u16::from_ne_bytes(data) as usize);
            view = &view[2..];
        }

        if chunk_size > 1 && !view.is_empty() {
            let data = unsafe { view.try_into().unwrap_unchecked() };

            self.add(u8::from_ne_bytes(data) as usize);
        }
    }

    fn finish(&self) -> u64 {
        self.hash as u64
    }
}
