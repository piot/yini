/*
 * Copyright (c) Peter Bjorklund. All rights reserved. https://github.com/piot/yini
 * Licensed under the MIT License. See LICENSE in the project root for license information.
 */

use yini::{ErrorKind, Parser, Value};

#[test]
fn parse_sample() {
    let data = r#"
            # comment line
            key1: -42
            "key2": 100
            key3: 3.14
            key4: -0.5
            key5: "string"
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
            tflag: true
            fflag: false
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
            Parent: {
                child1: 1
                child2: 2
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
            level1: {
                Level2 { # intentionally without :
                    key: 42
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
            a: 10 # inline comment
            # another comment
            b: 20
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
            numbers: [1, 2, 3, 4, 5]
            mixed: ["hello", 42, true, 3.14]
            spaced: [This, Is, "A list", 23]
        "#;
    let mut parser = Parser::new(data);
    let map = parser.parse();
    assert!(
        parser.errors().is_empty(),
        "Parse errors: {:?}",
        parser.errors()
    );

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
fn array_with_tuples() {
    let data = r#"
            pairs: [
                "key1" "value",
                "another" "another_value",
                "mixed" 42,
            ]
        "#;
    let mut parser = Parser::new(data);
    let map = parser.parse();
    assert!(
        parser.errors().is_empty(),
        "Parse errors: {:?}",
        parser.errors()
    );

    if let Some(Value::Array(pairs)) = map.get("pairs") {
        assert_eq!(pairs.len(), 3);

        if let Some(items) = pairs[0].as_tuple() {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].as_str(), Some("key1"));
            assert_eq!(items[1].as_str(), Some("value"));
        } else {
            panic!("First element not parsed as tuple");
        }

        if let Some(items) = pairs[1].as_tuple() {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].as_str(), Some("another"));
            assert_eq!(items[1].as_str(), Some("another_value"));
        } else {
            panic!("Second element not parsed as tuple");
        }

        if let Some(items) = pairs[2].as_tuple() {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].as_str(), Some("mixed"));
            assert_eq!(items[1].as_int(), Some(42));
        } else {
            panic!("Third element not parsed as tuple");
        }
    } else {
        panic!("pairs not parsed as array");
    }
}

#[test]
fn array_with_objects() {
    let data = r#"
            people: [
                {
                    name: "Alice"
                    age: 30
                },
                {
                    name: "Bob"
                    age: 25
                }
            ]
            mixed_array: [
                "simple string",
                42,
                {
                    nested_key: "nested_value"
                    inner_array: [1, 2, 3]
                }
            ]
        "#;
    let mut parser = Parser::new(data);
    let map = parser.parse();
    assert!(
        parser.errors().is_empty(),
        "Parse errors: {:?}",
        parser.errors()
    );

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
            assert_eq!(
                nested.get("nested_key").and_then(Value::as_str),
                Some("nested_value")
            );
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
            key1: 42
            key2:
            "broken value"
            key3: 99
        "#;
    let mut parser = Parser::new(data);
    let _map = parser.parse();
    assert!(!parser.errors().is_empty(), "Should have parsing errors");

    // Should have an error about missing value on same line
    let errors = parser.errors();
    assert!(
        errors
            .iter()
            .any(|e| matches!(e.kind, ErrorKind::ExpectedValueOnSameLine))
    );
}

#[test]
fn error_missing_newline_after_value() {
    let data = r"key1: 42 key2: 99";
    let mut parser = Parser::new(data);
    let _map = parser.parse();
    // Under the new rule, unquoted rest-of-line strings are allowed for object fields,
    // so this should parse without errors.
    assert!(
        parser.errors().is_empty(),
        "Should have no parsing errors: {:?}",
        parser.errors()
    );
}

