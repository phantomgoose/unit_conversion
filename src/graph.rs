use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Debug;
use std::hash::Hash;
use std::iter::once;
use std::sync::{Arc, RwLock, RwLockReadGuard, Weak};

/// A [`Vertex`][Vertex] behind an [`Arc`][Arc] reference counting and thread-safe pointer.
/// This allows safe (at runtime) shared access from multiple origin Vertices to the same
/// destination Vertex.
#[derive(Default, Clone, Debug)]
struct ArcVertex<T>(Arc<RwLock<Vertex<T>>>);

type WeakVertex<T> = Weak<RwLock<Vertex<T>>>;

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
    fn read_lock(&self) -> RwLockReadGuard<Vertex<T>> {
        let err_msg = "Attempted to read from a poisoned RwLock.";
        self.0.read().expect(err_msg)
    }

    /// Helper method for getting a weak reference to the underlying [`Vertex`][Vertex]. Useful when
    /// creating [`Edges`][Edge], where keeping strong references to graph vertices
    /// would lead to circular references that never get cleaned up, and thus memory leaks.
    fn weak_ref(&self) -> WeakVertex<T> {
        Arc::downgrade(&self.0)
    }
}

/// Edges connect a [`Vertex`][Vertex] to another `Vertex`.
/// Each edge also contains a weight and weak pointer to the destination vertex. A weak pointer is
/// useful here, because a strong one would prevent vertices from ever being dropped.
#[derive(Default, Clone, Debug)]
struct Edge<T> {
    weight: f32,
    to: WeakVertex<T>,
}

/// Graph vertex, containing a value and a vector of edges
#[derive(Default, Debug)]
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
    T: Hash + Eq + PartialEq + Clone + Debug,
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
    T: Hash + Eq + PartialEq + Clone + Debug,
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

    /// Traverse the graph via BFS looking for the vertex containing the target value.
    /// Returns a vector of [`Edge`][Edge]s forming a path between the two vertices.
    fn find_path(&self, starting_value: T, target_value: T) -> Option<Vec<Edge<T>>> {
        // Tracker for visited vertices. We keep a set of seen values rather than vertices
        // themselves, because hashing vertices has some additional challenges
        // https://github.com/rust-lang/rust/issues/39128.
        let mut visited: HashSet<T> = HashSet::new();

        // queue for storing vertices and the path to them
        let mut queue = VecDeque::new();

        let starting_vertex = self.vertices.get(&starting_value)?.clone();
        // our starting path is empty, because we're storing edge, and we haven't traversed any yet
        let starting_path = Vec::new();

        queue.push_front((starting_vertex, starting_path));

        while let Some((curr_vertex, path)) = queue.pop_back() {
            let vertex_lock = curr_vertex.read_lock();
            visited.insert(vertex_lock.value.clone());

            if vertex_lock.value == target_value {
                // found the target
                return Some(path);
            }

            vertex_lock.edges.iter().for_each(|edge| {
                let next: ArcVertex<T> = edge.into();
                let next_value = &next.read_lock().value;

                if !visited.contains(next_value) {
                    let new_path: Vec<Edge<T>> =
                        path.iter().cloned().chain(once(edge.clone())).collect();
                    queue.push_front((next.clone(), new_path));
                }
            });
        }

        None
    }

    /// Helper method for finding a path between two values and multiplying the provided value by
    /// all the weights in the path
    pub fn fold_path(&self, from: T, to: T, value: f32) -> Option<f32> {
        self.find_path(from, to)?
            .iter()
            .fold(value, |acc, edge| acc * edge.weight)
            .into()
    }
}
