use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::{Arc, RwLock, RwLockReadGuard, Weak};

/// A [`Vertex`][Vertex] behind an [`Arc`][Arc] reference counting and thread-safe pointer.
/// This allows safe (at runtime) shared access from multiple origin Vertices to the same
/// destination Vertex.
#[derive(Default, Clone)]
struct ArcVertex<T>(Arc<RwLock<Vertex<T>>>);

impl<T> From<T> for ArcVertex<T> {
    /// Helper method for creating a new [`ArcVertex`][ArcVertex] from a given generic value
    fn from(value: T) -> Self {
        ArcVertex(Arc::new(RwLock::new(Vertex {
            value,
            edges: Vec::new(),
        })))
    }
}

impl<T> From<&Edge<T>> for ArcVertex<T> {
    fn from(edge: &Edge<T>) -> Self {
        ArcVertex(
            edge.to
                .upgrade()
                .expect("Vertices should exist as long as the graph itself hasn't been dropped."),
        )
    }
}

impl<T> ArcVertex<T> {
    /// Adds an edge to the [`Vertex`][Vertex]
    fn add_edge(&self, edge: Edge<T>) {
        let write_err_msg = "Vertices should be writeable while edges are being added.";
        self.0.write().expect(write_err_msg).edges.push(edge);
    }

    /// Helper method for getting the [`Vertex`][Vertex] behind the [`Arc`][Arc] pointer
    fn get_vertex(&self) -> RwLockReadGuard<Vertex<T>> {
        let err_msg = "Attempted to read from a poisoned RwLock.";
        self.0.read().expect(err_msg)
    }

    /// Helper method for getting a weak reference to the underlying [`Vertex`][Vertex]. Useful when
    /// creating [`Edges`][Edge], where keeping strong references to graph vertices
    /// would lead to circular references that never get cleaned up, and thus memory leaks.
    fn weak_ref(&self) -> Weak<RwLock<Vertex<T>>> {
        Arc::downgrade(&self.0)
    }
}

/// Edges connect a [`Vertex`][Vertex] to another `Vertex`.
/// Each edge also contains a weight and weak pointer to the destination vertex. A weak pointer is
/// useful here, because a strong one would prevent vertices from ever being dropped.
#[derive(Default, Clone)]
struct Edge<T> {
    weight: f32,
    to: Weak<RwLock<Vertex<T>>>,
}

/// Graph vertex, containing a value and a vector of edges
#[derive(Default)]
struct Vertex<T> {
    value: T,
    edges: Vec<Edge<T>>,
}

pub struct Connection<T>
where
    T: Hash + Eq + PartialEq + Clone,
{
    from: T,
    to: T,
    value: f32,
}

impl<T> Connection<T>
where
    T: Hash + Eq + PartialEq + Clone,
{
    pub fn new(from: T, to: T, value: f32) -> Self {
        Self { from, to, value }
    }
}

/// Stores all vertices in a hashmap for O(1) lookups
pub struct Graph<T> {
    vertices: HashMap<T, ArcVertex<T>>,
}

impl<T> Graph<T>
where
    T: Hash + Eq + PartialEq + Clone,
{
    /// Helper method for creating a graph from a vec of connections
    pub fn new(connections: Vec<Connection<T>>) -> Self {
        let mut vertex_map: HashMap<T, ArcVertex<T>> = HashMap::new();

        // create a map of our Vertex pairs
        connections.iter().for_each(|fact| {
            vertex_map.insert(fact.to.clone(), ArcVertex::from(fact.to.clone()));

            vertex_map.insert(fact.from.clone(), ArcVertex::from(fact.from.clone()));
        });

        // create the edges between our vertices
        connections.iter().for_each(|fact| {
            if let (Some(origin), Some(destination)) =
                (vertex_map.get(&fact.from), vertex_map.get(&fact.to))
            {
                origin.add_edge(Edge {
                    weight: fact.value,
                    to: destination.weak_ref(),
                });

                destination.add_edge(Edge {
                    // assume the weight from destination to origin is the inverse of the given one
                    // TODO: we could allow the caller of `new` to specify how to determine weights
                    weight: 1.0 / fact.value,
                    to: origin.weak_ref(),
                });
            }
        });

        Self {
            vertices: vertex_map,
        }
    }

    /// Recursively traverse the graph looking for the vertex containing the target value
    ///
    /// # Arguments
    ///
    /// * `curr_vertex`: Current graph vertex that we're on (or started with)
    /// * `target_value`: Search target
    /// * `path`: Path taken so far
    /// * `visited`: Tracker for visited vertices. We keep a set of seen values rather than vertices
    /// themselves, because hashing vertices has some
    /// [additional challenges](https://github.com/rust-lang/rust/issues/39128).
    fn traverse(
        &self,
        curr_vertex: Option<&ArcVertex<T>>,
        target_value: T,
        path: Vec<Edge<T>>,
        visited: &mut HashSet<T>,
    ) -> Option<Vec<Edge<T>>> {
        let vertex = curr_vertex?.get_vertex();

        if vertex.value == target_value {
            // found the target
            return Some(path);
        }

        if vertex.edges.is_empty() {
            // target does not exist in the current branch
            return None;
        }

        let curr_value = vertex.value.clone();
        if visited.contains(&curr_value) {
            // detected a cycle in the current branch before getting to the target value
            return None;
        }

        visited.insert(curr_value);

        // TODO: remove recursion, which makes things rather confusing and runs the risk of stack overflow for larger graphs
        for edge in &vertex.edges {
            let mut updated_path = path.clone();
            updated_path.push(edge.clone());
            if let Some(possible_path) = self.traverse(
                Some(&edge.into()),
                target_value.clone(),
                updated_path,
                visited,
            ) {
                return Some(possible_path);
            }
        }

        None
    }

    fn find_path(&self, from: T, to: T) -> Option<Vec<Edge<T>>> {
        let starting_vertex = self.vertices.get(&from);
        self.traverse(starting_vertex, to, Vec::new(), &mut HashSet::new())
    }

    pub fn fold_path(&self, from: T, to: T, value: f32) -> Option<f32> {
        self.find_path(from, to)?
            .iter()
            .fold(value, |acc, edge| acc * edge.weight)
            .into()
    }
}
