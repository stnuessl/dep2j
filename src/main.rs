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

mod args;
mod dependency;
mod json;
mod stdin;

use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::process::exit;

use crate::dependency::parser::DependencyParser;
use crate::json::JsonSerializer;
use crate::stdin::Term;

fn help() {
    println!(
        "\
USAGE: dep2j [options] <file1> [... <fileN>]

OPTIONS:

    -o <file>       Write generated output to <file>.
    --              Intepret the remaining arguments as input files.
                    This is useful if a file name starts with a '-'.
Generic Options:

    --help, -h      Print this help message and exit.
    --version       Print version information and exit.
"
    );
}

fn version() {
    let version = env!("CARGO_PKG_VERSION");

    println!("dep2j {version}");
}

fn main() {
    let argv = env::args();
    let argc = argv.len();
    let args = args::parse(argv);

    let mut stdin = io::stdin();
    let isatty = stdin.isatty();

    if args.help || (isatty && argc < 2) {
        help();
        exit(0)
    }

    if args.version {
        version();
        exit(0)
    }

    if isatty && args.input.is_empty() {
        eprintln!("error: no input data available");
        exit(1);
    }

    let mut data = Vec::with_capacity(4096 * args.input.len());

    for path in &args.input {
        let mut file = match File::open(path) {
            Ok(val) => val,
            Err(err) => {
                eprintln!("error: failed to open \"{path}\": {err}");
                exit(1);
            }
        };

        match file.metadata() {
            Ok(item) => data.reserve(item.len() as usize),
            Err(_) => {}
        }

        match file.read_to_end(&mut data) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("error: failed to read file \"{path}\": {}", err);
                exit(1);
            }
        }
    }

    if !isatty {
        data.reserve(4096);

        match stdin.read_to_end(&mut data) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("error: failed to read stdin: {}", err);
                exit(1);
            }
        }
    }

    let mut parser = DependencyParser::new();
    let deps = parser.parse(data);

    let mut serializer = JsonSerializer::new();
    serializer.write_vec(&deps);

    let json = serializer.get_json_str();

    if args.output.is_empty() {
        println!("{json}");
        return;
    }

    File::create(&args.output)
        .and_then(|mut file| {
            file.write_all(json.as_bytes())
        })
        .unwrap_or_else(|err| {
            eprintln!("error: failed to write to \"{}\": {err}", args.output);
            exit(1);
        });
}
