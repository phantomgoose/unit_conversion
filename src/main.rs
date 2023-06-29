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
use std::sync::{Arc, RwLock, RwLockReadGuard, Weak};

use lazy_static::lazy_static;

mod tests;

// Rust has no support for complex constant initialization (since constants are initialized at
// compile time), so we're using the lazy_static crate here to lazily init our set of supported
// units at runtime instead
lazy_static! {
    static ref VALID_UNITS: HashSet<&'static str> =
        HashSet::from(["m", "in", "hr", "ft", "min", "sec"]);
}

lazy_static! {
    static ref TEST_GRAPH: ConversionGraph = ConversionGraph::new(vec![
        UnitConversion::new("m", "ft", 3.28),
        UnitConversion::new("ft", "in", 12.0),
        UnitConversion::new("hr", "min", 60.0),
        UnitConversion::new("min", "sec", 60.0),
    ]);
}

/// A [`Vertex`][Vertex] behind an [`Arc`][Arc] reference counting and thread-safe pointer.
/// This allows safe (at runtime) shared access from multiple origin Vertices to the same
/// destination Vertex.
#[derive(Default, Clone)]
struct ArcVertex(Arc<RwLock<Vertex>>);

impl From<Unit> for ArcVertex {
    /// Helper method for creating a new [`ArcVertex`][ArcVertex] for a given [`Unit`][Unit]
    fn from(unit: Unit) -> Self {
        ArcVertex(Arc::new(RwLock::new(Vertex {
            unit,
            edges: Vec::new(),
        })))
    }
}

impl From<&Edge> for ArcVertex {
    fn from(value: &Edge) -> Self {
        ArcVertex(
            value
                .to
                .upgrade()
                .expect("Vertices should exist as long as the graph itself hasn't been dropped."),
        )
    }
}

impl ArcVertex {
    /// Adds an edge to the [`Vertex`][Vertex]
    fn add_edge(&self, edge: Edge) {
        let write_err_msg = "Vertices should be writeable while edges are being added.";
        self.0.write().expect(write_err_msg).edges.push(edge);
    }

    /// Helper method for getting the [`Vertex`][Vertex] behind the [`Arc`][Arc] pointer
    fn get_vertex(&self) -> RwLockReadGuard<Vertex> {
        let err_msg = "Attempted to read from a poisoned RwLock.";
        self.0.read().expect(err_msg)
    }

    /// Helper method for getting a weak reference to the underlying [`Vertex`][Vertex]. Useful when
    /// creating [`Edges`][Edge], where keeping strong [`Arc`][Arc] references to graph vertices
    /// would lead to circular references that never get cleaned up, and thus memory leaks.
    fn weak_ref(&self) -> Weak<RwLock<Vertex>> {
        Arc::downgrade(&self.0)
    }
}

/// Edges connect a [`Vertex`][Vertex] to another `Vertex`.
/// Each edge also contains the conversion rate between the source and the destination vertices.
#[derive(Default, Clone)]
struct Edge {
    conversion_rate: f32,
    to: Weak<RwLock<Vertex>>,
}

/// Graph vertex, containing the conversion [`Unit`][Unit].
#[derive(Default)]
struct Vertex {
    unit: Unit,
    edges: Vec<Edge>,
}

/// Struct representing our conversion units.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
struct Unit(String);

