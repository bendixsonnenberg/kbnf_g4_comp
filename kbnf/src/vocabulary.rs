//! This module contains the `Vocabulary` struct, which represents a language model's vocabulary.
use ahash::AHashMap;
use jaggedarray::jagged_array::JaggedArray;
use jaggedarray::jagged_array::JaggedArrayViewTrait;
use nonmax::{NonMaxU32, NonMaxU8};
use std::array;
use std::fmt::Debug;
use tinyvec::ArrayVec;

const TOKEN_SEPARATOR: u8 = 0xFF;
const BYTES_NUM: usize = 257; // 256 + 1 because jagged array's implementation requires one additional index.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
/// A wrapper struct that represents a token in bytes in a language model's vocabulary.
pub struct Token(pub Box<[u8]>);
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct FirstBytes([u32; BYTES_NUM]);
impl tinyvec::Array for FirstBytes {
    type Item = u32;
    const CAPACITY: usize = BYTES_NUM;

    fn as_slice(&self) -> &[Self::Item] {
        &self.0
    }

    fn as_slice_mut(&mut self) -> &mut [Self::Item] {
        &mut self.0
    }

    fn default() -> Self {
        Self([0; 257])
    }
}
#[derive(Clone)]
/// The struct represents a language model's vocabulary.
pub struct Vocabulary {
    token_to_id: AHashMap<Token, u32>,
    id_to_token: AHashMap<u32, Token>,
    id_to_token_string: AHashMap<u32, String>,
    /// This field represents a map from the first byte of a token to the token id and token that DO NOT contain byte 0xFF.
    /// memory representation: \[Unicode unused byte\]\[token_id(3 bytes little endian)\]\[token(remaining bytes)\]
    // TODO: check whether a variable length token_id encoding is better
    first_byte_to_normal_tokens: JaggedArray<u8, ArrayVec<FirstBytes>, 2>,
    /// This field represents a map from the token id to the token that contains the Unicode unused byte in `first_byte_to_normal_tokens``.
    /// The number of such tokens is expected to be small so we probably do not need a jagged array(which does have some overhead).
    tokens_containing_separators: Vec<(u32, Token)>,
}

impl Debug for Vocabulary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vocabulary")
            .field("token_to_id", &self.token_to_id)
            .field("id_to_token", &self.id_to_token)
            .field("id_to_token_string", &self.id_to_token_string)
            .field("first_byte_to_normal_tokens", {
                let mut hash_map = AHashMap::new();
                for byte in 0..u8::MAX as usize + 1 {
                    let mut iter = self.get_normal_tokens_from_first_byte(byte as u8);
                    while let Some(item) = iter.next() {
                        if let TokenIterItem::TokenByte(byte) = item {
                            hash_map
                                .entry(iter.get_current_token_id().unwrap())
                                .or_insert_with(Vec::new)
                                .push(byte.get());
                        }
                    }
                }
                &Box::new(hash_map)
            })
            .field(
                "tokens_containing_separators",
                &self.tokens_containing_separators,
            )
            .finish()
    }
}

impl Vocabulary {
    /// Creates a new instance of `Vocabulary`. ID to token is separated into two fields: `id_to_token` and `id_to_token_string`,
    /// which allows the user to use custom encoding and to represent tokens that cannot be directly decoded to string.
    ///
    /// # Arguments
    ///
    /// * `token_to_id` - A HashMap that maps tokens to their corresponding IDs.
    /// * `id_to_token` - A vector that maps token IDs to their corresponding tokens in bytes.
    /// * `id_to_token_string` - A vector that maps token IDs to their corresponding token strings in UTF-8 String representation.
    /// This parameter is necessary because a token's UTF-8 representation may not be equivalent to the UTF-8 string decoded from its bytes,
    /// vice versa. For example, a token may contain `0xFF` byte.
    ///
    /// # Panics
    ///
    /// This function will panic if the length of `id_to_token` is greater than or equal to 2^24.
    pub fn new(
        token_to_id: AHashMap<Token, u32>,
        id_to_token: AHashMap<u32, Token>,
        id_to_token_string: AHashMap<u32, String>,
    ) -> Self {
        assert!(
            id_to_token.len() < 0x1000000,
            "max token id is larger than 2^24: {}",
            id_to_token.len() - 1
        );
        let mut first_byte_to_token = JaggedArray::with_capacity([256, 256]);
        let mut temp: [Vec<(u32, &Token)>; 256] = array::from_fn(|_| (vec![]));
        for (&token_id, token) in id_to_token.iter() {
            if token.0.is_empty() {
                continue;
            }
            let first_byte = token.0[0];
            temp[first_byte as usize].push((token_id, token));
        }
        let mut tokens_containing_separators = Vec::new();
        for tokens in temp.iter() {
            first_byte_to_token.new_row::<0>();
            for &(token_id, token) in tokens.iter() {
                let mut buffer = vec![TOKEN_SEPARATOR];
                if token.0.contains(&TOKEN_SEPARATOR) {
                    tokens_containing_separators.push((token_id, token.clone()));
                    continue;
                }
                buffer.extend(token_id.to_le_bytes().into_iter().take(3));
                buffer.extend(token.0.iter());
                first_byte_to_token.extend_last_row(buffer.into_iter());
            }
        }
        Self {
            token_to_id,
            id_to_token,
            id_to_token_string,
            first_byte_to_normal_tokens: first_byte_to_token,
            tokens_containing_separators,
        }
    }

