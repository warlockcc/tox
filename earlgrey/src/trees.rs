#![deny(warnings)]

use crate::spans::{Span, SpanSource};
use crate::parser::ParseTrees;
use std::collections::HashMap;
use std::rc::Rc;


// Semantic actions to execute when walking the tree
type SemAction<'a, ASTNode> = Box<dyn Fn(Vec<ASTNode>) -> ASTNode + 'a>;
// Given a Rule and a Token build an ASTNode
type LeafBuilder<'a, ASTNode> = Box<dyn Fn(&str, &str) -> ASTNode + 'a>;

// type TerminalParser<'a, ASTNode> = Box<dyn Fn(&str, &str) -> ASTNode + 'a>;

pub struct EarleyForest<'a, ASTNode: Clone> {
    actions: HashMap<String, SemAction<'a, ASTNode>>,
    leaf_builder: LeafBuilder<'a, ASTNode>,
}

impl<'a, ASTNode: Clone> EarleyForest<'a, ASTNode> {
    pub fn new<Builder>(leaf_builder: Builder) -> Self
            where Builder: Fn(&str, &str) -> ASTNode + 'a {
        EarleyForest{
            actions: HashMap::new(),
            leaf_builder: Box::new(leaf_builder)}
    }

    // Register semantic actions to act when rules are matched
    pub fn action<Action>(&mut self, rule: &str, action: Action)
            where Action: Fn(Vec<ASTNode>) -> ASTNode + 'a {
        self.actions.insert(rule.to_string(), Box::new(action));
    }
}

impl<'a, ASTNode: Clone> EarleyForest<'a, ASTNode> {
    fn reduce(&self, root: &Rc<Span>, args: Vec<ASTNode>)
            -> Result<Vec<ASTNode>, String> {
        // If span is not complete, reduce is a noop passthrough
        if !root.complete() { return Ok(args) }
        // Lookup semantic action to apply based on rule name
        let rulename = root.rule.to_string();
        match self.actions.get(&rulename) {
            None => Err(format!("Missing Action: {}", rulename)),
            Some(action) => {
                if cfg!(feature="debug") {
                    eprintln!("Reduction: {}", rulename);
                }
                Ok(vec![action(args)])
            }
        }
    }
}

impl<'a, ASTNode: Clone> EarleyForest<'a, ASTNode> {

    // To write this helper draw a tree of the backpointers and see how they link.
    // - If a span has no sources then its rule progress is at the start.
    // - If it originates from a 'completion' there's a span (the source)
    // that was extended because another (the trigger) completed its rule.
    // Recurse both spans transitively until they have no sources to follow.
    // They will return the 'scans' that happened along the way.
    // - If a span originates from a 'scan' then lift the text into an ASTNode.
    fn walker(&self, root: &Rc<Span>) -> Result<Vec<ASTNode>, String> {
        let mut args = Vec::new();
        match root.sources().iter().next() {
            Some(SpanSource::Completion(source, trigger)) => {
                args.extend(self.walker(source)?);
                args.extend(self.walker(trigger)?);
            },
            Some(SpanSource::Scan(source, trigger)) => {
                let symbol = source.next_symbol()
                    .expect("BUG: missing scan trigger symbol").name();
                args.extend(self.walker(source)?);
                args.push((self.leaf_builder)(symbol, trigger));
            },
            None => (),
        }
        self.reduce(root, args)
    }

    // for non-ambiguous grammars this retreieves the only possible parse
    pub fn eval_recursive(&self, ptrees: &ParseTrees) -> Result<ASTNode, String> {
        // walker will always return a Vec of size 1 because root.complete
        Ok(self.walker(ptrees.0.first().expect("BUG: ParseTrees empty"))?
           .swap_remove(0))
    }
}

