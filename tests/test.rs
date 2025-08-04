/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/piot/yini
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

use yini::{Parser, Value, ErrorKind};

#[test]
fn parse_sample() {
    let data = r#"
            # comment line
            key1 -42
            "key2" 100
            key3 3.14
            key4 -0.5
            key5 "string"
        "#;

    let mut parser = Parser::new(data);
    let map = parser.parse();
    assert!(
        parser.errors().is_empty(),
        "Parse errors: {:?}",
        parser.errors()
    );
    assert_eq!(map.get("key1").and_then(Value::as_int), Some(-42));
    assert_eq!(map.get("key2").and_then(Value::as_int), Some(100));
    assert_eq!(map.get("key3").and_then(Value::as_num), Some(3.14));
    assert_eq!(map.get("key4").and_then(Value::as_num), Some(-0.5));
    assert_eq!(map.get("key5").and_then(Value::as_str), Some("string"));
}

#[test]
fn booleans() {
    let data = r"
            tflag true
            fflag false
        ";
    let mut parser = Parser::new(data);
    let map = parser.parse();
    assert!(parser.errors().is_empty());
        assert_eq!(map.get("tflag").and_then(Value::as_bool), Some(true));
        assert_eq!(map.get("fflag").and_then(Value::as_bool), Some(false));
}

#[test]
fn object() {
    let data = r"
            Parent {
                child1 1
                child2 2
            }
        ";
    let mut parser = Parser::new(data);
    let map = parser.parse();
    assert!(parser.errors().is_empty());
    if let Some(Value::Object(child)) = map.get("Parent") {
        assert_eq!(child.get("child1").and_then(Value::as_int), Some(1));
        assert_eq!(child.get("child2").and_then(Value::as_int), Some(2));
    } else {
        panic!("Parent not parsed as object");
    }
}

#[test]
fn parse_two_nested() {
    let data = r"
            level1 {
                Level2 {
                    key 42
                }
            }
        ";
    let mut parser = Parser::new(data);
    let map = parser.parse();
    assert!(
        parser.errors().is_empty(),
        "Parse errors: {:?}",
        parser.errors()
    );

    if let Some(Value::Object(level1)) = map.get("level1") {
        if let Some(Value::Object(level2)) = level1.get("Level2") {
            assert_eq!(level2.get("key").and_then(Value::as_int), Some(42));
        } else {
            panic!("Level2 not parsed as object");
        }
    } else {
        panic!("Level1 not parsed as object");
    }
}

#[test]
fn comments() {
    let data = r"
            # full line comment
            a 10 # inline comment
            # another comment
            b 20
        ";
    let mut parser = Parser::new(data);
    let map = parser.parse();
    assert!(parser.errors().is_empty());
    assert_eq!(map.get("a").and_then(Value::as_int), Some(10));
    assert_eq!(map.get("b").and_then(Value::as_int), Some(20));
}

#[test]
fn flat_array() {
    let data = r#"
            numbers [1, 2, 3, 4, 5]
            mixed ["hello", 42, true, 3.14]
            spaced [This Is "A list" 23]
        "#;
    let mut parser = Parser::new(data);
    let map = parser.parse();
    assert!(parser.errors().is_empty(), "Parse errors: {:?}", parser.errors());

    if let Some(Value::Array(numbers)) = map.get("numbers") {
        assert_eq!(numbers.len(), 5);
        assert_eq!(numbers[0].as_int(), Some(1));
        assert_eq!(numbers[4].as_int(), Some(5));
    } else {
        panic!("numbers not parsed as array");
    }

    if let Some(Value::Array(mixed)) = map.get("mixed") {
        assert_eq!(mixed.len(), 4);
        assert_eq!(mixed[0].as_str(), Some("hello"));
        assert_eq!(mixed[1].as_int(), Some(42));
        assert_eq!(mixed[2].as_bool(), Some(true));
        assert_eq!(mixed[3].as_num(), Some(3.14));
    } else {
        panic!("mixed not parsed as array");
    }

    if let Some(Value::Array(spaced)) = map.get("spaced") {
        assert_eq!(spaced.len(), 4);
        assert_eq!(spaced[0].as_str(), Some("This"));
        assert_eq!(spaced[1].as_str(), Some("Is"));
        assert_eq!(spaced[2].as_str(), Some("A list"));
        assert_eq!(spaced[3].as_int(), Some(23));
    } else {
        panic!("spaced not parsed as array");
    }
}

