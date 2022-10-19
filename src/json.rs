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

use crate::dependency::Dependency;

pub struct JsonSerializer {
    buf: Vec<u8>,
}

impl JsonSerializer {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn get_json(&self) -> &[u8] {
        self.buf.as_slice()
    }

    pub fn write_vec(&mut self, vec: &Vec<Dependency>) {
        self.buf.reserve(4096 * vec.len());

        self.buf.push(b'[');

        for (i, dep) in vec.iter().enumerate() {
            if i != 0 {
                self.buf.push(b',');
            }

            self.buf.push(b'{');

            self.buf.extend_from_slice(b"\"target\":");
            self.write_str(dep.target);
            self.buf.extend_from_slice(b",\"prerequisites\":[");

            for (j, val) in dep.prerequisites.iter().enumerate() {
                if j != 0 {
                    self.buf.push(b',');
                }

                self.write_str(val);
            }

            self.buf.push(b']');
            self.buf.push(b'}');
        }

        self.buf.push(b']');
    }

    fn write_str(&mut self, data: &str) {
        let bytes = data.as_bytes();
        let mut i = 0;

        self.buf.push(b'\"');

        for (j, _) in bytes.iter().enumerate() {
            match bytes[j] {
                b'\\' | b'"' => {
                    self.buf.extend_from_slice(&bytes[i..j]);
                    self.buf.push(b'\\');
                    self.buf.push(bytes[j]);

                    i = j + 1;
                }
                _ => {}
            }
        }

        self.buf.extend_from_slice(&bytes[i..]);

        self.buf.push(b'\"');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_vec_001() {
        let vec: Vec<Dependency> = Vec::new();

        let mut serializer = JsonSerializer::new();
        serializer.write_vec(&vec);

        assert_eq!(b"[]", serializer.buf.as_slice());
    }

    #[test]
    fn write_vec_002() {
        let dep = Dependency {
            target: "a",
            prerequisites: Vec::from(["b"]),
        };

        let vec = Vec::from([dep]);

        let mut serializer = JsonSerializer::new();
        serializer.write_vec(&vec);

        assert_eq!(
            b"[{\"target\":\"a\",\"prerequisites\":[\"b\"]}]",
            serializer.buf.as_slice()
        );
    }

    #[test]
    fn write_vec_003() {
        let dep = Dependency {
            target: "a",
            prerequisites: Vec::from(["b", "c"]),
        };

        let vec = Vec::from([dep]);

        let mut serializer = JsonSerializer::new();
        serializer.write_vec(&vec);

        assert_eq!(
            b"[{\"target\":\"a\",\"prerequisites\":[\"b\",\"c\"]}]",
            serializer.buf.as_slice()
        );
    }

    #[test]
    fn write_str_001() {
        let mut serializer = JsonSerializer::new();
        serializer.write_str("");

        assert_eq!(b"\"\"", serializer.buf.as_slice());
    }

    #[test]
    fn write_str_002() {
        let mut serializer = JsonSerializer::new();
        serializer.write_str("ez");

        assert_eq!(b"\"ez\"", serializer.buf.as_slice());
    }

    #[test]
    fn write_str_003() {
        let mut serializer = JsonSerializer::new();
        serializer.write_str("\"e\\z\"");

        unsafe {
        assert_eq!("\"\\\"e\\\\z\\\"\"", std::str::from_utf8_unchecked(serializer.buf.as_slice()));
        }
        assert_eq!(b"\"\\\"e\\\\z\\\"\"", serializer.buf.as_slice());
    }
}