impl<'a, ASTNode: Clone> EarleyForest<'a, ASTNode> {
    /*
    ## S -> S + N | N
    ## N -> [0-9]
    ## "1 + 2"

                 S -> S + N. 
                    /  \
                   /    \
              S +.N     N -> [0-9].
               / \             / \
              /   \           /   \
           S.+ N   "+"    .[0-9]   "2"
             /\
            /  \
       .S + N   S -> N.
                  /\
                 /  \
               .N    N -> [0-9].
                       / \
                      /   \
                  .[0-9]   "1"
    */
    // for non-ambiguous grammars this retreieves the only possible parse
    pub fn eval(&self, ptrees: &ParseTrees) -> Result<ASTNode, String> {
        let mut args = Vec::new();
        let mut completions = Vec::new();
        let mut spans = vec![ptrees.0.first().expect("BUG: ParseTrees empty").clone()];

        while let Some(cursor) = spans.pop() {
            // As Earley chart is unwound keep a record of semantic actions to apply
            if cursor.complete() {
                completions.push(cursor.clone());
            }
            // Walk the chart following span sources (back-pointers) of the tree.
            match cursor.sources().iter().next() {
                // Completion sources -> Walk the chart. 
                Some(SpanSource::Completion(source, trigger)) => {
                    spans.push(source.clone());
                    spans.push(trigger.clone());
                },
                // Scan sources -> lift scanned tokens into AST nodes.
                Some(SpanSource::Scan(source, trigger)) => {
                    let symbol = source.next_symbol()
                        .expect("BUG: missing scan trigger symbol").name();
                    args.push((self.leaf_builder)(symbol, trigger));
                    spans.push(source.clone());
                },
                // (Reachable) Spans with no sources mean we've unwound to the
                // begining of a production/rule. Apply the rule reducing args.
                None => {
                    let completed_rule = &completions.pop().expect("BUG: span rule never completed").rule;
                    assert_eq!(&cursor.rule, completed_rule);
                    // Get input AST nodes for this reduction. Stored reversed.
                    let num_rule_slots = completed_rule.spec.len();
                    let rule_args = args.split_off(args.len() - num_rule_slots).into_iter().rev().collect();
                    // Apply the reduction.
                    let rulename = completed_rule.to_string();
                    let action = self.actions.get(&rulename).ok_or(format!("Missing Action: {}", rulename))?;
                    args.push(action(rule_args));
                }
            }
        }

        assert_eq!(args.len(), 1);
        Ok(args.pop().expect("BUG: mismatched reduce args"))
    }
}


impl<'a, ASTNode: Clone> EarleyForest<'a, ASTNode> {

    fn walker_all(&self, root: &Rc<Span>) -> Result<Vec<Vec<ASTNode>>, String> {
        let source = root.sources();
        if source.len() == 0 {
            return Ok(vec![self.reduce(root, Vec::new())?]);
        }
        let mut trees = Vec::new();
        for backpointer in source.iter() {
            match backpointer {
                SpanSource::Completion(source, trigger) => {
                    // collect left-side-tree of each node
                    for args in self.walker_all(source)? {
                        // collect right-side-tree of each node
                        for trig in self.walker_all(trigger)? {
                            let mut args = args.clone();
                            args.extend(trig);
                            trees.push(self.reduce(root, args)?);
                        }
                    }
                }
                SpanSource::Scan(source, trigger) => {
                    for mut args in self.walker_all(source)? {
                        let symbol = source.next_symbol()
                            .expect("BUG: missing scan trigger symbol").name();
                        args.push((self.leaf_builder)(symbol, trigger));
                        trees.push(self.reduce(root, args)?);
                    }
                }
            }
        }
        Ok(trees)
    }

    // Retrieves all parse trees
    pub fn eval_all_recursive(&self, ptrees: &ParseTrees) -> Result<Vec<ASTNode>, String> {
        let mut trees = Vec::new();
        for root in &ptrees.0 {
            trees.extend(
                self.walker_all(root)?.into_iter()
                    .map(|mut treevec| treevec.swap_remove(0)));
        }
        Ok(trees)
    }

    // TODO: provide an estimate
    pub fn num_trees(&self) -> Option<u32> { None }
}