    /// Retrieves the token ID associated with the given token.
    ///
    /// # Arguments
    ///
    /// * `token` - The token to retrieve the ID for.
    ///
    /// # Returns
    ///
    /// * `Some(u32)` - The token ID if it exists.
    /// * `None` - If the token does not exist in the vocabulary.
    pub fn get_token_id_from_token(&self, token: &Token) -> Option<u32> {
        self.token_to_id.get(token).copied()
    }

    /// Retrieves the token associated with the given token ID.
    ///
    /// # Arguments
    ///
    /// * `token_id` - The ID of the token to retrieve.
    ///
    /// # Returns
    ///
    /// * `Some(&Token)` - The token if it exists.
    /// * `None` - If the token ID is out of range.
    pub fn get_token_from_token_id(&self, token_id: u32) -> Option<&Token> {
        self.id_to_token.get(&token_id)
    }

    /// Retrieves the token string associated with the given token ID.
    ///
    /// # Arguments
    ///
    /// * `token_id` - The ID of the token to retrieve the string for.
    ///
    /// # Returns
    ///
    /// * `Some(&str)` - The token string if it exists.
    /// * `None` - If the token ID is out of range.
    pub fn get_token_string_from_token_id(&self, token_id: u32) -> Option<&str> {
        self.id_to_token_string.get(&token_id).map(|x| x.as_str())
    }

    /// Retrieves the size of the vocabulary.
    ///
    /// # Returns
    ///
    /// The number of tokens in the vocabulary.
    pub fn get_vocab_size(&self) -> usize {
        self.id_to_token.len()
    }

    /// Retrieves an iterator over the normal tokens that have the given first byte.
    ///
    /// # Arguments
    ///
    /// * `first_byte` - The first byte of the tokens to retrieve.
    ///
    /// # Returns
    ///
    /// An iterator over the normal tokens with the given first byte.
    pub(crate) fn get_normal_tokens_from_first_byte(&self, first_byte: u8) -> TokensIter {
        TokensIter {
            current_token_id: None,
            iter: self
                .first_byte_to_normal_tokens
                .view::<1, 1>([first_byte as usize])
                .as_slice()
                .iter(),
        }
    }

    /// Retrieves an iterator over the tokens that contain separators.
    ///
    /// # Returns
    ///
    /// An iterator over the tokens that contain separators.
    pub(crate) fn get_tokens_containing_separators(&self) -> impl Iterator<Item = (u32, &Token)> {
        self.tokens_containing_separators
            .iter()
            .map(|(x, y)| (*x, y))
    }
}
#[derive(Debug, Clone)]
pub(crate) struct TokensIter<'a> {
    current_token_id: Option<NonMaxU32>,
    iter: std::slice::Iter<'a, u8>,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum TokenIterItem {
    TokenByte(NonMaxU8),
    NewToken,
}

impl Iterator for TokensIter<'_> {
    type Item = TokenIterItem; // We excludes 0xFF from the token before

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|x| {
            if *x == TOKEN_SEPARATOR {
                let buffer = [
                    *self.iter.next().unwrap(),
                    *self.iter.next().unwrap(),
                    *self.iter.next().unwrap(),
                    0x00,
                ];
                self.current_token_id = Some(NonMaxU32::new(u32::from_le_bytes(buffer)).unwrap());
                self.current_token_id = Some(NonMaxU32::new(u32::from_le_bytes(buffer)).unwrap());
                TokenIterItem::NewToken
            } else {
                // SAFETY: We excludes 0xFF from the token before
                TokenIterItem::TokenByte(unsafe { NonMaxU8::new_unchecked(*x) })
            }
        })
    }
}

impl TokensIter<'_> {
    pub fn get_current_token_id(&self) -> Option<NonMaxU32> {
        self.current_token_id
    }
}
