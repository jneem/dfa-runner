// Copyright 2015 Joe Neeman.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::fmt::{Debug, Formatter, Error as FmtError};
use std::{u32, usize};

pub trait RegexSearcher {
    fn shortest_match(&self, haystack: &str) -> Option<(usize, usize)>;
}

// TODO: get rid of this in favor of a bool saying whether we're anchored. From now on,
// start state is always zero.
#[derive(Clone, Debug)]
pub enum InitStates {
    Anchored(usize),
    Constant(usize),
}

impl InitStates {
    /// Returns the starting state if we are at the given pos in the input.
    pub fn state_at_pos(&self, _: &[u8], pos: usize) -> Option<usize> {
        use program::InitStates::*;

        match self {
            &Anchored(s) => if pos == 0 { Some(s) } else { None },
            &Constant(s) => Some(s),
        }
    }

    /// If we can start only at the beginning of the input, return the start state.
    pub fn anchored(&self) -> Option<usize> {
        match self {
            &InitStates::Anchored(s) => Some(s),
            _ => None,
        }
    }

}

#[derive(Clone, Debug, PartialEq)]
pub enum Inst {
    Byte(u8),
    ByteSet(usize),
    Acc(usize),
    Branch(usize),
}

pub trait Instructions: Clone + Debug {
    /// Returns (next_state, accept), where
    ///   - next_state is the next state to try
    ///   - accept gives some data associated with the acceptance.
    fn step(&self, state: usize, input: &[u8]) -> (Option<usize>, Option<usize>);

    /// The number of states in this program.
    fn num_states(&self) -> usize;
}

#[derive(Clone, Debug)]
pub struct Program<Insts: Instructions> {
    pub init: InitStates,
    pub accept_at_eoi: Vec<usize>,
    pub instructions: Insts,
}

impl<Insts: Instructions> Instructions for Program<Insts> {
    fn step(&self, state: usize, input: &[u8]) -> (Option<usize>, Option<usize>) {
        self.instructions.step(state, input)
    }

    fn num_states(&self) -> usize {
        self.instructions.num_states()
    }
}

impl<Insts: Instructions> Program<Insts> {
    /// If the program should accept at the end of input in state `state`, returns the data
    /// associated with the match.
    pub fn check_eoi(&self, state: usize) -> Option<usize> {
        if self.accept_at_eoi[state] != usize::MAX {
            Some(self.accept_at_eoi[state])
        } else {
            None
        }
    }

    /// If this program matches an empty match at the end of the input, return it.
    pub fn check_empty_match_at_end(&self, input: &[u8]) -> Option<(usize, usize)> {
        let pos = input.len();
        if let Some(state) = self.init.state_at_pos(input, pos) {
            if self.accept_at_eoi[state] != usize::MAX {
                return Some((pos, pos));
            }
        }
        None
    }


    /// The initial state when starting the program.
    pub fn init(&self) -> &InitStates {
        &self.init
    }
}

#[derive(Clone, PartialEq)]
pub struct VmInsts {
    pub byte_sets: Vec<bool>,
    pub branch_table: Vec<u32>,
    pub insts: Vec<Inst>,
}

impl Instructions for VmInsts {
    #[inline(always)]
    fn step(&self, state: usize, input: &[u8]) -> (Option<usize>, Option<usize>) {
        use program::Inst::*;
        match self.insts[state] {
            Acc(a) => {
                return (Some(state + 1), Some(a));
            },
            Byte(b) => {
                if b == input[0] {
                    return (Some(state + 1), None);
                }
            },
            ByteSet(bs_idx) => {
                if self.byte_sets[bs_idx + input[0] as usize] {
                    return (Some(state + 1), None);
                }
            },
            Branch(table_idx) => {
                let next_state = self.branch_table[table_idx + input[0] as usize];
                if next_state != u32::MAX {
                    return (Some(next_state as usize), None);
                }
            },
        }
        (None, None)
    }

    fn num_states(&self) -> usize {
        self.insts.len()
    }
}


impl Debug for VmInsts {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        try!(f.write_fmt(format_args!("VmInsts ({} instructions):\n", self.insts.len())));

        for (idx, inst) in self.insts.iter().enumerate() {
            try!(f.write_fmt(format_args!("\tInst {}: {:?}\n", idx, inst)));
        }
        Ok(())
    }
}

pub type TableStateIdx = u32;

/// A DFA program implemented as a lookup table.
#[derive(Clone)]
pub struct TableInsts {
    /// A `256 x num_instructions`-long table.
    pub table: Vec<TableStateIdx>,
    /// If `accept[st]` is not `usize::MAX`, then it gives some data to return if we match the
    /// input when we're in state `st`.
    pub accept: Vec<usize>,
}

impl Debug for TableInsts {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        try!(f.write_fmt(format_args!("TableInsts ({} instructions):\n", self.accept.len())));

        for idx in 0..self.accept.len() {
            try!(f.write_fmt(format_args!("State {}:\n", idx)));
            try!(f.debug_map()
                .entries((0usize..255)
                    .map(|c| (c, self.table[idx * 256 + c]))
                    .filter(|x| x.1 != u32::MAX))
                .finish());
            try!(f.write_str("\n"));
        }

        try!(f.write_str("Accept: "));
        for idx in 0..self.accept.len() {
            let val = self.accept[idx];
            if val != usize::MAX {
                try!(f.write_fmt(format_args!("{} -> {}, ", idx, val)));
            }
        }
        Ok(())
    }
}


impl Instructions for TableInsts {
    #[inline(always)]
    fn step(&self, state: usize, input: &[u8]) -> (Option<usize>, Option<usize>) {
        let accept = self.accept[state];
        let next_state = self.table[state * 256 + input[0] as usize];

        let accept = if accept != usize::MAX { Some(accept) } else { None };
        let next_state = if next_state != u32::MAX { Some(next_state as usize) } else { None };

        (next_state, accept)
    }

    fn num_states(&self) -> usize {
        self.accept.len()
    }
}

