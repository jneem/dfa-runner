// Copyright 2015 Joe Neeman.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use aho_corasick::Automaton;
use Engine;
use prefix::{Prefix, PrefixSearcher};
use program::{Instructions, Program};

#[derive(Clone, Debug)]
pub struct BacktrackingEngine<Insts: Instructions> {
    prog: Program<Insts>,
    prefix: Prefix,
}

impl<Insts: Instructions> BacktrackingEngine<Insts> {
    pub fn new(prog: Program<Insts>, pref: Prefix) -> BacktrackingEngine<Insts> {
        BacktrackingEngine {
            prog: prog,
            prefix: pref,
        }
    }

    fn shortest_match_from<'a>(&self, input: &[u8], pos: usize, mut state: usize)
    -> Option<usize> {
        for pos in pos..input.len() {
            let (next_state, accepted) = self.prog.step(state, &input[pos..]);
            if let Some(bytes_ago) = accepted {
                // We need to use saturating_sub here because Nfa::determinize_for_shortest_match
                // makes it so that bytes_ago can be positive even when start_idx == 0.
                return Some(pos.saturating_sub(bytes_ago));
            } else if let Some(next_state) = next_state {
                state = next_state;
            } else {
                return None;
            }
        }

        if let Some(bytes_ago) = self.prog.check_eoi(state) {
            Some(input.len().saturating_sub(bytes_ago))
        } else {
            None
        }
    }

    fn shortest_match_from_searcher(&self, input: &[u8], search: &mut PrefixSearcher)
    -> Option<(usize, usize)> {
        while let Some(res) = search.search() {
            if let Some(end) = self.shortest_match_from(input, res.end_pos, res.end_state) {
                return Some((res.start_pos, end));
            }
        }

        None
    }
}

impl<I: Instructions + 'static> Engine for BacktrackingEngine<I> {
    fn shortest_match(&self, s: &str) -> Option<(usize, usize)> {
        let input = s.as_bytes();
        if self.prog.num_states() == 0 {
            return None;
        } else if self.prog.is_anchored {
            return self.shortest_match_from(input, 0, 0).map(|x| (0, x));
        }

        let mut searcher = self.prefix.make_searcher(input);
        self.shortest_match_from_searcher(input, &mut *searcher)
    }

    fn clone_box(&self) -> Box<Engine> {
        Box::new(self.clone())
    }
}
