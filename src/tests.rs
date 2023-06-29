#[cfg(test)]
mod test_convert {
    use approx::assert_relative_eq;

    use crate::{ConversionResult, UnitConversion, TEST_GRAPH};

    #[test]
    fn it_works_for_m_to_in() {
        let res = TEST_GRAPH.convert(UnitConversion::new("m", "in", 2.0));

        assert_relative_eq!(res.0.unwrap(), 78.72);
    }

    #[test]
    fn it_works_for_in_to_m() {
        let res = TEST_GRAPH.convert(UnitConversion::new("in", "m", 13.0));

        assert_relative_eq!(res.0.unwrap(), 0.33028457);
    }

    #[test]
    fn it_works_for_sec_to_hr() {
        let res = TEST_GRAPH.convert(UnitConversion::new("sec", "hr", 3600.0));

        assert_relative_eq!(res.0.unwrap(), 1.0);
    }

    #[test]
    fn it_correctly_does_not_work_for_in_to_hr() {
        let res = TEST_GRAPH.convert(UnitConversion::new("in", "hr", 13.0));

        assert_eq!(res, ConversionResult(None));
    }
}
