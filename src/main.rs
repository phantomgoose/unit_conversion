use lazy_static::lazy_static;

use conversion::ConversionGraph;

use crate::conversion::UnitConversion;

mod conversion;
mod graph;
mod tests;

// Rust has no support for complex constant initialization (since constants are initialized at
// compile time), so we're using the lazy_static crate here to lazily init our set of supported
// units at runtime instead
lazy_static! {
    static ref TEST_GRAPH: ConversionGraph = ConversionGraph::new(vec![
        UnitConversion::new("m", "ft", 3.28),
        UnitConversion::new("ft", "in", 12.0),
        UnitConversion::new("hr", "min", 60.0),
        UnitConversion::new("min", "sec", 60.0),
    ]);
}

// simple scenario for cargo run
fn main() {
    let res = TEST_GRAPH
        .convert(UnitConversion::new("min", "sec", 2.0))
        .to_string();
    dbg!(res);
}
