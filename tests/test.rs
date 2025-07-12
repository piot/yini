/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/piot/yini
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

use yini::{Parser, Value};

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
    let data = r#"
            tflag true
            fflag false
        "#;
    let mut parser = Parser::new(data);
    let map = parser.parse();
    assert!(parser.errors().is_empty());
        assert_eq!(map.get("tflag").and_then(Value::as_bool), Some(true));
        assert_eq!(map.get("fflag").and_then(Value::as_bool), Some(false));
}

#[test]
fn object() {
    let data = r#"
            Parent {
                child1 1
                child2 2
            }
        "#;
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
    let data = r#"
            level1 {
                Level2 {
                    key 42
                }
            }
        "#;
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
    let data = r#"
            // full line comment
            a 10 // inline comment
            # another comment
            b 20
        "#;
    let mut parser = Parser::new(data);
    let map = parser.parse();
    assert!(parser.errors().is_empty());
    assert_eq!(map.get("a").and_then(Value::as_int), Some(10));
    assert_eq!(map.get("b").and_then(Value::as_int), Some(20));
}
