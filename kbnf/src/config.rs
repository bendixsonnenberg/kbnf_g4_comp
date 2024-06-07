//! The configuration module of the KBNF engine.
use ebnf::regex::FiniteStateAutomatonConfig;
use serde::{Deserialize, Serialize};

use crate::engine_base::EngineConfig;
#[derive(Debug, Clone)]
/// The internal configuration of the KBNF engine. This is intended for advanced usages.
pub struct InternalConfig {
    /// The configuration of the regular expressions.
    pub regex_config: FiniteStateAutomatonConfig,
    /// The configuration about how to compress terminals in the grammar.
    pub compression_config: ebnf::config::CompressionConfig,
    /// The configuration of the engine itself.
    pub engine_config: EngineConfig,
    /// The configuration of except!.
    pub excepted_config: FiniteStateAutomatonConfig,
    /// The start nonterminal of the grammar.
    pub start_nonterminal: String,
}
/// The configuration of the `Engine` struct. This should suffice most scenarios.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Config {
    /// The configuration of the regular expressions.
    pub regex_config: RegexConfig,
    /// The configuration of except!.
    pub excepted_config: RegexConfig,
    /// The configuration of the engine.
    pub engine_config: EngineConfig,
    /// The start nonterminal of the grammar.
    /// The default is `start`.
    pub start_nonterminal: String,
    /// The length of the expected output in bytes.
    /// This is used to determine the index type used in EngineBase.
    /// IF you are sure that the output length will be short,
    /// you can set a shorter length to save memory and potentially speed up the engine.
    /// The default is `2^32-1`.
    pub expected_output_length: usize,
    /// The configuration of the compression.
    pub compression_config: CompressionConfig,
}
/// The type of the Finite State Automaton to be used.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FsaType {
    /// The Deterministic Finite Automaton.
    /// It is a deterministic finite automaton that eagerly computes all the state transitions.
    /// It is the fastest type of finite automaton, but it is also the most memory-consuming.
    /// In particular, construction time and space required could be exponential in the worst case.
    Dfa
}
/// The configuration of regular expressions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RegexConfig {
    /// The maximum memory usage in bytes allowed when compiling the regex.
    /// If the memory usage exceeds this limit, an error will be returned.
    /// The default is `None`, which means no limit for dfa and some reasonable limits for ldfa.
    pub max_memory_usage: Option<usize>,
    /// The type of the Finite State Automaton to be used.
    /// The default is `FsaType::Ldfa`.
    pub fsa_type: FsaType,
}

/// The configuration of regular expressions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CompressionConfig {
    /// The minimum number of terminals to be compressed. The default is 5.
    pub min_terminals: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            regex_config: RegexConfig {
                max_memory_usage: None,
                fsa_type: FsaType::Dfa,
            },
            excepted_config: RegexConfig {
                max_memory_usage: None,
                fsa_type: FsaType::Dfa,
            },
            engine_config: EngineConfig {
                cache_enabled: true,
                compaction_enabled: true,
            },
            start_nonterminal: "start".to_string(),
            compression_config: CompressionConfig { min_terminals: 5 },
            expected_output_length: u32::MAX as usize,
        }
    }
}

impl Config {
    /// Converts the configuration to the internal configuration.
    pub fn internal_config(self) -> InternalConfig {
        let regex_config = match self.regex_config.fsa_type {
            FsaType::Dfa => FiniteStateAutomatonConfig::Dfa(
                regex_automata::dfa::dense::Config::new()
                    .dfa_size_limit(self.regex_config.max_memory_usage)
                    .start_kind(regex_automata::dfa::StartKind::Anchored),
            )
        };
        let excepted_config = match self.excepted_config.fsa_type {
            FsaType::Dfa => FiniteStateAutomatonConfig::Dfa(
                regex_automata::dfa::dense::Config::new()
                    .dfa_size_limit(self.excepted_config.max_memory_usage),
            )
        };
        let compression_config = ebnf::config::CompressionConfig {
            min_terminals: self.compression_config.min_terminals,
            regex_config: FiniteStateAutomatonConfig::Dfa(regex_automata::dfa::dense::Config::new()),
        };
        InternalConfig {
            regex_config,
            compression_config,
            engine_config: self.engine_config,
            excepted_config,
            start_nonterminal: self.start_nonterminal,
        }
    }
}
