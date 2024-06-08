use nom::{
    bytes::complete::{tag, take_while1, take_while_m_n},
    character::complete::{multispace0, multispace1},
    combinator::{map, opt},
    multi::separated_list1,
    sequence::{delimited, preceded, tuple},
    IResult,
};

/// Represents a program containing a list of imports and blocks.
///
/// # Example
///
/// ```
/// let program = Program {
///     import_list: Some(vec![Import { identifier: "telex".to_string() }]),
///     block_list: Some(vec![Block {
///         key_list: vec!["a".to_string()],
///         function_call_list: vec![FunctionCall {
///             identifier: "hello".to_string(),
///             identifier_list: None,
///             key_list: None,
///         }],
///     }]),
/// };
/// println!("{:?}", program);
/// ```
#[derive(Debug, PartialEq)]
pub struct Program {
    import_list: Option<Vec<Import>>,
    block_list: Option<Vec<Block>>,
}

/// Represents an import statement with an identifier.
///
/// # Example
///
/// ```
/// let import = Import {
///     identifier: "telex".to_string(),
/// };
/// println!("{:?}", import);
/// ```
#[derive(Debug, PartialEq)]
pub struct Import {
    identifier: String,
}

/// Represents a block containing a list of keys and function calls.
///
/// # Example
///
/// ```
/// let block = Block {
///     key_list: vec!["a".to_string()],
///     function_call_list: vec![FunctionCall {
///         identifier: "hello".to_string(),
///         identifier_list: None,
///         key_list: None,
///     }],
/// };
/// println!("{:?}", block);
/// ```
#[derive(Debug, PartialEq)]
pub struct Block {
    key_list: Vec<String>,
    function_call_list: Vec<FunctionCall>,
}

/// Represents a function call with an identifier, and optional lists of identifiers and keys.
///
/// # Example
///
/// ```
/// let function_call = FunctionCall {
///     identifier: "hello".to_string(),
///     identifier_list: Some(vec!["world".to_string()]),
///     key_list: Some(vec!["a".to_string()]),
/// };
/// println!("{:?}", function_call);
/// ```
#[derive(Debug, PartialEq)]
pub struct FunctionCall {
    identifier: String,
    identifier_list: Option<Vec<String>>,
    key_list: Option<Vec<String>>,
}

/// Checks if a character is a valid key character (not whitespace).
///
/// # Example
///
/// ```
/// let result = is_key_char('a');
/// assert!(result);
/// let result = is_key_char(' ');
/// assert!(!result);
/// ```
fn is_key_char(c: char) -> bool {
    !c.is_whitespace()
}

/// Parses a key from the input string.
///
/// # Example
///
/// ```
/// let result = parse_key("a");
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap().1, "a".to_string());
/// ```
fn parse_key(input: &str) -> IResult<&str, String> {
    map(take_while_m_n(1, 1, is_key_char), |s: &str| s.to_string())(input)
}

/// Parses a list of keys from the input string.
///
/// # Example
///
/// ```
/// let result = parse_key_list("a or b or c");
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap().1, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
/// ```
fn parse_key_list(input: &str) -> IResult<&str, Vec<String>> {
    separated_list1(delimited(multispace1, tag("or"), multispace1), parse_key)(input)
}

/// Checks if a character is a valid identifier character (alphanumeric or underscore).
///
/// # Example
///
/// ```
/// let result = is_identifier_char('a');
/// assert!(result);
/// let result = is_identifier_char('1');
/// assert!(result);
/// let result = is_identifier_char('_');
/// assert!(result);
/// let result = is_identifier_char(' ');
/// assert!(!result);
/// ```
fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Parses an identifier from the input string.
///
/// # Example
///
/// ```
/// let result = parse_identifier("abc123");
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap().1, "abc123".to_string());
/// ```
fn parse_identifier(input: &str) -> IResult<&str, String> {
    map(take_while1(is_identifier_char), |s: &str| s.to_string())(input)
}