#[test]
fn valid_formatting_no_errors() {
    let data = r#"
            key1: 42
            key2: "value with spaces"
            key3: true # comment is OK
            key4: 3.14 # another comment style
        "#;
    let mut parser = Parser::new(data);
    let _map = parser.parse();
    assert!(
        parser.errors().is_empty(),
        "Should have no parsing errors: {:?}",
        parser.errors()
    );
}

#[test]
fn error_missing_colon() {
    let data = r#"
            key1 42
            key2: 99
        "#;
    let mut parser = Parser::new(data);
    let _map = parser.parse();
    assert!(!parser.errors().is_empty(), "Should have parsing errors");

    // Should have an error about missing colon
    let errors = parser.errors();
    assert!(
        errors
            .iter()
            .any(|e| matches!(e.kind, ErrorKind::ExpectedColonAfterKey)),
        "Expected ExpectedColonAfterKey error, got: {:?}",
        errors
    );
}

#[test]
fn tuples_in_object_values() {
    let data = r#"
            pair: ("key", "value")
            triple: ("a", "b", "c")
            quad: (1, 2, 3, 4)
            mixed: (Alice, 30, true)
        "#;
    let mut parser = Parser::new(data);
    let map = parser.parse();

    assert!(
        parser.errors().is_empty(),
        "Parse errors: {:?}",
        parser.errors()
    );

    // 2-tuple
    if let Some(items) = map.get("pair").and_then(Value::as_tuple) {
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].as_str(), Some("key"));
        assert_eq!(items[1].as_str(), Some("value"));
    } else {
        panic!("pair not parsed as 2-tuple");
    }

    // 3-tuple
    if let Some(items) = map.get("triple").and_then(Value::as_tuple) {
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].as_str(), Some("a"));
        assert_eq!(items[1].as_str(), Some("b"));
        assert_eq!(items[2].as_str(), Some("c"));
    } else {
        panic!("triple not parsed as 3-tuple");
    }

    // 4-tuple
    if let Some(items) = map.get("quad").and_then(Value::as_tuple) {
        assert_eq!(items.len(), 4);
        assert_eq!(items[0].as_int(), Some(1));
        assert_eq!(items[1].as_int(), Some(2));
        assert_eq!(items[2].as_int(), Some(3));
        assert_eq!(items[3].as_int(), Some(4));
    } else {
        panic!("quad not parsed as 4-tuple");
    }

    // Mixed types
    if let Some(items) = map.get("mixed").and_then(Value::as_tuple) {
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].as_str(), Some("Alice"));
        assert_eq!(items[1].as_int(), Some(30));
        assert_eq!(items[2].as_bool(), Some(true));
    } else {
        panic!("mixed not parsed as 3-tuple");
    }
}

#[test]
fn array_with_three_item_tuples() {
    let data = r#"
            triples [ # intentionally without :
                "a" "b" "c",
                1 2 3,
            ]
        "#;
    let mut parser = Parser::new(data);
    let map = parser.parse();

    assert!(
        parser.errors().is_empty(),
        "Parse errors: {:?}",
        parser.errors()
    );

    if let Some(Value::Array(triples)) = map.get("triples") {
        assert_eq!(triples.len(), 2);

        // First triple: "a" "b" "c"
        if let Some(items) = triples[0].as_tuple() {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0].as_str(), Some("a"));
            assert_eq!(items[1].as_str(), Some("b"));
            assert_eq!(items[2].as_str(), Some("c"));
        } else {
            panic!("First element not parsed as 3-tuple");
        }

        // Second triple: 1 2 3
        if let Some(items) = triples[1].as_tuple() {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0].as_int(), Some(1));
            assert_eq!(items[1].as_int(), Some(2));
            assert_eq!(items[2].as_int(), Some(3));
        } else {
            panic!("Second element not parsed as 3-tuple");
        }
    } else {
        panic!("triples not parsed as array");
    }
}

