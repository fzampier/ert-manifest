use crate::types::SafeValue;

/// Bucket a count into a privacy-safe range
pub fn bucket_count(n: u64) -> &'static str {
    match n {
        0 => "0",
        1 => "1",
        2..=5 => "2-5",
        6..=10 => "6-10",
        11..=20 => "11-20",
        21..=100 => "21-100",
        101..=1000 => "101-1000",
        _ => ">1000",
    }
}

/// Convert a count to a SafeValue, bucketing if requested
pub fn safe_count(n: u64, bucket: bool) -> SafeValue {
    if bucket {
        SafeValue::ShortString(bucket_count(n).to_string())
    } else {
        SafeValue::Integer(n as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucket_count_zero() {
        assert_eq!(bucket_count(0), "0");
    }

    #[test]
    fn test_bucket_count_one() {
        assert_eq!(bucket_count(1), "1");
    }

    #[test]
    fn test_bucket_count_two_to_five() {
        assert_eq!(bucket_count(2), "2-5");
        assert_eq!(bucket_count(3), "2-5");
        assert_eq!(bucket_count(5), "2-5");
    }

    #[test]
    fn test_bucket_count_six_to_ten() {
        assert_eq!(bucket_count(6), "6-10");
        assert_eq!(bucket_count(10), "6-10");
    }

    #[test]
    fn test_bucket_count_eleven_to_twenty() {
        assert_eq!(bucket_count(11), "11-20");
        assert_eq!(bucket_count(20), "11-20");
    }

    #[test]
    fn test_bucket_count_twentyone_to_hundred() {
        assert_eq!(bucket_count(21), "21-100");
        assert_eq!(bucket_count(50), "21-100");
        assert_eq!(bucket_count(100), "21-100");
    }

    #[test]
    fn test_bucket_count_hundred_one_to_thousand() {
        assert_eq!(bucket_count(101), "101-1000");
        assert_eq!(bucket_count(500), "101-1000");
        assert_eq!(bucket_count(1000), "101-1000");
    }

    #[test]
    fn test_bucket_count_over_thousand() {
        assert_eq!(bucket_count(1001), ">1000");
        assert_eq!(bucket_count(10000), ">1000");
        assert_eq!(bucket_count(1000000), ">1000");
    }

    #[test]
    fn test_safe_count_bucketed() {
        let result = safe_count(15, true);
        assert_eq!(result, SafeValue::ShortString("11-20".to_string()));
    }

    #[test]
    fn test_safe_count_exact() {
        let result = safe_count(15, false);
        assert_eq!(result, SafeValue::Integer(15));
    }
}
