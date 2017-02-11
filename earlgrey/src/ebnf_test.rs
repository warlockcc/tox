use lexers::DelimTokenizer;
use ebnf::{ebnf_grammar, ParserBuilder};
use trees::all_trees;

#[test]
fn build_ebnf_grammar() {
    ebnf_grammar();
}

#[test]
fn test_minimal_parser() {
    let g = r#" Number := "0" ; "#;
    let p = ParserBuilder::new(&g).into_parser("Number");
    let mut tok = DelimTokenizer::from_str("0", " ", true);
    let state = p.parse(&mut tok).unwrap();
    let trees = all_trees(p.g.start(), &state);
    assert_eq!(format!("{:?}", trees),
               r#"[Node("Number -> 0", [Leaf("0", "0")])]"#);
}

#[test]
fn test_arith_parser() {
    let g = r#"
        expr := Number
              | expr "+" Number ;

        Number := "0" | "1" | "2" | "3" ;
    "#;
    let p = ParserBuilder::new(&g).into_parser("expr");
    let mut tok = DelimTokenizer::from_str("3 + 2 + 1", " ", true);
    let state = p.parse(&mut tok).unwrap();
    let trees = all_trees(p.g.start(), &state);
    assert_eq!(format!("{:?}", trees),
               r#"[Node("expr -> expr + Number", [Node("expr -> expr + Number", [Node("expr -> Number", [Node("Number -> 3", [Leaf("3", "3")])]), Leaf("+", "+"), Node("Number -> 2", [Leaf("2", "2")])]), Leaf("+", "+"), Node("Number -> 1", [Leaf("1", "1")])])]"#);
}

#[test]
fn test_repetition() {
    let g = r#"
        arg := b { "," b } ;
        b := "0" | "1" ;
    "#;
    let p = ParserBuilder::new(&g).into_parser("arg");
    let mut tok = DelimTokenizer::from_str("1 , 0 , 1", " ", true);
    let state = p.parse(&mut tok).unwrap();
    let trees = all_trees(p.g.start(), &state);
    assert_eq!(format!("{:?}", trees),
               r#"[Node("arg -> b <Uniq-2>", [Node("b -> 1", [Leaf("1", "1")]), Node("<Uniq-2> -> , b <Uniq-2>", [Leaf(",", ","), Node("b -> 0", [Leaf("0", "0")]), Node("<Uniq-2> -> , b <Uniq-2>", [Leaf(",", ","), Node("b -> 1", [Leaf("1", "1")]), Node("<Uniq-2> -> ", [])])])])]"#);
}

#[test]
fn test_option() {
    let g = r#"
        complex := d [ "i" ];
        d := "0" | "1" | "2";
    "#;
    let p = ParserBuilder::new(&g).into_parser("complex");
    let mut tok = DelimTokenizer::from_str("1", " ", true);
    let state = p.parse(&mut tok).unwrap();
    let trees = all_trees(p.g.start(), &state);
    assert_eq!(format!("{:?}", trees),
               r#"[Node("complex -> d <Uniq-2>", [Node("d -> 1", [Leaf("1", "1")]), Node("<Uniq-2> -> ", [])])]"#);
    let mut tok = DelimTokenizer::from_str("2 i", " ", true);
    assert!(p.parse(&mut tok).is_ok());
}

#[test]
fn test_group() {
    let g = r#"
        row := ("a" | "b") ("0" | "1") ;
    "#;
    let p = ParserBuilder::new(&g).into_parser("row");
    let mut tok = DelimTokenizer::from_str("b 1", " ", true);
    let state = p.parse(&mut tok).unwrap();
    let trees = all_trees(p.g.start(), &state);
    assert_eq!(format!("{:?}", trees),
               r#"[Node("row -> <Uniq-1> <Uniq-4>", [Node("<Uniq-1> -> b", [Leaf("b", "b")]), Node("<Uniq-4> -> 1", [Leaf("1", "1")])])]"#);
    let mut tok = DelimTokenizer::from_str("a 0", " ", true);
    assert!(p.parse(&mut tok).is_ok());
}