#[test]
fn unquoted_strings() {
    let data = r#"
            description: this is a very long description
            tuple: (this is a long description inside a tuple, another)
        "#;

    let mut parser = Parser::new(data);
    let map = parser.parse();
    assert!(
        parser.errors().is_empty(),
        "Parse errors: {:?}",
        parser.errors()
    );

    assert_eq!(
        map.get("description").and_then(Value::as_str),
        Some("this is a very long description")
    );

    if let Some(Value::Tuple(items)) = map.get("tuple") {
        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].as_str(),
            Some("this is a long description inside a tuple")
        );
        assert_eq!(items[1].as_str(), Some("another"));
    } else {
        panic!("tuple not parsed as tuple");
    }
}

#[test]
fn variants() {
    let data = r#"
            mode: :Fullscreen
            window_mode: :Windowed
            style: :Borderless
            mixed: lowercase
            lowercase_variant: :fullscreen
            snake_case_variant: :window_mode
            tuple_with_variant: (:Player, 100, :Active)
            array_of_variants: [:North, :South, :East, :West]
            mixed_case_array: [:fullscreen, :windowed, :borderless]
        "#;

    let mut parser = Parser::new(data);
    let map = parser.parse();
    assert!(
        parser.errors().is_empty(),
        "Parse errors: {:?}",
        parser.errors()
    );

    // Colon-prefixed identifiers = Variant (uppercase)
    assert_eq!(
        map.get("mode").and_then(Value::as_variant),
        Some("Fullscreen")
    );
    assert_eq!(
        map.get("window_mode").and_then(Value::as_variant),
        Some("Windowed")
    );
    assert_eq!(
        map.get("style").and_then(Value::as_variant),
        Some("Borderless")
    );

    // Lowercase identifier without colon = Str
    assert_eq!(map.get("mixed").and_then(Value::as_str), Some("lowercase"));

    // Colon-prefixed identifiers = Variant (lowercase works too!)
    assert_eq!(
        map.get("lowercase_variant").and_then(Value::as_variant),
        Some("fullscreen")
    );
    assert_eq!(
        map.get("snake_case_variant").and_then(Value::as_variant),
        Some("window_mode")
    );

    // Variants in tuples
    if let Some(Value::Tuple(items)) = map.get("tuple_with_variant") {
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].as_variant(), Some("Player"));
        assert_eq!(items[1].as_int(), Some(100));
        assert_eq!(items[2].as_variant(), Some("Active"));
    } else {
        panic!("tuple_with_variant not parsed as tuple");
    }

    // Variants in arrays (uppercase)
    if let Some(Value::Array(arr)) = map.get("array_of_variants") {
        assert_eq!(arr.len(), 4);
        assert_eq!(arr[0].as_variant(), Some("North"));
        assert_eq!(arr[1].as_variant(), Some("South"));
        assert_eq!(arr[2].as_variant(), Some("East"));
        assert_eq!(arr[3].as_variant(), Some("West"));
    } else {
        panic!("array_of_variants not parsed as array");
    }

    // Variants in arrays (lowercase)
    if let Some(Value::Array(arr)) = map.get("mixed_case_array") {
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_variant(), Some("fullscreen"));
        assert_eq!(arr[1].as_variant(), Some("windowed"));
        assert_eq!(arr[2].as_variant(), Some("borderless"));
    } else {
        panic!("mixed_case_array not parsed as array");
    }
}

