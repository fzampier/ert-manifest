use std::collections::HashSet;

/// Welford's online algorithm for computing mean and variance in O(1) memory
#[derive(Debug, Clone)]
pub struct WelfordStats {
    count: u64,
    mean: f64,
    m2: f64, // Sum of squares of differences from current mean
    min: Option<f64>,
    max: Option<f64>,
}

impl WelfordStats {
    pub fn new() -> Self {
        Self {
            count: 0,
            mean: 0.0,
            m2: 0.0,
            min: None,
            max: None,
        }
    }

    /// Add a new value to the running statistics
    pub fn update(&mut self, value: f64) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / self.count as f64;
        let delta2 = value - self.mean;
        self.m2 += delta * delta2;

        // Update min/max
        self.min = Some(self.min.map_or(value, |m| m.min(value)));
        self.max = Some(self.max.map_or(value, |m| m.max(value)));
    }

    pub fn count(&self) -> u64 {
        self.count
    }

    pub fn mean(&self) -> Option<f64> {
        if self.count > 0 {
            Some(self.mean)
        } else {
            None
        }
    }

    pub fn variance(&self) -> Option<f64> {
        if self.count > 1 {
            Some(self.m2 / (self.count - 1) as f64)
        } else {
            None
        }
    }

    pub fn std_dev(&self) -> Option<f64> {
        self.variance().map(|v| v.sqrt())
    }

    pub fn min(&self) -> Option<f64> {
        self.min
    }

    pub fn max(&self) -> Option<f64> {
        self.max
    }
}

impl Default for WelfordStats {
    fn default() -> Self {
        Self::new()
    }
}

/// P² (Piecewise-Parabolic) quantile estimator for streaming median estimation
/// Based on: Jain, R. and Chlamtac, I. (1985) "The P² Algorithm for Dynamic Calculation
/// of Quantiles and Histograms Without Storing Observations"
#[derive(Debug, Clone)]
pub struct P2Quantile {
    // Target quantile (0.5 for median)
    p: f64,
    // Marker heights (q[0]..q[4])
    q: [f64; 5],
    // Marker positions (n[0]..n[4])
    n: [i64; 5],
    // Desired marker positions (n'[0]..n'[4])
    n_prime: [f64; 5],
    // Increments for desired positions (dn'[0]..dn'[4])
    dn: [f64; 5],
    // Number of observations
    count: u64,
    // Whether estimator is initialized
    initialized: bool,
    // Initial values buffer (for first 5 observations)
    initial_values: Vec<f64>,
}

impl P2Quantile {
    /// Create a new P² estimator for the given quantile
    pub fn new(p: f64) -> Self {
        assert!((0.0..=1.0).contains(&p), "Quantile must be between 0 and 1");

        Self {
            p,
            q: [0.0; 5],
            n: [1, 2, 3, 4, 5],
            n_prime: [1.0, 1.0 + 2.0 * p, 1.0 + 4.0 * p, 3.0 + 2.0 * p, 5.0],
            dn: [0.0, p / 2.0, p, (1.0 + p) / 2.0, 1.0],
            count: 0,
            initialized: false,
            initial_values: Vec::with_capacity(5),
        }
    }

    /// Create a new P² estimator for the median (p=0.5)
    pub fn median() -> Self {
        Self::new(0.5)
    }

    /// Add a new observation
    pub fn update(&mut self, x: f64) {
        self.count += 1;

        if !self.initialized {
            self.initial_values.push(x);
            if self.initial_values.len() == 5 {
                self.initialize();
            }
            return;
        }

        // Find cell k such that q[k] <= x < q[k+1]
        let k = if x < self.q[0] {
            self.q[0] = x;
            0
        } else if x < self.q[1] {
            0
        } else if x < self.q[2] {
            1
        } else if x < self.q[3] {
            2
        } else if x < self.q[4] {
            3
        } else {
            self.q[4] = x;
            3
        };

        // Increment positions of markers k+1 through 4
        for i in (k + 1)..5 {
            self.n[i] += 1;
        }

        // Update desired positions
        for i in 0..5 {
            self.n_prime[i] += self.dn[i];
        }

        // Adjust marker heights
        for i in 1..4 {
            let d = self.n_prime[i] - self.n[i] as f64;
            if (d >= 1.0 && self.n[i + 1] - self.n[i] > 1)
                || (d <= -1.0 && self.n[i - 1] - self.n[i] < -1)
            {
                let d_sign = if d >= 0.0 { 1 } else { -1 };
                let q_new = self.parabolic(i, d_sign as f64);

                if self.q[i - 1] < q_new && q_new < self.q[i + 1] {
                    self.q[i] = q_new;
                } else {
                    self.q[i] = self.linear(i, d_sign);
                }
                self.n[i] += d_sign;
            }
        }
    }

