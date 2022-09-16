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
    buf: String,
}

impl JsonSerializer {
    pub fn new() -> Self {
        Self {
            buf: String::with_capacity(4096),
        }
    }

    pub fn get_json_str(&self) -> &str {
        self.buf.as_str()
    }

    pub fn write_vec(&mut self, vec: &Vec<Dependency>) {
        self.buf.push('[');

        for (i, dep) in vec.iter().enumerate() {
            if i != 0 {
                self.buf.push(',');
            }

            self.buf.push('{');

            self.buf.push_str("\"target\":");
            self.write_str(dep.target);
            self.buf.push_str(",\"prerequisites\":[");

            for (j, val) in dep.prerequisites.iter().enumerate() {
                if j != 0 {
                    self.buf.push(',');
                }

                self.write_str(val);
            }

            self.buf.push(']');
            self.buf.push('}');
        }

        self.buf.push(']');
    }

    fn write_str(&mut self, slice: &str) {
        self.buf.push('\"');

        for c in slice.chars() {
            match c {
                '\\' => self.buf.push_str("\\\\"),
                '"' => self.buf.push_str("\\\""),
                _ => self.buf.push(c),
            };
        }

        self.buf.push('\"');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_vec_empty() {
        let vec: Vec<Dependency> = Vec::new();

        let mut serializer = JsonSerializer::new();
        serializer.write_vec(&vec);

        assert_eq!("[]", serializer.buf);
    }

    #[test]
    fn write_vec_one() {
        let dep = Dependency {
            target: "a",
            prerequisites: Vec::from(["b"]),
        };

        let vec = Vec::from([dep]);

        let mut serializer = JsonSerializer::new();
        serializer.write_vec(&vec);

        assert_eq!(
            "[{\"target\":\"a\",\"prerequisites\":[\"b\"]}]",
            serializer.buf
        );
    }

    #[test]
    fn write_vec_two() {
        let dep = Dependency {
            target: "a",
            prerequisites: Vec::from(["b", "c"]),
        };

        let vec = Vec::from([dep]);

        let mut serializer = JsonSerializer::new();
        serializer.write_vec(&vec);

        assert_eq!(
            "[{\"target\":\"a\",\"prerequisites\":[\"b\",\"c\"]}]",
            serializer.buf
        );
    }

    #[test]
    fn write_str_empty() {
        let mut serializer = JsonSerializer::new();
        serializer.write_str("");

        assert_eq!("\"\"", serializer.buf);
    }

    #[test]
    fn write_str_noescaping() {
        let mut serializer = JsonSerializer::new();
        serializer.write_str("ez");

        assert_eq!("\"ez\"", serializer.buf);
    }

    #[test]
    fn write_str_escaping() {
        let mut serializer = JsonSerializer::new();
        serializer.write_str("\"e\\z\"");

        assert_eq!("\"\\\"e\\\\z\\\"\"", serializer.buf);
    }
}
