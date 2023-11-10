#[macro_export]
macro_rules! assert_where_conditional_analysis {
    ($array: ident, $index: expr, $depth: expr, $output_count: expr, $details: expr) => {
        let (depth, details, where_conditional_analysis) = &$array[$index];

        assert_eq!(depth, &$depth);
        assert_eq!(details, &$details);
        assert_eq!(where_conditional_analysis.output_count, $output_count);
        assert_eq!(where_conditional_analysis.elapsed_duration.is_zero(), false);
    }
}

pub use assert_where_conditional_analysis;