    /// Initialize the estimator with the first 5 observations
    fn initialize(&mut self) {
        self.initial_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for (i, &v) in self.initial_values.iter().enumerate() {
            self.q[i] = v;
        }
        self.initialized = true;
    }

    /// Parabolic (P²) formula for marker adjustment
    fn parabolic(&self, i: usize, d: f64) -> f64 {
        let qi = self.q[i];
        let qi_m1 = self.q[i - 1];
        let qi_p1 = self.q[i + 1];
        let ni = self.n[i] as f64;
        let ni_m1 = self.n[i - 1] as f64;
        let ni_p1 = self.n[i + 1] as f64;

        qi + (d / (ni_p1 - ni_m1))
            * ((ni - ni_m1 + d) * (qi_p1 - qi) / (ni_p1 - ni)
                + (ni_p1 - ni - d) * (qi - qi_m1) / (ni - ni_m1))
    }

    /// Linear formula for marker adjustment (fallback)
    fn linear(&self, i: usize, d: i64) -> f64 {
        let qi = self.q[i];
        let q_adj = if d > 0 {
            self.q[i + 1]
        } else {
            self.q[i - 1]
        };
        let ni = self.n[i] as f64;
        let n_adj = if d > 0 {
            self.n[i + 1] as f64
        } else {
            self.n[i - 1] as f64
        };

        qi + (d as f64) * (q_adj - qi) / (n_adj - ni)
    }

    /// Get the current quantile estimate
    pub fn quantile(&self) -> Option<f64> {
        if !self.initialized {
            if self.initial_values.is_empty() {
                return None;
            }
            // For fewer than 5 observations, compute exact quantile
            let mut sorted = self.initial_values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let idx = ((sorted.len() - 1) as f64 * self.p).round() as usize;
            return Some(sorted[idx]);
        }
        Some(self.q[2]) // Middle marker is the quantile estimate
    }

}

impl Default for P2Quantile {
    fn default() -> Self {
        Self::median()
    }
}

/// Combined statistics tracker for a column
#[derive(Debug, Clone)]
pub struct ColumnStatTracker {
    pub welford: WelfordStats,
    pub p2_median: P2Quantile,
    pub missing_count: u64,
    pub unique_tracker: CappedUniqueTracker,
}

impl ColumnStatTracker {
    pub fn new(max_unique: usize) -> Self {
        Self {
            welford: WelfordStats::new(),
            p2_median: P2Quantile::median(),
            missing_count: 0,
            unique_tracker: CappedUniqueTracker::new(max_unique),
        }
    }

    pub fn update_numeric(&mut self, value: f64, raw_value: &str) {
        self.welford.update(value);
        self.p2_median.update(value);
        self.unique_tracker.add(raw_value);
    }

    pub fn update_string(&mut self, value: &str) {
        self.unique_tracker.add(value);
    }

    pub fn update_missing(&mut self) {
        self.missing_count += 1;
    }

    pub fn count(&self) -> u64 {
        self.welford.count()
    }
}

impl Default for ColumnStatTracker {
    fn default() -> Self {
        Self::new(2000)
    }
}

/// Capped unique value tracker that stops tracking after hitting a limit
#[derive(Debug, Clone)]
pub struct CappedUniqueTracker {
    values: HashSet<String>,
    max_values: usize,
    high_cardinality: bool,
    value_counts: std::collections::HashMap<String, u64>,
}

impl CappedUniqueTracker {
    pub fn new(max_values: usize) -> Self {
        Self {
            values: HashSet::new(),
            max_values,
            high_cardinality: false,
            value_counts: std::collections::HashMap::new(),
        }
    }

    pub fn add(&mut self, value: &str) {
        if self.high_cardinality {
            return;
        }

        *self.value_counts.entry(value.to_string()).or_insert(0) += 1;
        self.values.insert(value.to_string());

        if self.values.len() > self.max_values {
            self.high_cardinality = true;
            self.values.clear();
            self.value_counts.clear();
        }
    }

