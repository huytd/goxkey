/// This module, `parser`, is built for the goxscript language.
/// It parses the goxscript language and returns an AST which can be used to
/// generate the corresponding vi-rs rule map.
///
/// # Example
/// The script would look like this:
///
/// ```
/// import telex
/// import vni
///
/// on s or ': add_tone(acute) end
///
/// on a or e or o or 6:
///   letter_mod(circumflex for a or e or o)
/// end
///
/// on w or 7 or 8:
///   reset_inserted_uw() or
///   letter_mod(horn or breve for u or o) or
///   insert_uw()
/// end
/// ```
///
/// # Syntax
/// The following EBNF describes the syntax of the goxscript language:
///
/// ```ebnf
/// <program> ::= <import_list>? <whitespace> <block_list>?
///
/// <import_list> ::= <import> ( <whitespace> <import_list> )?
/// <import> ::= "import" <whitespace> <identifier>
///
/// <block_list> ::= <block> ( <whitespace> <block_list> )?
/// <block> ::= "on" <whitespace> <key_list> <whitespace> ":" <whitespace> <function_call_list> <whitespace> "end"
///
/// <function_call_list> ::= <function_call> ( <whitespace> "or" <whitespace> <function_call_list> )?
/// <function_call> ::= <identifier> "(" ( <identifier_list> ( <whitespace> "for" <whitespace> <key_list> )? )? ")"
///
/// <identifier_list> ::= <identifier> ( <whitespace> "or" <whitespace> <identifier_list> )?
/// <identifier> ::= (<upper_letter> | <lower_letter> | <digit> | "_")+
///
/// <key_list> ::= <key> ( <whitespace> "or" <whitespace> <key_list> )?
/// <key> ::= <any_character>
///
/// <whitespace> ::= (" " | "\n")*
/// <any_character> ::= <upper_letter> | <lower_letter> | <digit> | <punctuation>
/// <upper_letter> ::= "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "I" | "J" | "K" | "L" | "M" | "N" | "O" |
///                    "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X" | "Y" | "Z"
/// <lower_letter> ::= "a" | "b" | "c" | "d" | "e" | "f" | "g" | "h" | "i" | "j" | "k" | "l" | "m" | "n" | "o" |
///                    "p" | "q" | "r" | "s" | "t" | "u" | "v" | "w" | "x" | "y" | "z"
/// <digit> ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
/// <punctuation> ::= "!" | "\"" | "#" | "$" | "%" | "&" | "'" | "(" | ")" | "*" | "+" | "," | "-" | "." | "/" |
///                   ":" | ";" | "<" | "=" | ">" | "?" | "@" | "[" | "\\" | "]" | "^" | "_" | "`" | "{" | "}" | "~"
/// ```
pub mod parser;