/// Parses a list of identifiers from the input string.
///
/// # Example
///
/// ```
/// let result = parse_identifier_list("abc or def or ghi");
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap().1, vec!["abc".to_string(), "def".to_string(), "ghi".to_string()]);
/// ```
fn parse_identifier_list(input: &str) -> IResult<&str, Vec<String>> {
    separated_list1(
        delimited(multispace1, tag("or"), multispace1),
        parse_identifier,
    )(input)
}

/// Parses an import statement from the input string.
///
/// # Example
///
/// ```
/// let result = parse_import("import abc");
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap().1, Import { identifier: "abc".to_string() });
/// ```
fn parse_import(input: &str) -> IResult<&str, Import> {
    let (input, _) = preceded(tag("import"), multispace1)(input)?;
    let (input, identifier) = parse_identifier(input)?;
    Ok((
        input,
        Import {
            identifier: identifier.to_string(),
        },
    ))
}

/// Parses a list of import statements from the input string.
///
/// # Example
///
/// ```
/// let result = parse_import_list("import abc import def");
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap().1, vec![
///     Import { identifier: "abc".to_string() },
///     Import { identifier: "def".to_string() }
/// ]);
/// ```
fn parse_import_list(input: &str) -> IResult<&str, Vec<Import>> {
    separated_list1(multispace1, parse_import)(input)
}

/// Parses a function call from the input string.
///
/// # Example
///
/// ```
/// let result = parse_function_call("hello(world)");
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap().1, FunctionCall {
///     identifier: "hello".to_string(),
///     identifier_list: Some(vec!["world".to_string()]),
///     key_list: None,
/// });
/// ```
fn parse_function_call(input: &str) -> IResult<&str, FunctionCall> {
    let parse_identifier_list = opt(parse_identifier_list);
    let parse_key_list = map(
        opt(tuple((
            multispace1,
            tag("for"),
            multispace1,
            parse_key_list,
        ))),
        |x| x.map(|(_, _, _, key_list)| key_list),
    );
    let (input, (identifier, _, _, identifier_list, key_list, _, _)) = tuple((
        parse_identifier,
        tag("("),
        multispace0,
        parse_identifier_list,
        parse_key_list,
        multispace0,
        tag(")"),
    ))(input)?;
    Ok((
        input,
        FunctionCall {
            identifier: identifier.to_string(),
            identifier_list,
            key_list,
        },
    ))
}

/// Parses a list of function calls from the input string.
///
/// # Example
///
/// ```
/// let result = parse_function_call_list("hello() or world(abc)");
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap().1, vec![
///     FunctionCall {
///         identifier: "hello".to_string(),
///         identifier_list: None,
///         key_list: None,
///     },
///     FunctionCall {
///         identifier: "world".to_string(),
///         identifier_list: Some(vec!["abc".to_string()]),
///         key_list: None,
///     }
/// ]);
/// ```
fn parse_function_call_list(input: &str) -> IResult<&str, Vec<FunctionCall>> {
    separated_list1(
        delimited(multispace1, tag("or"), multispace1),
        parse_function_call,
    )(input)
}

/// Parses a block from the input string.
///
/// # Example
///
/// ```
/// let result = parse_block("on a: hello() end");
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap().1, Block {
///     key_list: vec!["a".to_string()],
///     function_call_list: vec![FunctionCall {
///         identifier: "hello".to_string(),
///         identifier_list: None,
///         key_list: None,
///     }],
/// });
/// ```
fn parse_block(input: &str) -> IResult<&str, Block> {
    let (input, (_, _, key_list, _, _, _, function_call_list, _, _)) = tuple((
        tag("on"),
        multispace1,
        parse_key_list,
        multispace0,
        tag(":"),
        multispace1,
        parse_function_call_list,
        multispace1,
        tag("end"),
    ))(input)?;
    Ok((
        input,
        Block {
            key_list,
            function_call_list,
        },
    ))
}

/// Parses a program from the input string.
///
/// # Example
///
/// ```
/// let result = parse_program("import telex\non a: hello() end");
/// assert!(result.is_ok());
/// assert_eq!(result.unwrap().1, Program {
///     import_list: Some(vec![Import { identifier: "telex".to_string() }]),
///     block_list: Some(vec![Block {
///         key_list: vec!["a".to_string()],
///         function_call_list: vec![FunctionCall {
///             identifier: "hello".to_string(),
///             identifier_list: None,
///             key_list: None,
///         }],
///     }]),
/// });
/// ```
pub fn parse_program(input: &str) -> IResult<&str, Program> {
    let parse_import_list = opt(parse_import_list);
    let parse_block_list = opt(separated_list1(multispace1, parse_block));
    let (input, (_, import_list, _, block_list, _)) = tuple((
        multispace0,
        parse_import_list,
        multispace0,
        parse_block_list,
        multispace0,
    ))(input)?;
    Ok((
        input,
        Program {
            import_list,
            block_list,
        },
    ))
}

#[test]
fn test_parse_key() {
    let input = "a";
    let result = parse_key(input);
    assert!(result.is_ok());
    assert!(result.unwrap().1 == "a");
}

#[test]
fn test_parse_key_should_parse_a_single_key() {
    let input = "abc";
    let result = parse_key(input);
    assert!(result.is_ok());
    assert!(result.unwrap().1 == "a");
}

#[test]
fn test_parse_key_list() {
    let input = "a or   b  or c";
    let result = parse_key_list(input);
    assert!(result.is_ok());
    println!("{result:?}");
    assert!(result.unwrap().1 == vec!["a", "b", "c"]);
}

#[test]
fn test_parse_identifier() {
    let input = "abc12_abc";
    let result = parse_identifier(input);
    assert!(result.is_ok());
    assert!(result.unwrap().1 == "abc12_abc");
}

#[test]
fn test_parse_identifier_list() {
    let input = "a or abc12 or ab_cd12";
    let result = parse_identifier_list(input);
    assert!(result.is_ok());
    assert!(result.unwrap().1 == vec!["a", "abc12", "ab_cd12"]);
}

#[test]
fn test_parse_identifier_list_single_item() {
    let input = "abc";
    let result = parse_identifier_list(input);
    assert!(result.is_ok());
    assert!(result.unwrap().1 == vec!["abc"]);
}

#[test]
fn test_parse_key_list_single() {
    let input = "a";
    let result = parse_key_list(input);
    assert!(result.is_ok());
    assert!(result.unwrap().1 == vec!["a"]);
}

#[test]
fn parse_import_fail() {
    let input = "import;";
    let result = parse_import(input);
    assert!(result.is_err());
}

#[test]
fn parse_import_fail_not_a_function() {
    let input = "import ()";
    let result = parse_import(input);
    assert!(result.is_err());
}

#[test]
fn parse_import_fail_no_module() {
    let input = "import";
    let result = parse_import(input);
    assert!(result.is_err());
}

#[test]
fn parse_import_fail_no_module_just_space() {
    let input = "import ";
    let result = parse_import(input);
    assert!(result.is_err());
}

#[test]
fn parse_import_success() {
    let input = "import abc";
    let result = parse_import(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == Import {
                identifier: "abc".to_string()
            }
    );
}

#[test]
fn parse_import_list_success_single() {
    let input = "import abc\n";
    let result = parse_import_list(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == vec![Import {
                identifier: "abc".to_string()
            }]
    );
}

#[test]
fn parse_import_list_success() {
    let input = "import abc import def";
    let result = parse_import_list(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == vec![
                Import {
                    identifier: "abc".to_string()
                },
                Import {
                    identifier: "def".to_string()
                }
            ]
    );
}

#[test]
fn parse_function_call_fail() {
    let input = "abc";
    let result = parse_function_call(input);
    assert!(result.is_err());
}

#[test]
fn parse_function_call_space_before_parens_fail() {
    let input = "abc ()";
    let result = parse_function_call(input);
    assert!(result.is_err());
}

#[test]
fn parse_function_call_success_with_no_params() {
    let input = "abc() ";
    let result = parse_function_call(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == FunctionCall {
                identifier: "abc".to_string(),
                identifier_list: None,
                key_list: None
            }
    );
}

#[test]
fn parse_function_call_success_with_no_params_with_space() {
    let input = "abc(  )";
    let result = parse_function_call(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == FunctionCall {
                identifier: "abc".to_string(),
                identifier_list: None,
                key_list: None
            }
    );
}

#[test]
fn parse_function_call_success_with_single_param() {
    let input = "abc(   hello   )";
    let result = parse_function_call(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == FunctionCall {
                identifier: "abc".to_string(),
                identifier_list: Some(vec!["hello".to_string()]),
                key_list: None
            }
    );
}

#[test]
fn parse_function_call_success_with_multiple_param() {
    let input = "say_this(   hello or word  )";
    let result = parse_function_call(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == FunctionCall {
                identifier: "say_this".to_string(),
                identifier_list: Some(vec!["hello".to_string(), "word".to_string()]),
                key_list: None
            }
    );
}

#[test]
fn parse_function_call_success_with_single_param_with_single_key() {
    let input = "say_this(   hello for a  )";
    let result = parse_function_call(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == FunctionCall {
                identifier: "say_this".to_string(),
                identifier_list: Some(vec!["hello".to_string()]),
                key_list: Some(vec!["a".to_string()])
            }
    );
}

#[test]
fn parse_function_call_success_with_single_param_with_multiple_key() {
    let input = "say_this(   hello for a or b or '  )";
    let result = parse_function_call(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == FunctionCall {
                identifier: "say_this".to_string(),
                identifier_list: Some(vec!["hello".to_string()]),
                key_list: Some(vec!["a".to_string(), "b".to_string(), "'".to_string()])
            }
    );
}

#[test]
fn parse_function_call_success_with_multiple_param_with_single_key() {
    let input = "say_this_123(   hello or world or zoo for a  )";
    let result = parse_function_call(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == FunctionCall {
                identifier: "say_this_123".to_string(),
                identifier_list: Some(vec![
                    "hello".to_string(),
                    "world".to_string(),
                    "zoo".to_string()
                ]),
                key_list: Some(vec!["a".to_string()])
            }
    );
}

#[test]
fn parse_function_call_success_with_multiple_param_with_multiple_key() {
    let input = "say_this_123(   hello or world or zoo for a or b or '  )";
    let result = parse_function_call(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == FunctionCall {
                identifier: "say_this_123".to_string(),
                identifier_list: Some(vec![
                    "hello".to_string(),
                    "world".to_string(),
                    "zoo".to_string()
                ]),
                key_list: Some(vec!["a".to_string(), "b".to_string(), "'".to_string()])
            }
    );
}

#[test]
fn parse_function_call_fail_with_multiple_param_with_no_key() {
    let input = "say_this_123(   hello or world or zoo for )";
    let result = parse_function_call(input);
    assert!(result.is_err());
}

#[test]
fn parse_function_call_fail_for_unclosed_call() {
    let input = "say_this_123(   hello or world or zoo ";
    let result = parse_function_call(input);
    assert!(result.is_err());
}

#[test]
fn parse_function_call_list_fail() {
    let input = "abc";
    let result = parse_function_call_list(input);
    assert!(result.is_err());
}

#[test]
fn parse_function_call_list_success_with_single_call() {
    let input = "abc()";
    let result = parse_function_call_list(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == vec![FunctionCall {
                identifier: "abc".to_string(),
                identifier_list: None,
                key_list: None
            }]
    );
}

#[test]
fn parse_function_call_list_success_with_multiple_call() {
    let input = "abc() or foo_bar(hello) or say_this(   hello or world or zoo for a or b or '  )";
    let result = parse_function_call_list(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == vec![
                FunctionCall {
                    identifier: "abc".to_string(),
                    identifier_list: None,
                    key_list: None
                },
                FunctionCall {
                    identifier: "foo_bar".to_string(),
                    identifier_list: Some(vec!["hello".to_string()]),
                    key_list: None
                },
                FunctionCall {
                    identifier: "say_this".to_string(),
                    identifier_list: Some(vec![
                        "hello".to_string(),
                        "world".to_string(),
                        "zoo".to_string()
                    ]),
                    key_list: Some(vec!["a".to_string(), "b".to_string(), "'".to_string()])
                }
            ]
    );
}

#[test]
fn parse_block_fail() {
    let input = "on abc: ";
    let result = parse_block(input);
    assert!(result.is_err());
}

#[test]
fn parse_block_fail_no_key() {
    let input = "on : end";
    let result = parse_block(input);
    assert!(result.is_err());
}

#[test]
fn parse_block_fail_empty_block() {
    let input = "on a: end";
    let result = parse_block(input);
    assert!(result.is_err());
}

#[test]
fn parse_block_success_single_key() {
    let input = "on a: hello() end";
    let result = parse_block(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == Block {
                key_list: Vec::from(["a".to_string()]),
                function_call_list: vec![FunctionCall {
                    identifier: "hello".to_string(),
                    identifier_list: None,
                    key_list: None
                }]
            }
    );
}

#[test]
fn parse_block_success_multiple_key() {
    let input = "on a or ' or #: hello() end";
    let result = parse_block(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == Block {
                key_list: Vec::from(["a".to_string(), "'".to_string(), "#".to_string()]),
                function_call_list: vec![FunctionCall {
                    identifier: "hello".to_string(),
                    identifier_list: None,
                    key_list: None
                }]
            }
    );
}

#[test]
fn parse_block_success_multiple_key_multiple_calls() {
    let input = "on a or ' or #: hello() or foo(abc) or foo_bar(abc or bee) or foo_foo(abc or bee for a or # or c) end";
    let result = parse_block(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == Block {
                key_list: Vec::from(["a".to_string(), "'".to_string(), "#".to_string()]),
                function_call_list: vec![
                    FunctionCall {
                        identifier: "hello".to_string(),
                        identifier_list: None,
                        key_list: None
                    },
                    FunctionCall {
                        identifier: "foo".to_string(),
                        identifier_list: Some(vec!["abc".to_string()]),
                        key_list: None
                    },
                    FunctionCall {
                        identifier: "foo_bar".to_string(),
                        identifier_list: Some(vec!["abc".to_string(), "bee".to_string()]),
                        key_list: None
                    },
                    FunctionCall {
                        identifier: "foo_foo".to_string(),
                        identifier_list: Some(vec!["abc".to_string(), "bee".to_string()]),
                        key_list: Some(vec!["a".to_string(), "#".to_string(), "c".to_string()])
                    }
                ]
            }
    );
}

#[test]
fn parse_program_single_block() {
    let input = "on a: hello() end";
    let result = parse_program(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == Program {
                import_list: None,
                block_list: Some(vec![Block {
                    key_list: Vec::from(["a".to_string()]),
                    function_call_list: vec![FunctionCall {
                        identifier: "hello".to_string(),
                        identifier_list: None,
                        key_list: None
                    }]
                }])
            }
    );
}

#[test]
fn parse_program_single_block_with_import() {
    let input = "import telex\non a: hello() end";
    let result = parse_program(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == Program {
                import_list: Some(vec![Import {
                    identifier: "telex".to_string()
                }]),
                block_list: Some(vec![Block {
                    key_list: Vec::from(["a".to_string()]),
                    function_call_list: vec![FunctionCall {
                        identifier: "hello".to_string(),
                        identifier_list: None,
                        key_list: None
                    }]
                }])
            }
    );
}

#[test]
fn parse_program_multiple_block() {
    let input = "on a: hello() end \n on b or c: foo() end\n\n\n\non d or e or f: bar() end";
    let result = parse_program(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == Program {
                import_list: None,
                block_list: Some(vec![
                    Block {
                        key_list: Vec::from(["a".to_string()]),
                        function_call_list: vec![FunctionCall {
                            identifier: "hello".to_string(),
                            identifier_list: None,
                            key_list: None
                        }]
                    },
                    Block {
                        key_list: Vec::from(["b".to_string(), "c".to_string()]),
                        function_call_list: vec![FunctionCall {
                            identifier: "foo".to_string(),
                            identifier_list: None,
                            key_list: None
                        }]
                    },
                    Block {
                        key_list: Vec::from(["d".to_string(), "e".to_string(), "f".to_string()]),
                        function_call_list: vec![FunctionCall {
                            identifier: "bar".to_string(),
                            identifier_list: None,
                            key_list: None
                        }]
                    }
                ])
            }
    );
}

#[test]
fn parse_program_multiple_block_with_multiple_import() {
    let input = "import telex\n\n\nimport vni on a: hello() end \n on b or c: foo() end\n\n\n\non d or e or f: bar() end";
    let result = parse_program(input);
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == Program {
                import_list: Some(vec![
                    Import {
                        identifier: "telex".to_string()
                    },
                    Import {
                        identifier: "vni".to_string()
                    }
                ]),
                block_list: Some(vec![
                    Block {
                        key_list: Vec::from(["a".to_string()]),
                        function_call_list: vec![FunctionCall {
                            identifier: "hello".to_string(),
                            identifier_list: None,
                            key_list: None
                        }]
                    },
                    Block {
                        key_list: Vec::from(["b".to_string(), "c".to_string()]),
                        function_call_list: vec![FunctionCall {
                            identifier: "foo".to_string(),
                            identifier_list: None,
                            key_list: None
                        }]
                    },
                    Block {
                        key_list: Vec::from(["d".to_string(), "e".to_string(), "f".to_string()]),
                        function_call_list: vec![FunctionCall {
                            identifier: "bar".to_string(),
                            identifier_list: None,
                            key_list: None
                        }]
                    }
                ])
            }
    );
}

#[test]
fn parse_full_program_success() {
    let input = r#"
        import telex
        import vni

        on s or ': add_tone(acute) end

        on a or e or o or 6:
          letter_mod(circumflex for a or e or o)
        end

        on w or 7 or 8:
          reset_inserted_uw() or
          letter_mod(horn or breve for u or o) or
          insert_uw()
        end
        "#;
    let result = parse_program(input);
    println!("{result:?}");
    assert!(result.is_ok());
    assert!(
        result.unwrap().1
            == Program {
                import_list: Some(vec![
                    Import {
                        identifier: "telex".to_string()
                    },
                    Import {
                        identifier: "vni".to_string()
                    }
                ]),
                block_list: Some(vec![
                    Block {
                        key_list: Vec::from(["s".to_string(), "'".to_string()]),
                        function_call_list: vec![FunctionCall {
                            identifier: "add_tone".to_string(),
                            identifier_list: Some(vec!["acute".to_string()]),
                            key_list: None
                        }]
                    },
                    Block {
                        key_list: Vec::from([
                            "a".to_string(),
                            "e".to_string(),
                            "o".to_string(),
                            "6".to_string()
                        ]),
                        function_call_list: vec![FunctionCall {
                            identifier: "letter_mod".to_string(),
                            identifier_list: Some(vec!["circumflex".to_string()]),
                            key_list: Some(vec!["a".to_string(), "e".to_string(), "o".to_string()])
                        }]
                    },
                    Block {
                        key_list: Vec::from(["w".to_string(), "7".to_string(), "8".to_string()]),
                        function_call_list: vec![
                            FunctionCall {
                                identifier: "reset_inserted_uw".to_string(),
                                identifier_list: None,
                                key_list: None
                            },
                            FunctionCall {
                                identifier: "letter_mod".to_string(),
                                identifier_list: Some(vec![
                                    "horn".to_string(),
                                    "breve".to_string()
                                ]),
                                key_list: Some(vec!["u".to_string(), "o".to_string()])
                            },
                            FunctionCall {
                                identifier: "insert_uw".to_string(),
                                identifier_list: None,
                                key_list: None
                            }
                        ]
                    }
                ])
            }
    );
}