    pub fn is_high_cardinality(&self) -> bool {
        self.high_cardinality
    }

    pub fn unique_count(&self) -> usize {
        self.values.len()
    }

    pub fn values(&self) -> Option<&HashSet<String>> {
        if self.high_cardinality {
            None
        } else {
            Some(&self.values)
        }
    }

    pub fn value_counts(&self) -> Option<&std::collections::HashMap<String, u64>> {
        if self.high_cardinality {
            None
        } else {
            Some(&self.value_counts)
        }
    }

    #[cfg(test)]
    pub fn get_count(&self, value: &str) -> u64 {
        self.value_counts.get(value).copied().unwrap_or(0)
    }
}

impl Default for CappedUniqueTracker {
    fn default() -> Self {
        Self::new(2000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_welford_basic() {
        let mut stats = WelfordStats::new();
        stats.update(1.0);
        stats.update(2.0);
        stats.update(3.0);
        stats.update(4.0);
        stats.update(5.0);

        assert_eq!(stats.count(), 5);
        assert!((stats.mean().unwrap() - 3.0).abs() < 1e-10);
        assert!((stats.variance().unwrap() - 2.5).abs() < 1e-10);
        assert_eq!(stats.min(), Some(1.0));
        assert_eq!(stats.max(), Some(5.0));
    }

    #[test]
    fn test_welford_single_value() {
        let mut stats = WelfordStats::new();
        stats.update(42.0);

        assert_eq!(stats.count(), 1);
        assert_eq!(stats.mean(), Some(42.0));
        assert!(stats.variance().is_none()); // Need at least 2 values
    }

    #[test]
    fn test_welford_empty() {
        let stats = WelfordStats::new();
        assert_eq!(stats.count(), 0);
        assert!(stats.mean().is_none());
        assert!(stats.variance().is_none());
    }

    #[test]
    fn test_p2_median_basic() {
        let mut p2 = P2Quantile::median();

        // Add values 1 through 100
        for i in 1..=100 {
            p2.update(i as f64);
        }

        let median = p2.quantile().unwrap();
        // Median of 1..100 is 50.5
        assert!(
            (median - 50.5).abs() < 2.0,
            "Estimated median {} should be close to 50.5",
            median
        );
    }

    #[test]
    fn test_p2_median_small_sample() {
        let mut p2 = P2Quantile::median();
        p2.update(1.0);
        p2.update(2.0);
        p2.update(3.0);

        // For 3 values, median should be the middle one
        let median = p2.quantile().unwrap();
        assert!((median - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_p2_quantile_25() {
        let mut p2 = P2Quantile::new(0.25);

        for i in 1..=100 {
            p2.update(i as f64);
        }

        let q25 = p2.quantile().unwrap();
        assert!(
            (q25 - 25.0).abs() < 5.0,
            "Estimated 25th percentile {} should be close to 25",
            q25
        );
    }

    #[test]
    fn test_capped_unique_tracker() {
        let mut tracker = CappedUniqueTracker::new(5);

        tracker.add("a");
        tracker.add("b");
        tracker.add("c");
        tracker.add("a"); // Duplicate

        assert!(!tracker.is_high_cardinality());
        assert_eq!(tracker.unique_count(), 3);
        assert_eq!(tracker.get_count("a"), 2);
        assert_eq!(tracker.get_count("b"), 1);
    }

    #[test]
    fn test_capped_unique_tracker_overflow() {
        let mut tracker = CappedUniqueTracker::new(3);

        tracker.add("a");
        tracker.add("b");
        tracker.add("c");
        tracker.add("d"); // Exceeds limit

        assert!(tracker.is_high_cardinality());
        assert!(tracker.values().is_none());
    }

    #[test]
    fn test_column_stat_tracker() {
        let mut tracker = ColumnStatTracker::new(100);

        tracker.update_numeric(1.0, "1");
        tracker.update_numeric(2.0, "2");
        tracker.update_numeric(3.0, "3");
        tracker.update_missing();

        assert_eq!(tracker.count(), 3);
        assert_eq!(tracker.missing_count, 1);
        assert!((tracker.welford.mean().unwrap() - 2.0).abs() < 1e-10);
    }
}
