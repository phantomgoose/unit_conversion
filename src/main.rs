/*
 * example facts:
 * m = 3.28 ft
 * ft = 12 in
 * hr = 60 min
 * min = 60 sec
 *
 * example queries:
 * 2 m = ? in --> answer = 78.72
 * 13 in = ? m --> answer = 0.330 (roughly)
 * 13 in = ? hr --> "not convertible!"
 */

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Default, Clone)]
struct Edge {
    conversion_rate: f32,
    to: Arc<RefCell<Node>>,
}

#[derive(Default)]
struct Node {
    unit: String,
    edges: Vec<Edge>,
}

#[derive(Debug, PartialEq)]
struct ConvertedUnit(Option<f32>);

impl ToString for ConvertedUnit {
    fn to_string(&self) -> String {
        if let Some(value) = self.0 {
            format!("answer = {}", value)
        } else {
            "not convertible!".to_string()
        }
    }
}

struct ConversionGraph {
    // a map of units to respective entrypoint nodes for O(1) lookups
    node_map: HashMap<String, Arc<RefCell<Node>>>,
}

impl ConversionGraph {
    fn new(facts: Vec<Conversion>) -> Self {
        let mut node_map: HashMap<String, Arc<RefCell<Node>>> = HashMap::new();

        facts.iter().for_each(|fact| {
            node_map
                .entry(fact.to.clone())
                .or_insert(Arc::new(RefCell::new(Node {
                    unit: fact.to.clone(),
                    edges: Vec::default(),
                })));

            node_map
                .entry(fact.from.clone())
                .or_insert(Arc::new(RefCell::new(Node {
                    unit: fact.from.clone(),
                    edges: Vec::default(),
                })));
        });

        facts.iter().for_each(|fact| {
            if let (Some(origin), Some(destination)) =
                (node_map.get(&fact.from), node_map.get(&fact.to))
            {
                destination.borrow_mut().edges.push(Edge {
                    conversion_rate: 1.0 / fact.value,
                    to: origin.clone(),
                });

                origin.borrow_mut().edges.push(Edge {
                    conversion_rate: fact.value,
                    to: destination.clone(),
                });
            }
        });

        ConversionGraph { node_map }
    }

    fn traverse(
        curr_node: Option<&Arc<RefCell<Node>>>,
        search_string: &str,
        path: Vec<Edge>,
        seen_units: &mut HashSet<String>,
    ) -> Option<Vec<Edge>> {
        if curr_node?.borrow().unit == search_string {
            // found target
            return Some(path);
        }

        if curr_node?.borrow().edges.is_empty() {
            // target does not exist in this branch
            return None;
        }

        let curr_unit = &*curr_node?.borrow().unit;
        if seen_units.contains(curr_unit) {
            // cycle
            return None;
        }

        seen_units.insert(curr_unit.to_string());

        for edge in &curr_node?.borrow().edges {
            let mut new_result = path.clone();
            new_result.push(edge.clone());
            if let Some(possible_path) =
                Self::traverse(Some(&edge.to), search_string, new_result, seen_units)
            {
                return Some(possible_path);
            }
        }

        None
    }

    fn convert(&self, query: Conversion) -> ConvertedUnit {
        let from_node = self.node_map.get(query.from.as_str());
        if let Some(path) = Self::traverse(
            from_node,
            query.to.as_str(),
            Vec::new(),
            &mut HashSet::new(),
        ) {
            let res = path
                .iter()
                .fold(query.value, |acc, edge| acc * edge.conversion_rate);
            ConvertedUnit(Some(res))
        } else {
            ConvertedUnit(None)
        }
    }
}

#[derive(Debug)]
struct Conversion {
    from: String,
    to: String,
    value: f32,
}

impl Conversion {
    fn new(from: String, to: String, value: f32) -> Self {
        Conversion { from, to, value }
    }
}

fn main() {
    let graph = ConversionGraph::new(vec![
        Conversion::new("m".to_string(), "ft".to_string(), 3.28),
        Conversion::new("ft".to_string(), "in".to_string(), 12.0),
        Conversion::new("hr".to_string(), "min".to_string(), 60.0),
        Conversion::new("min".to_string(), "sec".to_string(), 60.0),
    ]);
    let res = graph
        .convert(Conversion::new("m".to_string(), "in".to_string(), 2.0))
        .to_string();
    dbg!(res);
}

#[cfg(test)]
mod tests {
    mod convert {
        use crate::{Conversion, ConversionGraph, ConvertedUnit};
        use approx::assert_relative_eq;

        fn create_test_graph() -> ConversionGraph {
            ConversionGraph::new(vec![
                Conversion::new("m".to_string(), "ft".to_string(), 3.28),
                Conversion::new("ft".to_string(), "in".to_string(), 12.0),
                Conversion::new("hr".to_string(), "min".to_string(), 60.0),
                Conversion::new("min".to_string(), "sec".to_string(), 60.0),
            ])
        }

        #[test]
        fn it_works_for_m_to_in() {
            let graph = create_test_graph();
            let res = graph.convert(Conversion::new("m".to_string(), "in".to_string(), 2.0));

            assert_relative_eq!(res.0.unwrap(), 78.72);
        }

        #[test]
        fn it_works_for_in_to_m() {
            let graph = create_test_graph();
            let res = graph.convert(Conversion::new("in".to_string(), "m".to_string(), 13.0));

            assert_relative_eq!(res.0.unwrap(), 0.33028457);
        }

        #[test]
        fn it_works_for_sec_to_hr() {
            let graph = create_test_graph();
            let res = graph.convert(Conversion::new("sec".to_string(), "hr".to_string(), 3600.0));

            assert_relative_eq!(res.0.unwrap(), 1.0);
        }

        #[test]
        fn it_correctly_does_not_work_for_in_to_h() {
            let graph = create_test_graph();
            let res = graph.convert(Conversion::new("in".to_string(), "hr".to_string(), 13.0));

            assert_eq!(res, ConvertedUnit(None));
        }
    }
}