impl From<&str> for Unit {
    /// Helper functionality for converting str slices to [`Unit`][Unit]s.
    /// Performs basic validation on the provided string (i.e. it must be one of the known unit
    /// types).
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
struct ConversionResult(Option<f32>);

impl ToString for ConversionResult {
    /// Format the conversion result in accordance with the interview examples
    fn to_string(&self) -> String {
        if let Some(value) = self.0 {
            format!("answer = {}", value)
        } else {
            "not convertible!".to_string()
        }
    }
}

/// Captures information about the provided units and conversion rates between them
struct ConversionGraph {
    // a map of units to respective entrypoint vertices for O(1) lookups for the first vertex in a conversion chain
    vertices: HashMap<Unit, ArcVertex>,
}

impl ConversionGraph {
    /// Creates a new graph to capture relationships between unit conversion facts,
    /// which can then be used to answer queries via [`convert`][convert].
    ///
    /// [convert]: ConversionGraph::convert
    pub fn new(facts: Vec<UnitConversion>) -> Self {
        let mut vertex_map: HashMap<Unit, ArcVertex> = HashMap::new();

        // create a map of our unit -> Vertex pairs
        facts.iter().for_each(|fact| {
            vertex_map.insert(fact.to.clone(), ArcVertex::from(fact.to.clone()));

            vertex_map.insert(fact.from.clone(), ArcVertex::from(fact.from.clone()));
        });

        // create the edges between our unit vertices to capture conversion rate information
        facts.iter().for_each(|fact| {
            if let (Some(origin), Some(destination)) =
                (vertex_map.get(&fact.from), vertex_map.get(&fact.to))
            {
                origin.add_edge(Edge {
                    conversion_rate: fact.value,
                    to: destination.weak_ref(),
                });

                destination.add_edge(Edge {
                    // conversion rate from destination to origin is the inverse of the given rate
                    conversion_rate: 1.0 / fact.value,
                    to: origin.weak_ref(),
                });
            }
        });

        ConversionGraph {
            vertices: vertex_map,
        }
    }

    /// Traverse the graph looking for the vertex containing the target unit
    ///
    /// # Arguments
    ///
    /// * `curr_vertex`: Current graph vertex that we're on (or started with)
    /// * `target_unit`: Search target
    /// * `path`: Path taken so far
    /// * `visited`: Tracker for visited vertices. We keep a set of seen units rather than vertices
    /// themselves, because hashing vertices has some
    /// [additional challenges](https://github.com/rust-lang/rust/issues/39128).
    ///
    /// returns: Option<Vec<Edge, Global>>
    fn traverse(
        curr_vertex: Option<&ArcVertex>,
        target_unit: Unit,
        path: Vec<Edge>,
        visited: &mut HashSet<Unit>,
    ) -> Option<Vec<Edge>> {
        let vertex = curr_vertex?.get_vertex();

        if vertex.unit == target_unit {
            // found the target
            return Some(path);
        }

        if vertex.edges.is_empty() {
            // target does not exist in the current branch
            return None;
        }

        let curr_unit = vertex.unit.clone();
        if visited.contains(&curr_unit) {
            // detected a cycle in the current branch before getting to the target unit
            return None;
        }

        visited.insert(curr_unit);

        // TODO: remove recursion, which makes things rather confusing
        for edge in &vertex.edges {
            let mut updated_path = path.clone();
            updated_path.push(edge.clone());
            if let Some(possible_path) = Self::traverse(
                Some(&edge.into()),
                target_unit.clone(),
                updated_path,
                visited,
            ) {
                return Some(possible_path);
            }
        }

        None
    }

    /// Attempts to perform the requested unit conversion based on the graph of factuals that we have.
    pub fn convert(&self, query: UnitConversion) -> ConversionResult {
        let starting_vertex = self.vertices.get(&query.from);
        if let Some(path) =
            Self::traverse(starting_vertex, query.to, Vec::new(), &mut HashSet::new())
        {
            let res = path
                .iter()
                .fold(query.value, |acc, edge| acc * edge.conversion_rate);
            ConversionResult(Some(res))
        } else {
            ConversionResult(None)
        }
    }
}

/// Represents a unit conversion (whether a known factual or a query)
#[derive(Debug)]
struct UnitConversion {
    from: Unit,
    to: Unit,
    value: f32,
}

impl UnitConversion {
    /// Helper for initializing a `UnitConversion` from str slices
    fn new(from: &str, to: &str, value: f32) -> Self {
        UnitConversion {
            from: Unit::from(from),
            to: Unit::from(to),
            value,
        }
    }
}

// simple scenario for cargo run
fn main() {
    let res = TEST_GRAPH
        .convert(UnitConversion::new("min", "sec", 2.0))
        .to_string();
    dbg!(res);
}
