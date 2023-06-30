use crate::graph::{Connection, Graph};
use lazy_static::lazy_static;
use std::collections::HashSet;

lazy_static! {
    static ref VALID_UNITS: HashSet<&'static str> =
        HashSet::from(["m", "in", "hr", "ft", "min", "sec"]);
}

pub struct ConversionGraph {
    graph: Graph<Unit>,
}

impl ConversionGraph {
    pub fn new(facts: Vec<UnitConversion>) -> Self {
        Self {
            graph: Graph::new(
                facts
                    .iter()
                    .map(|f| f.into())
                    .collect::<Vec<Connection<Unit>>>(),
            ),
        }
    }

    /// Attempts to perform the requested unit conversion based on the graph of factuals that we have.
    pub fn convert(&self, query: UnitConversion) -> ConversionResult {
        ConversionResult(self.graph.fold_path(query.from, query.to, query.value))
    }
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
pub struct ConversionResult(pub(crate) Option<f32>);

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

/// Represents a unit conversion (whether a known factual or a query)
#[derive(Debug)]
pub struct UnitConversion {
    from: Unit,
    to: Unit,
    value: f32,
}

impl From<&UnitConversion> for Connection<Unit> {
    fn from(conversion: &UnitConversion) -> Self {
        Self::new(
            conversion.from.clone(),
            conversion.to.clone(),
            conversion.value,
        )
    }
}

impl UnitConversion {
    /// Helper for initializing a `UnitConversion` from str slices
    pub fn new(from: &str, to: &str, value: f32) -> Self {
        UnitConversion {
            from: Unit::from(from),
            to: Unit::from(to),
            value,
        }
    }
}
