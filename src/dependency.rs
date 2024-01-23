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

use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::hash::BuildHasherDefault;
use std::process::exit;
use std::{cmp, mem, ptr, str};

use crate::hash::PathHasher;


#[derive(Debug, PartialEq, Eq)]
pub struct Dependency<'a> {
    pub target: &'a str,
    pub prerequisites: Vec<&'a str>,
}

impl<'a> Dependency<'a> {
    pub fn new(name: &'a str) -> Self {
        Self {
            target: name,
            prerequisites: Vec::with_capacity(32),
        }
    }
}

pub struct DependencyParser<'a> {
    data: Vec<u8>,
    deps: Vec<Dependency<'a>>,
}

impl<'a> DependencyParser<'a> {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            deps: Vec::new(),
        }
    }

    #[must_use]
    pub fn parse(&mut self, data: Vec<u8>) -> &Vec<Dependency> {
        self.data = data;

        if self.deps.capacity() == 0 {
            /*
             * The number of dependencies is correlating with the size of the
             * input. We just assume that a dependency occurs an specific
             * amount of characters to reflect this and get a rough estimate.
             */
            let estimate = cmp::max(1 + self.data.len() / 256, 16);
            self.deps.reserve(estimate);
        }

        self.deps.clear();
        self.parse_rules();
        self.merge_deps();

        &self.deps
    }

    fn parse_rules(&mut self) {
        unsafe {
            let mut ptr = self.data.as_ptr();
            let end = ptr.add(self.data.len());

            while ptr < end {
                match *ptr {
                    b'\n' | b' ' | b'\\' => {
                        ptr = ptr.add(1);
                    }
                    b'#' => {
                        ptr = util::skip_comment(ptr, end);
                        continue;
                    }
                    _ => {
                        ptr = self.parse_rule(ptr, end);
                        continue;
                    }
                }
            }
        }
    }

    fn merge_deps(&mut self) {
        type DependencyMap<'a> = HashMap<&'a str, usize>;
        type StrHashSet<'a> = HashSet<&'a str, BuildHasherDefault<PathHasher>>;
        type PrerequisiteMap<'a> = HashMap<&'a str, StrHashSet<'a>>;

        let len = self.deps.len();
        let deps = mem::replace(&mut self.deps, Vec::with_capacity(len));
        let mut deps_map: DependencyMap = HashMap::with_capacity(len);
        let mut prereq_map: PrerequisiteMap = HashMap::with_capacity(len);

        for dep in deps {
            match deps_map.entry(dep.target) {
                Entry::Occupied(entry) => {
                    let merged_dep = &mut self.deps[*entry.get()];
                    let set = prereq_map.get_mut(merged_dep.target).unwrap();

                    set.reserve(dep.prerequisites.len());

                    for &prereq in &dep.prerequisites {
                        if set.insert(prereq) {
                            merged_dep.prerequisites.push(prereq);
                        }
                    }
                }
                Entry::Vacant(entry) => {
                    let hasher = BuildHasherDefault::<PathHasher>::default();
                    let mut set = HashSet::with_hasher(hasher);

                    let capacity = 2 * dep.prerequisites.len();
                    set.reserve(capacity);

                    for &prereq in &dep.prerequisites {
                        set.insert(prereq);
                    }

                    entry.insert(self.deps.len());
                    prereq_map.insert(dep.target, set);
                    self.deps.push(dep);
                }
            };
        }
    }

    unsafe fn parse_rule(
        &mut self,
        begin: *const u8,
        end: *const u8,
    ) -> *const u8 {
        let mut ptr = begin;

        while ptr < end {
            match *ptr {
                b' ' | b'\n' => {
                    ptr = ptr.add(1);
                }
                b'#' => {
                    ptr = util::skip_comment(ptr, end);
                }
                _ => {
                    ptr = self.parse_targets(ptr, end);
                }
            }
        }

        ptr
    }

    unsafe fn parse_targets(
        &mut self,
        begin: *const u8,
        end: *const u8,
    ) -> *const u8 {
        let len = self.deps.len();
        let mut str_begin = begin;
        let mut ptr = begin;

        while ptr < end {
            match *ptr {
                b' ' | b'\t' => {
                    let prev = ptr.sub(1);
                    ptr = ptr.add(1);

                    if ptr == str_begin {
                        continue;
                    }

                    if str_begin.is_null() {
                        continue;
                    }

                    if *prev == b':' {
                        /*
                         * Given "a b : ", ensure that the final ':' is not
                         * evaluated as a target.
                         */
                        if prev != str_begin {
                            self.emit_target(str_begin, prev);
                        }

                        return self.parse_prerequisites(len, ptr, end);
                    }

                    if *prev != b'\\' {
                        self.emit_target(str_begin, prev.add(1));
                        str_begin = ptr::null();
                    }
                }
                b'#' => {
                    eprintln!("error: invalid comment in target definition");
                    exit(1)
                }
                b'\n' => {
                    let prev = ptr.sub(1);

                    if ptr != str_begin && *prev != b':' {
                        eprintln!("error: invalid dependency file syntax");
                        exit(1);
                    }

                    self.emit_target(str_begin, prev);

                    return ptr.add(1);
                }
                _ => {
                    if str_begin.is_null() {
                        str_begin = ptr;
                    }

                    ptr = ptr.add(1);
                }
            }
        }

        ptr
    }

    fn emit_target(&mut self, begin: *const u8, end: *const u8) {
        let target = util::make_str(begin, end);
        self.deps.push(Dependency::new(target));
    }

    unsafe fn parse_prerequisites(
        &mut self,
        start: usize,
        begin: *const u8,
        end: *const u8,
    ) -> *const u8 {
        let mut done = false;
        let mut ptr = begin;

        while ptr < end && !done {
            match *ptr {
                b' ' | b'\t' | b'\\' => {}
                b'\n' => {
                    if ptr != begin && *ptr.sub(1) != b'\\' {
                        return ptr.add(1);
                    }
                }
                b'#' => {
                    ptr = util::skip_comment(ptr, begin);
                    continue;
                }
                _ => {
                    (ptr, done) = self.parse_prerequisite(start, ptr, end);
                    continue;
                }
            }

            ptr = ptr.add(1);
        }

        ptr
    }

    unsafe fn parse_prerequisite(
        &mut self,
        start: usize,
        begin: *const u8,
        end: *const u8,
    ) -> (*const u8, bool) {
        let mut ptr = begin;

        while ptr < end {
            match *ptr {
                b'\n' => {
                    if ptr != begin && *ptr.sub(1) != b'\\' {
                        self.emit_prerequisite(start, begin, ptr);

                        return (ptr.add(1), true);
                    }
                }
                b'#' => {
                    if ptr != begin && *ptr.sub(1) != b'\\' {
                        self.emit_prerequisite(start, begin, ptr);
                        ptr = util::skip_comment(ptr, end);

                        return (ptr, false);
                    }
                }
                b' ' | b'\t' => {
                    self.emit_prerequisite(start, begin, ptr);

                    return (ptr.add(1), false);
                }
                _ => {}
            }

            ptr = ptr.add(1);
        }

        self.emit_prerequisite(start, begin, ptr);

        (ptr, false)
    }

    fn emit_prerequisite(
        &mut self,
        start: usize,
        begin: *const u8,
        end: *const u8,
    ) {
        let prereq = util::make_str(begin, end);

        for dep in &mut self.deps[start..] {
            dep.prerequisites.push(prereq);
        }
    }
}