#[test]
fn variants_with_payloads() {
    let data = r#"
            simple: :fullscreen
            with_tuple: :windowed (768, 1024)
            with_single: :borderless (true)
            with_object: :player {
                name: "Alice"
                hp: 100
            }
            with_array: :colors [255, 128, 0]
            empty_tuple: :empty ()
            array_of_payloads: [
                :ok (42),
                :error ("failed"),
                :pending
            ]
        "#;

    let mut parser = Parser::new(data);
    let map = parser.parse();
    assert!(
        parser.errors().is_empty(),
        "Parse errors: {:?}",
        parser.errors()
    );

    // Simple variant without payload
    if let Some((name, payload)) = map.get("simple").and_then(Value::as_variant_with_payload) {
        assert_eq!(name, "fullscreen");
        assert!(payload.is_none());
    } else {
        panic!("simple not parsed as variant");
    }

    // Variant with tuple payload (multiple values in parens)
    if let Some((name, payload)) = map
        .get("with_tuple")
        .and_then(Value::as_variant_with_payload)
    {
        assert_eq!(name, "windowed");
        if let Some(Value::Tuple(items)) = payload {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].as_int(), Some(768));
            assert_eq!(items[1].as_int(), Some(1024));
        } else {
            panic!("with_tuple payload not parsed as tuple, got: {:?}", payload);
        }
    } else {
        panic!("with_tuple not parsed as variant");
    }

    // Variant with single value payload (wrapped in tuple)
    if let Some((name, payload)) = map
        .get("with_single")
        .and_then(Value::as_variant_with_payload)
    {
        assert_eq!(name, "borderless");
        if let Some(Value::Tuple(items)) = payload {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].as_bool(), Some(true));
        } else {
            panic!("with_single should have tuple payload");
        }
    } else {
        panic!("with_single not parsed as variant");
    }

    // Variant with object payload
    if let Some((name, payload)) = map
        .get("with_object")
        .and_then(Value::as_variant_with_payload)
    {
        assert_eq!(name, "player");
        if let Some(Value::Object(obj)) = payload {
            assert_eq!(obj.get("name").and_then(Value::as_str), Some("Alice"));
            assert_eq!(obj.get("hp").and_then(Value::as_int), Some(100));
        } else {
            panic!("with_object payload not parsed as object");
        }
    } else {
        panic!("with_object not parsed as variant");
    }

    // Variant with array payload
    if let Some((name, payload)) = map
        .get("with_array")
        .and_then(Value::as_variant_with_payload)
    {
        assert_eq!(name, "colors");
        if let Some(Value::Array(arr)) = payload {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0].as_int(), Some(255));
            assert_eq!(arr[1].as_int(), Some(128));
            assert_eq!(arr[2].as_int(), Some(0));
        } else {
            panic!("with_array payload not parsed as array");
        }
    } else {
        panic!("with_array not parsed as variant");
    }

    // Empty payload
    if let Some((name, payload)) = map
        .get("empty_tuple")
        .and_then(Value::as_variant_with_payload)
    {
        assert_eq!(name, "empty");
        if let Some(Value::Tuple(items)) = payload {
            assert_eq!(items.len(), 0);
        } else {
            panic!("empty_tuple should have empty tuple payload");
        }
    } else {
        panic!("empty_tuple not parsed as variant");
    }

    // Array containing variants with payloads
    if let Some(Value::Array(arr)) = map.get("array_of_payloads") {
        assert_eq!(arr.len(), 3);

        // :ok (42)
        if let Some((name, payload)) = arr[0].as_variant_with_payload() {
            assert_eq!(name, "ok");
            if let Some(Value::Tuple(items)) = payload {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].as_int(), Some(42));
            } else {
                panic!("First array element payload should be tuple");
            }
        } else {
            panic!("First array element not parsed as variant");
        }

        // :error ("failed")
        if let Some((name, payload)) = arr[1].as_variant_with_payload() {
            assert_eq!(name, "error");
            if let Some(Value::Tuple(items)) = payload {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].as_str(), Some("failed"));
            } else {
                panic!("Second array element payload should be tuple");
            }
        } else {
            panic!("Second array element not parsed as variant");
        }

        // :pending (no payload)
        if let Some((name, payload)) = arr[2].as_variant_with_payload() {
            assert_eq!(name, "pending");
            assert!(payload.is_none());
        } else {
            panic!("Third array element not parsed as variant");
        }
    } else {
        panic!("array_of_payloads not parsed as array");
    }
}
