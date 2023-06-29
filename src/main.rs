/*
* The goal is to create a function that allows unit conversions given an initial set of facts.
* See the following video: https://youtu.be/V8DGdPkBBxg

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
*
* For our solution, we're going to implement a graph to capture the relationships between the known Units
* with conversion rates stored as part of the edge metadata.
*/

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use lazy_static::lazy_static;

// Rust has no support for complex constant initialization (since constants are initialized at
// compile time), so we're using the lazy_static crate here to lazily init our set of supported
// units at runtime instead
lazy_static! {
    static ref VALID_UNITS: HashSet<&'static str> =
        HashSet::from(["m", "in", "hr", "ft", "min", "sec"]);
}

lazy_static! {
    static ref TEST_GRAPH: ConversionGraph = ConversionGraph::new(vec![
        Conversion::new("m", "ft", 3.28),
        Conversion::new("ft", "in", 12.0),
        Conversion::new("hr", "min", 60.0),
        Conversion::new("min", "sec", 60.0),
    ]);
}

#[derive(Default, Clone)]
struct ThreadSafeNode(Arc<RwLock<Node>>);

impl From<Unit> for ThreadSafeNode {
    fn from(unit: Unit) -> Self {
        ThreadSafeNode(Arc::new(RwLock::new(Node {
            unit,
            edges: Vec::new(),
        })))
    }
}

impl ThreadSafeNode {
    fn add_edge(&self, edge: Edge) {
        let err_msg = "Nodes should be writeable when adding edges";
        self.0.write().expect(err_msg).edges.push(edge);
    }
}

#[derive(Default, Clone)]
struct Edge {
    conversion_rate: f32,
    to: ThreadSafeNode,
}

#[derive(Default)]
struct Node {
    unit: Unit,
    edges: Vec<Edge>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
struct Unit(String);

impl From<&str> for Unit {
    fn from(value: &str) -> Self {
        assert!(
            VALID_UNITS.contains(value),
            "Received invalid unit value {}",
            value
        );

        Unit(value.to_string())
    }
}

#[derive(Debug, PartialEq)]
struct ConvertedValue(Option<f32>);

impl ToString for ConvertedValue {
    fn to_string(&self) -> String {
        if let Some(value) = self.0 {
            format!("answer = {}", value)
        } else {
            "not convertible!".to_string()
        }
    }
}

struct ConversionGraph {
    // a map of units to respective entrypoint nodes for O(1) lookups for the first node in a conversion chain
    nodes: HashMap<Unit, ThreadSafeNode>,
}

impl ConversionGraph {
    fn new(facts: Vec<Conversion>) -> Self {
        let mut node_map: HashMap<Unit, ThreadSafeNode> = HashMap::new();

        // create a map of our unit -> Node pairs
        facts.iter().for_each(|fact| {
            node_map.insert(fact.to.clone(), ThreadSafeNode::from(fact.to.clone()));

            node_map.insert(fact.from.clone(), ThreadSafeNode::from(fact.from.clone()));
        });

        // create the edges between our unit nodes to capture conversion rate information
        facts.iter().for_each(|fact| {
            if let (Some(origin), Some(destination)) =
                (node_map.get(&fact.from), node_map.get(&fact.to))
            {
                origin.add_edge(Edge {
                    conversion_rate: fact.value,
                    to: destination.clone(),
                });

                destination.add_edge(Edge {
                    // conversion rate from destination to origin is the inverse of the given rate
                    conversion_rate: 1.0 / fact.value,
                    to: origin.clone(),
                });
            }
        });

        ConversionGraph { nodes: node_map }
    }

    fn traverse(
        curr_node: Option<&ThreadSafeNode>,
        target_unit: Unit,
        path: Vec<Edge>,
        visited: &mut HashSet<Unit>,
    ) -> Option<Vec<Edge>> {
        let read_failure_msg = "read access to nodes during traversal should always be possible";

        let node = curr_node?.0.read().expect(read_failure_msg);
        if node.unit == target_unit {
            // found the target
            return Some(path);
        }

        if node.edges.is_empty() {
            // target does not exist in the current branch
            return None;
        }

        let curr_unit = node.unit.clone();
        if visited.contains(&curr_unit) {
            // detected a cycle in the current branch before getting to the target unit
            return None;
        }

        visited.insert(curr_unit);

        // TODO: remove recursion, which makes things rather confusing
        for edge in &node.edges {
            let mut updated_path = path.clone();
            updated_path.push(edge.clone());
            if let Some(possible_path) =
                Self::traverse(Some(&edge.to), target_unit.clone(), updated_path, visited)
            {
                return Some(possible_path);
            }
        }

        None
    }

    fn convert(&self, query: Conversion) -> ConvertedValue {
        let from_node = self.nodes.get(&query.from);
        if let Some(path) = Self::traverse(from_node, query.to, Vec::new(), &mut HashSet::new()) {
            let res = path
                .iter()
                .fold(query.value, |acc, edge| acc * edge.conversion_rate);
            ConvertedValue(Some(res))
        } else {
            ConvertedValue(None)
        }
    }
}

#[derive(Debug)]
struct Conversion {
    from: Unit,
    to: Unit,
    value: f32,
}

impl Conversion {
    fn new(from: &str, to: &str, value: f32) -> Self {
        Conversion {
            from: Unit::from(from),
            to: Unit::from(to),
            value,
        }
    }
}

fn main() {
    let res = TEST_GRAPH
        .convert(Conversion::new("min", "sec", 2.0))
        .to_string();
    dbg!(res);
}

#[cfg(test)]
mod tests {
    mod convert {
        use approx::assert_relative_eq;

        use crate::{Conversion, ConvertedValue, TEST_GRAPH};

        #[test]
        fn it_works_for_m_to_in() {
            let res = TEST_GRAPH.convert(Conversion::new("m", "in", 2.0));

            assert_relative_eq!(res.0.unwrap(), 78.72);
        }

        #[test]
        fn it_works_for_in_to_m() {
            let res = TEST_GRAPH.convert(Conversion::new("in", "m", 13.0));

            assert_relative_eq!(res.0.unwrap(), 0.33028457);
        }

        #[test]
        fn it_works_for_sec_to_hr() {
            let res = TEST_GRAPH.convert(Conversion::new("sec", "hr", 3600.0));

            assert_relative_eq!(res.0.unwrap(), 1.0);
        }

        #[test]
        fn it_correctly_does_not_work_for_in_to_hr() {
            let res = TEST_GRAPH.convert(Conversion::new("in", "hr", 13.0));

            assert_eq!(res, ConvertedValue(None));
        }
    }
}
