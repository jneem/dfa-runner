extern crate aho_corasick;
extern crate memchr;
extern crate memmem;

#[cfg(test)]
#[macro_use] extern crate matches;

use std::fmt::Debug;

pub trait Engine: Debug {
    fn shortest_match(&self, s: &str) -> Option<(usize, usize)>;
    fn clone_box(&self) -> Box<Engine>;
}

pub mod backtracking;
pub mod prefix;
pub mod program;
pub mod threaded;

