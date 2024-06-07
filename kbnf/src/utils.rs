//! Utility functions for the library.
use ahash::AHashMap;
use ebnf::node::FinalNode;
use ebnf::regex::FiniteStateAutomaton;
use ebnf::simplified_grammar::SimplifiedGrammar;
use fixedbitset::on_stack::{get_nblock, FixedBitSet};
use nom::error::VerboseError;
use regex_automata::dfa::Automaton;
use regex_automata::util::primitives::StateID;

use crate::config::InternalConfig;
use crate::grammar::GrammarError;

pub(crate) type ByteSet = FixedBitSet<{ get_nblock(u8::MAX as usize) }>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
pub(crate) enum FsaStateStatus {
    Accept,
    Reject,
    InProgress,
}
/// Helper function to construct a simplified grammar from an EBNF grammar string.
pub fn construct_ebnf_grammar(
    input: &str,
    config: InternalConfig,
) -> Result<SimplifiedGrammar, GrammarError> {
    let grammar = ebnf::get_grammar(input).map_err(|e| match e {
        nom::Err::Error(e) => nom::Err::Error(VerboseError {
            errors: e
                .errors
                .into_iter()
                .map(|(e, v)| (e.to_string(), v))
                .collect::<Vec<_>>(),
        }),
        nom::Err::Failure(e) => nom::Err::Failure(VerboseError {
            errors: e
                .errors
                .into_iter()
                .map(|(e, v)| (e.to_string(), v))
                .collect::<Vec<_>>(),
        }),
        nom::Err::Incomplete(e) => nom::Err::Incomplete(e),
    })?;
    let grammar = grammar.validate_grammar(&config.start_nonterminal, config.regex_config)?;
    let grammar = grammar.simplify_grammar(
        config.compression_config,
        config.excepted_config,
        &regex_automata::util::start::Config::new().anchored(regex_automata::Anchored::Yes),
    );
    Ok(grammar)
}
/// Helper function to find the maximum repetition from an EBNF grammar.
/// This is useful for determining [EngineBase](crate::engine_base::EngineBase) and [Grammar](crate::grammar::Grammar)'s generic parameter(TI).
pub fn find_max_repetition_from_ebnf_grammar(grammar: &SimplifiedGrammar) -> usize {
    let mut max_repetition = 0;
    for rule in grammar.expressions.iter() {
        for production in rule.alternations.iter() {
            for symbol in production.concatenations.iter() {
                if let &FinalNode::EXCEPT(_, Some(r)) = symbol {
                    max_repetition = max_repetition.max(r);
                }
            }
        }
    }
    max_repetition
}
/// Helper function to find the maximum state ID from an EBNF grammar.
/// This is useful for determining [EngineBase](crate::engine_base::EngineBase) and [Grammar](crate::grammar::Grammar)'s generic parameter(TS).
pub fn find_max_state_id_from_ebnf_grammar(grammar: &SimplifiedGrammar) -> usize {
    let mut max_state_id = 0;
    let terminals = &grammar.interned_strings.terminals;
    for (_, i) in terminals {
        max_state_id = max_state_id.max(i.bytes().len());
    }
    let regexes = &grammar.id_to_regex;
    for i in regexes {
        max_state_id = max_state_id.max(match i {
            FiniteStateAutomaton::Dfa(dfa) => dfa.state_len(),
        });
    }
    let excepted = &grammar.id_to_excepted;
    for i in excepted {
        max_state_id = max_state_id.max(match i {
            FiniteStateAutomaton::Dfa(dfa) => dfa.state_len(),
        });
    }
    max_state_id
}
/// Helper function to find the maximum dotted position from an EBNF grammar.
/// This is useful for determining [EngineBase](crate::engine_base::EngineBase) and [Grammar](crate::grammar::Grammar)'s generic parameter(TD).
pub fn find_max_dotted_position_from_ebnf_grammar(grammar: &SimplifiedGrammar) -> usize {
    let mut max_dotted_position = 0;
    for i in grammar.expressions.iter() {
        for j in i.alternations.iter() {
            max_dotted_position = max_dotted_position.max(j.concatenations.len());
        }
    }
    max_dotted_position
}
/// Helper function to find the maximum production ID from an EBNF grammar.
/// This is useful for determining [EngineBase](crate::engine_base::EngineBase) and [Grammar](crate::grammar::Grammar)'s generic parameter(TP).
pub fn find_max_production_id_from_ebnf_grammar(grammar: &SimplifiedGrammar) -> usize {
    let mut max_production_id = 0;
    for i in grammar.expressions.iter() {
        max_production_id = max_production_id.max(i.alternations.len());
    }
    max_production_id
}
#[inline]
pub(crate) fn check_dfa_state_status(
    dfa_state: StateID,
    dfa: &regex_automata::dfa::dense::DFA<Vec<u32>>,
) -> FsaStateStatus {
    if dfa.is_special_state(dfa_state) && !dfa.is_match_state(dfa_state) {
        // If the state is a special state and not a match state, then it is a dead state/quit state.
        return FsaStateStatus::Reject;
    }
    if dfa.is_match_state(dfa.next_eoi_state(dfa_state)) {
        FsaStateStatus::Accept
    } else {
        FsaStateStatus::InProgress
    }
}

pub(crate) fn get_display_form_from_bitset_on_stack<const NBLOCK: usize>(
    bitset: &FixedBitSet<NBLOCK>,
) -> Vec<usize> {
    bitset.ones().collect()
}

pub(crate) fn get_display_form_from_bitset(bitset: &fixedbitset::FixedBitSet) -> Vec<usize> {
    bitset.ones().collect()
}

pub(crate) fn fill_debug_form_of_id_to_x<'a, T: std::fmt::Debug>(
    id_to_x: impl Iterator<Item = T> + 'a,
    get_str: impl Fn(usize) -> String,
) -> AHashMap<String, T> {
    id_to_x.enumerate().map(|(i, x)| (get_str(i), x)).collect()
}