#[test]
fn array_with_objects() {
    let data = r#"
            people [
                {
                    name "Alice"
                    age 30
                },
                {
                    name "Bob"
                    age 25
                }
            ]
            mixed_array [
                "simple string",
                42,
                {
                    nested_key "nested_value"
                    inner_array [1, 2, 3]
                }
            ]
        "#;
    let mut parser = Parser::new(data);
    let map = parser.parse();
    assert!(parser.errors().is_empty(), "Parse errors: {:?}", parser.errors());

    if let Some(Value::Array(people)) = map.get("people") {
        assert_eq!(people.len(), 2);

        if let Some(alice) = people[0].as_object() {
            assert_eq!(alice.get("name").and_then(Value::as_str), Some("Alice"));
            assert_eq!(alice.get("age").and_then(Value::as_int), Some(30));
        } else {
            panic!("First person not parsed as object");
        }

        if let Some(bob) = people[1].as_object() {
            assert_eq!(bob.get("name").and_then(Value::as_str), Some("Bob"));
            assert_eq!(bob.get("age").and_then(Value::as_int), Some(25));
        } else {
            panic!("Second person not parsed as object");
        }
    } else {
        panic!("people not parsed as array");
    }

    if let Some(Value::Array(mixed)) = map.get("mixed_array") {
        assert_eq!(mixed.len(), 3);
        assert_eq!(mixed[0].as_str(), Some("simple string"));
        assert_eq!(mixed[1].as_int(), Some(42));

        if let Some(nested) = mixed[2].as_object() {
            assert_eq!(nested.get("nested_key").and_then(Value::as_str), Some("nested_value"));
            if let Some(Value::Array(inner)) = nested.get("inner_array") {
                assert_eq!(inner.len(), 3);
                assert_eq!(inner[0].as_int(), Some(1));
                assert_eq!(inner[2].as_int(), Some(3));
            } else {
                panic!("inner_array not parsed as array");
            }
        } else {
            panic!("Third element not parsed as object");
        }
    } else {
        panic!("mixed_array not parsed as array");
    }
}

#[test]
fn error_line_break_between_key_value() {
    let data = r#"
            key1 42
            key2
            "broken value"
            key3 99
        "#;
    let mut parser = Parser::new(data);
    let _map = parser.parse();
    assert!(!parser.errors().is_empty(), "Should have parsing errors");

    // Should have an error about missing value on same line
    let errors = parser.errors();
    assert!(errors.iter().any(|e| matches!(e.kind, ErrorKind::ExpectedValueOnSameLine)));
}

#[test]
fn error_missing_newline_after_value() {
    let data = r"key1 42 key2 99";
    let mut parser = Parser::new(data);
    let _map = parser.parse();
    assert!(!parser.errors().is_empty(), "Should have parsing errors");

    // Should have an error about expected newline
    let errors = parser.errors();
    assert!(errors.iter().any(|e| matches!(e.kind, ErrorKind::ExpectedNewlineAfterKeyValue)));
}

#[test]
fn error_extra_content_after_value() {
    let data = r"
            key1 42 extra stuff
            key2 99
        ";
    let mut parser = Parser::new(data);
    let _map = parser.parse();
    assert!(!parser.errors().is_empty(), "Should have parsing errors");

    // Should have an error about expected newline
    let errors = parser.errors();
    assert!(errors.iter().any(|e| matches!(e.kind, ErrorKind::ExpectedNewlineAfterKeyValue)));
}

#[test]
fn valid_formatting_no_errors() {
    let data = r#"
            key1 42
            key2 "value with spaces"
            key3 true # comment is OK
            key4 3.14 # another comment style
        "#;
    let mut parser = Parser::new(data);
    let _map = parser.parse();
    assert!(parser.errors().is_empty(), "Should have no parsing errors: {:?}", parser.errors());
}
