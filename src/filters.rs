#[macro_export]
macro_rules! gaussian {
    ($points:expr, $window_size:expr) => {{
        let mut res = Vec::with_capacity($points.len());
        for i in 0..$points.len() {
            let window = $points.iter().skip(i).take($window_size);
            let avg = window.fold(f64x2::from([0.0, 0.0]), |acc, p| acc + *p) / $window_size as f64;
            res.push(avg);
        }
        res
    }};
}
