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

use std::process::exit;

#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    pub input: Vec<String>,
    pub output: String,
    pub help: bool,
    pub version: bool,
}

impl Args {
    pub fn new() -> Self {
        Self {
            input: Vec::new(),
            output: String::new(),
            help: false,
            version: false,
        }
    }
}

#[must_use]
pub fn parse<I: Iterator<Item = String> + ExactSizeIterator>(
    mut argv: I,
) -> Args {
    let mut result = Args::new();
    let mut dash_dash = false;

    /* Skip the name of the program */
    argv.next();

    while let Some(arg) = argv.next() {
        if !arg.starts_with('-') || dash_dash {
            if result.input.capacity() == 0 {
                result.input.reserve(argv.len());
            }

            result.input.push(arg);
        } else if arg == "--" {
            dash_dash = true;
        } else if arg == "--help" || arg == "-h" {
            result.help = true;
        } else if arg == "--version" {
            result.version = true;
        } else {
            let value = argv.next();

            if value.is_none() {
                eprintln!("error: missing argument for \"{arg}\"");
                exit(1);
            }

            if arg == "-o" {
                result.output = value.unwrap();
            } else {
                eprintln!("error: unknown argument \"{arg}\"");
                exit(1);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn do_parse(vec: Vec<&str>) -> Args {
        let args = vec.iter().map(|x| x.to_string()).into_iter();

        parse(args)
    }

    /**
     * parse()
     *
     * Verify that the function correctly handles empty input.
     */
    #[test]
    fn parse_001() {
        let vec = Vec::new();

        assert_eq!(Args::new(), do_parse(vec));
    }

    /**
     * parse()
     *
     * Verify that the function correctly handles input with just the
     * program name.
     */
    #[test]
    fn parse_002() {
        let vec = Vec::from(["dep2j"]);

        assert_eq!(Args::new(), do_parse(vec));
    }

    /**
     * parse()
     *
     * Verify that the function correctly handles arguments specifiying input
     * and output files.
     */
    #[test]
    fn parse_003() {
        let vec = Vec::from(["dep2j", "-o", "output.json", "input.d"]);

        let args = do_parse(vec);

        assert_eq!(false, args.help);
        assert_eq!(false, args.version);
        assert_eq!("output.json", args.output);
        assert_eq!(1, args.input.len());
        assert_eq!("input.d", args.input[0]);
    }

    /**
     * parse()
     *
     * Verify that the function correctly handles arguments with '--help'
     * and an input file.
     */
    #[test]
    fn parse_004() {
        let vec = Vec::from(["dep2j", "--help", "input.d"]);

        let args = do_parse(vec);

        assert_eq!(true, args.help);
        assert_eq!(1, args.input.len());
        assert_eq!("input.d", args.input[0]);
    }

    /**
     * parse()
     *
     * Verify that the function correctly handles input with the "dash dash"
     * argument.
     */
    #[test]
    fn parse_005() {
        let vec = Vec::from([
            "dep2j",
            "--version",
            "-o",
            "output.json",
            "--",
            "-h",
            "-input.d",
        ]);

        let args = do_parse(vec);

        assert_eq!(true, args.version);
        assert_eq!("output.json", args.output);
        assert_eq!(2, args.input.len());
        assert_eq!("-h", args.input[0]);
        assert_eq!("-input.d", args.input[1]);
    }
}