mod util {
    use std::slice;
    use std::str;

    pub fn make_str<'a>(begin: *const u8, end: *const u8) -> &'a str {
        unsafe {
            let size = end as usize - begin as usize;
            let slice = slice::from_raw_parts(begin, size);

            str::from_utf8_unchecked(slice)
        }
    }

    pub fn skip_line(begin: *const u8, end: *const u8) -> *const u8 {
        unsafe {
            let mut ptr = begin;

            while ptr < end && *ptr != b'\n' {
                ptr = ptr.add(1);
            }

            ptr.add(1)
        }
    }

    pub fn skip_comment(begin: *const u8, end: *const u8) -> *const u8 {
        skip_line(begin, end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /**
     * DependencyParser::parse_targets()
     *
     * Verify that the function correctly deals with empty input.
     */
    #[test]
    fn parse_targets_001() {
        let data = "";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();

        let ptr = unsafe { parser.parse_targets(begin, end) };

        assert_eq!(begin, ptr);
        assert_eq!(end, ptr);
        assert_eq!(0, parser.deps.len());
    }

    /**
     * DependencyParser::parse_targets()
     *
     * Verify that the function correctly deals with one target in a rule.
     */
    #[test]
    fn parse_targets_002() {
        let data = "a: ";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();

        let ptr = unsafe { parser.parse_targets(begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(1, parser.deps.len());
        assert_eq!("a", parser.deps[0].target);
    }

    /**
     * DependencyParser::parse_targets()
     *
     * Verify that the function correctly deals with two targets in a rule.
     */
    #[test]
    fn parse_targets_003() {
        let data = "a b: ";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();

        let ptr = unsafe { parser.parse_targets(begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(2, parser.deps.len());
        assert_eq!("a", parser.deps[0].target);
        assert_eq!("b", parser.deps[1].target);
    }

    /**
     * DependencyParser::parse_targets()
     *
     * Verify that the function correctly deals with three targets in a rule.
     */
    #[test]
    fn parse_targets_004() {
        let data = "a b c: ";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();
        
        let ptr = unsafe { parser.parse_targets(begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(3, parser.deps.len());
        assert_eq!("a", parser.deps[0].target);
        assert_eq!("b", parser.deps[1].target);
        assert_eq!("c", parser.deps[2].target);
    }

    /**
     * DependencyParser::parse_targets()
     *
     * Verify that the function correctly deals with targets separated
     * by two spaces in a rule.
     */
    #[test]
    fn parse_targets_005() {
        let data = "a  b: ";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();

        let ptr = unsafe { parser.parse_targets(begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(2, parser.deps.len());
        assert_eq!("a", parser.deps[0].target);
        assert_eq!("b", parser.deps[1].target);
    }

    /**
     * DependencyParser::parse_targets()
     *
     * Verify that the function correctly deals with a space before the
     * colon marking the end of the list defining one target.
     */
    #[test]
    fn parse_targets_006() {
        let data = "a : ";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();

        let ptr = unsafe { parser.parse_targets(begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(1, parser.deps.len());
        assert_eq!("a", parser.deps[0].target);
    }

    /**
     * DependencyParser::parse_targets()
     *
     * Verify that the function correctly deals with a space before the
     * colon marking the end of the list defining two targets.
     */
    #[test]
    fn parse_targets_007() {
        let data = "a b : ";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();

        let ptr = unsafe { parser.parse_targets(begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(2, parser.deps.len());
        assert_eq!("a", parser.deps[0].target);
        assert_eq!("b", parser.deps[1].target);
    }

    /**
     * DependencyParser::parse_prerequisite()
     *
     * Verify that the function correctly deals with an empty input.
     */
    #[test]
    fn parse_prerequisite_001() {
        let data = "";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();

        let (ptr, done) = unsafe { parser.parse_prerequisite(0, begin, end) };

        assert_eq!(false, done);
        assert_eq!(end, ptr);
        assert_eq!(0, parser.deps.len());
    }

    /**
     * DependencyParser::parse_prerequisite()
     *
     * Verify that the function correctly deals with one prerequisite.
     */
    #[test]
    fn parse_prerequisite_002() {
        let data = "a";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();

        let (ptr, done) = unsafe { parser.parse_prerequisite(0, begin, end) };

        assert_eq!(false, done);
        assert_eq!(end, ptr);
        assert_eq!(0, parser.deps.len());
    }

    /**
     * DependencyParser::parse_prerequisite()
     *
     * Verify that the function correctly deals with one prerequisite given
     * two prerequisites as input.
     */
    #[test]
    fn parse_prerequisite_003() {
        let data = "a b";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();

        let (ptr, done) = unsafe { parser.parse_prerequisite(0, begin, end) };

        assert_eq!(false, done);
        assert_eq!(unsafe { begin.add(2) }, ptr);
        assert_eq!(0, parser.deps.len());
    }

    /**
     * DependencyParser::parse_prerequisites()
     *
     * Verify that the function correctly deals with an empty input.
     */
    #[test]
    fn parse_prerequisites_001() {
        let data = "";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();

        let ptr = unsafe { parser.parse_prerequisites(0, begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(0, parser.deps.len());
    }

    /**
     * DependencyParser::parse_prerequisites()
     *
     * Verify that the function correctly deals with an empty input.
     */
    #[test]
    fn parse_prerequisites_002() {
        let data = "";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();
        parser.deps.push(Dependency::new("a"));

        let ptr = unsafe { parser.parse_prerequisites(0, begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(1, parser.deps.len());
        assert_eq!(0, parser.deps[0].prerequisites.len());
    }

    /**
     * DependencyParser::parse_prerequisites()
     *
     * Verify that the function correctly deals with one prerequisite.
     */
    #[test]
    fn parse_prerequisites_003() {
        let data = "a";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();

        let ptr = unsafe { parser.parse_prerequisites(0, begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(0, parser.deps.len());
    }

    /**
     * DependencyParser::parse_prerequisites()
     *
     * Verify that the function correctly deals with one prerequisite.
     */
    #[test]
    fn parse_prerequisites_004() {
        let data = "b";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();
        parser.deps.push(Dependency::new("a"));

        let ptr = unsafe { parser.parse_prerequisites(0, begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(1, parser.deps.len());
        assert_eq!(1, parser.deps[0].prerequisites.len());
        assert_eq!("b", parser.deps[0].prerequisites[0]);
    }

    /**
     * DependencyParser::parse_prerequisites()
     *
     * Verify that the function correctly deals with two prerequisites.
     */
    #[test]
    fn parse_prerequisites_005() {
        let data = "b c";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();
        parser.deps.push(Dependency::new("a"));

        let ptr = unsafe { parser.parse_prerequisites(0, begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(1, parser.deps.len());
        assert_eq!(2, parser.deps[0].prerequisites.len());
        assert_eq!("b", parser.deps[0].prerequisites[0]);
        assert_eq!("c", parser.deps[0].prerequisites[1]);
    }

    /**
     * DependencyParser::parse_prerequisites()
     *
     * Verify that the function correctly deals with two prerequisites.
     */
    #[test]
    fn parse_prerequisites_006() {
        let data = "c d";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();
        parser.deps.push(Dependency::new("a"));
        parser.deps.push(Dependency::new("b"));

        let ptr = unsafe { parser.parse_prerequisites(0, begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(2, parser.deps.len());

        assert_eq!(2, parser.deps[0].prerequisites.len());
        assert_eq!("c", parser.deps[0].prerequisites[0]);
        assert_eq!("d", parser.deps[0].prerequisites[1]);

        assert_eq!(2, parser.deps[1].prerequisites.len());
        assert_eq!("c", parser.deps[1].prerequisites[0]);
        assert_eq!("d", parser.deps[1].prerequisites[1]);
    }

    /**
     * DependencyParser::parse_prerequisites()
     *
     * Verify that the function correctly deals with a tab at the beginning
     * of the prerequisites string.
     */
    #[test]
    fn parse_prerequisites_007() {
        let data = "\tc d";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();
        parser.deps.push(Dependency::new("a"));
        parser.deps.push(Dependency::new("b"));

        let ptr = unsafe { parser.parse_prerequisites(0, begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(2, parser.deps.len());

        assert_eq!(2, parser.deps[0].prerequisites.len());
        assert_eq!("c", parser.deps[0].prerequisites[0]);
        assert_eq!("d", parser.deps[0].prerequisites[1]);

        assert_eq!(2, parser.deps[1].prerequisites.len());
        assert_eq!("c", parser.deps[1].prerequisites[0]);
        assert_eq!("d", parser.deps[1].prerequisites[1]);
    }

    /**
     * DependencyParser::parse_prerequisites()
     *
     * Verify that the function correctly deals with a tab at the beginning
     * of each prerequisite string.
     */
    #[test]
    fn parse_prerequisites_008() {
        let data = "\tc\td";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();
        parser.deps.push(Dependency::new("a"));
        parser.deps.push(Dependency::new("b"));

        let ptr = unsafe { parser.parse_prerequisites(0, begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(2, parser.deps.len());

        assert_eq!(2, parser.deps[0].prerequisites.len());
        assert_eq!("c", parser.deps[0].prerequisites[0]);
        assert_eq!("d", parser.deps[0].prerequisites[1]);

        assert_eq!(2, parser.deps[1].prerequisites.len());
        assert_eq!("c", parser.deps[1].prerequisites[0]);
        assert_eq!("d", parser.deps[1].prerequisites[1]);
    }

    /**
     * DependencyParser::parse_rule()
     *
     * Verify that the function correctly deals with an empty input.
     */
    #[test]
    fn parse_rule_001() {
        let data = "";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();

        let ptr = unsafe { parser.parse_rule(begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(0, parser.deps.len());
    }

    /**
     * DependencyParser::parse_rule()
     *
     * Verify that the function correctly deals with an empty input.
     */
    #[test]
    fn parse_rule_002() {
        let data = "";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();
        parser.deps.push(Dependency::new("a"));

        let ptr = unsafe { parser.parse_rule(begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(1, parser.deps.len());
        assert_eq!(0, parser.deps[0].prerequisites.len());
    }

    /**
     * DependencyParser::parse_rule()
     *
     * Verify that the function correctly deals with a rule consisting of
     * one target and one dependency.
     */
    #[test]
    fn parse_rule_003() {
        let data = "a: b";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();

        let ptr = unsafe { parser.parse_rule(begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(1, parser.deps.len());
        assert_eq!("a", parser.deps[0].target);

        assert_eq!(1, parser.deps[0].prerequisites.len());
        assert_eq!("b", parser.deps[0].prerequisites[0]);
    }

    /**
     * DependencyParser::parse_rule()
     *
     * Verify that the function correctly deals with a rule consisting of
     * one target and two dependencies.
     */
    #[test]
    fn parse_rule_004() {
        let data = "a: b c";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();

        let ptr = unsafe { parser.parse_rule(begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(1, parser.deps.len());
        assert_eq!("a", parser.deps[0].target);

        assert_eq!(2, parser.deps[0].prerequisites.len());
        assert_eq!("b", parser.deps[0].prerequisites[0]);
        assert_eq!("c", parser.deps[0].prerequisites[1]);
    }

    /**
     * DependencyParser::parse_rule()
     *
     * Verify that the function correctly deals with a rule consisting of
     * two target and one dependency.
     */
    #[test]
    fn parse_rule_005() {
        let data = "a b: c";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();

        let ptr = unsafe { parser.parse_rule(begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(2, parser.deps.len());
        assert_eq!("a", parser.deps[0].target);
        assert_eq!("b", parser.deps[1].target);

        assert_eq!(1, parser.deps[0].prerequisites.len());
        assert_eq!("c", parser.deps[0].prerequisites[0]);

        assert_eq!(1, parser.deps[1].prerequisites.len());
        assert_eq!("c", parser.deps[1].prerequisites[0]);
    }

    /**
     * DependencyParser::parse_rule()
     *
     * Verify that the function correctly deals with a rule consisting of
     * two target and two dependencies.
     */
    #[test]
    fn parse_rule_006() {
        let data = "a b: c d";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();

        let ptr = unsafe { parser.parse_rule(begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(2, parser.deps.len());
        assert_eq!("a", parser.deps[0].target);
        assert_eq!("b", parser.deps[1].target);

        assert_eq!(2, parser.deps[0].prerequisites.len());
        assert_eq!("c", parser.deps[0].prerequisites[0]);
        assert_eq!("d", parser.deps[0].prerequisites[1]);

        assert_eq!(2, parser.deps[1].prerequisites.len());
        assert_eq!("c", parser.deps[1].prerequisites[0]);
        assert_eq!("d", parser.deps[1].prerequisites[1]);
    }

    /**
     * DependencyParser::parse_rule()
     *
     * Verify that the function correctly deals with a rule consisting of
     * one target and two dependencies started by a tab.
     */
    #[test]
    fn parse_rule_007() {
        let data = "a:\tb c";
        let range = data.as_bytes().as_ptr_range();
        let (begin, end) = (range.start, range.end);

        let mut parser = DependencyParser::new();

        let ptr = unsafe { parser.parse_rule(begin, end) };

        assert_eq!(end, ptr);
        assert_eq!(1, parser.deps.len());
        assert_eq!("a", parser.deps[0].target);

        assert_eq!(2, parser.deps[0].prerequisites.len());
        assert_eq!("b", parser.deps[0].prerequisites[0]);
        assert_eq!("c", parser.deps[0].prerequisites[1]);
    }


    /**
     * DependencyParser::parse()
     *
     * Verify that the function correctly deals with empty input.
     */
    #[test]
    fn parse_001() {
        let mut parser = DependencyParser::new();

        let _ = parser.parse(Vec::from(""));

        assert_eq!(0, parser.deps.len());
        assert_eq!(0, parser.data.len());
    }

    /**
     * DependencyParser::parse()
     *
     * Verify that the function can handle a rule containing a backslash.
     */
    #[test]
    fn parse_002() {
        let data = Vec::from("a: b \\\n c");
        let mut parser = DependencyParser::new();

        let deps = parser.parse(data);

        assert_eq!(1, deps.len());
        assert_eq!("a", deps[0].target);

        assert_eq!(2, deps[0].prerequisites.len());
        assert_eq!("b", deps[0].prerequisites[0]);
        assert_eq!("c", deps[0].prerequisites[1]);
    }

    /**
     * DependencyParser::parse()
     *
     * Verify that the function can handle two consecutive dependencies which
     * are not separated by an empty line.
     */
    #[test]
    fn parse_003() {
        let data = Vec::from("a: b\nc: d");
        let mut parser = DependencyParser::new();

        let deps = parser.parse(data);

        assert_eq!(2, deps.len());
        assert_eq!("a", deps[0].target);
        assert_eq!("c", deps[1].target);

        assert_eq!(1, deps[0].prerequisites.len());
        assert_eq!("b", deps[0].prerequisites[0]);

        assert_eq!(1, deps[1].prerequisites.len());
        assert_eq!("d", deps[1].prerequisites[0]);
    }

    /**
     * DependencyParser::parse()
     *
     * Verify that the function correctly deals with complex input.
     */
    #[test]
    fn parse_004() {
        let mut data = Vec::new();

        data.append(&mut Vec::from("#a: b\n"));
        data.append(&mut Vec::from("a: c d e f g\n"));
        data.append(&mut Vec::from("\n"));
        data.append(&mut Vec::from("b: c d e f g\n"));
        data.append(&mut Vec::from("\n"));
        data.append(&mut Vec::from("c:\n"));
        data.append(&mut Vec::from("d:\n"));
        data.append(&mut Vec::from("e:\n"));
        data.append(&mut Vec::from("f:\n"));
        data.append(&mut Vec::from("g:\n"));
        data.append(&mut Vec::from("\n"));
        data.append(&mut Vec::from("#\n"));

        let mut parser = DependencyParser::new();

        let deps = parser.parse(data);

        assert_eq!(7, deps.len());

        assert_eq!("a", deps[0].target);
        assert_eq!("b", deps[1].target);
        assert_eq!("c", deps[2].target);
        assert_eq!("d", deps[3].target);
        assert_eq!("e", deps[4].target);
        assert_eq!("f", deps[5].target);
        assert_eq!("g", deps[6].target);

        assert_eq!(5, deps[0].prerequisites.len());
        assert_eq!(5, deps[1].prerequisites.len());
        assert_eq!(0, deps[2].prerequisites.len());
        assert_eq!(0, deps[3].prerequisites.len());
        assert_eq!(0, deps[4].prerequisites.len());
        assert_eq!(0, deps[5].prerequisites.len());
        assert_eq!(0, deps[6].prerequisites.len());

        assert_eq!("c", deps[0].prerequisites[0]);
        assert_eq!("d", deps[0].prerequisites[1]);
        assert_eq!("e", deps[0].prerequisites[2]);
        assert_eq!("f", deps[0].prerequisites[3]);
        assert_eq!("g", deps[0].prerequisites[4]);

        assert_eq!("c", deps[1].prerequisites[0]);
        assert_eq!("d", deps[1].prerequisites[1]);
        assert_eq!("e", deps[1].prerequisites[2]);
        assert_eq!("f", deps[1].prerequisites[3]);
        assert_eq!("g", deps[1].prerequisites[4]);
    }

    /**
     * DependencyParser::merge_deps()
     *
     * Verify that the function correctly deals with an empty dependency vector.
     */
    #[test]
    fn merge_deps_001() {
        let mut parser = DependencyParser::new();
        parser.merge_deps();

        assert_eq!(0, parser.deps.len());
    }

    /**
     * DependencyParser::merge_deps()
     *
     * Verify that the function correctly deals with one element inside the
     * dependency vector.
     */
    #[test]
    fn merge_deps_002() {
        let mut parser = DependencyParser::new();
        parser.deps.push(Dependency::new("a"));

        parser.merge_deps();

        assert_eq!(1, parser.deps.len());
        assert_eq!("a", parser.deps[0].target);
    }

    /**
     * DependencyParser::merge_deps()
     *
     * Verify that the function correctly deals with two identical targets
     * containing the same prerequisites inside the dependency vector.
     */
    #[test]
    fn merge_deps_003() {
        let mut parser = DependencyParser::new();

        parser.deps.push(Dependency::new("a"));
        parser.deps.push(Dependency::new("a"));

        parser.deps[0].prerequisites.push("b");
        parser.deps[1].prerequisites.push("b");

        parser.merge_deps();

        assert_eq!(1, parser.deps.len());
        assert_eq!("a", parser.deps[0].target);
        assert_eq!(1, parser.deps[0].prerequisites.len());
        assert_eq!("b", parser.deps[0].prerequisites[0]);
    }

    /**
     * DependencyParser::merge_deps()
     *
     * Verify that the function correctly deals with two identical targets
     * containing the different prerequisites inside the dependency vector.
     */
    #[test]
    fn merge_deps_004() {
        let mut parser = DependencyParser::new();

        parser.deps.push(Dependency::new("a"));
        parser.deps.push(Dependency::new("a"));

        parser.deps[0].prerequisites.push("b");
        parser.deps[1].prerequisites.push("c");

        parser.merge_deps();

        assert_eq!(1, parser.deps.len());
        assert_eq!("a", parser.deps[0].target);

        assert_eq!(2, parser.deps[0].prerequisites.len());
        assert_eq!("b", parser.deps[0].prerequisites[0]);
        assert_eq!("c", parser.deps[0].prerequisites[1]);
    }
}
