use toml::{Table, Value};

pub fn merge_table(higher: &mut Table, lower: Table) {
    for (key, value) in lower {
        if !higher.contains_key(&key) {
            higher.insert(key, value);
            continue;
        }

        let a_value = higher.get_mut(&key).unwrap();

        if let (Value::Table(a_table), Value::Table(b_table)) = (a_value, value) {
            merge_table(a_table, b_table);
        }
    }
}

pub fn merge_option_table(higher: &mut Option<Table>, lower: Option<Table>) {
    if higher.is_none() {
        *higher = lower;
    } else if let (Some(a_inner), Some(b_inner)) = (higher, lower) {
        merge_table(a_inner, b_inner);
    }
}

#[cfg(test)]
mod tests {
    use assert2::assert;

    use toml::Table;

    use super::merge_option_table;
    use super::merge_table;

    const HIGHER: &str = r#"
override = "overridden"
only_in_high = "only_in_high"

[table]
override = "overridden"
only_in_high = "only_in_high"
        "#;

    const LOWER: &str = r#"
override = "not_overridden"
only_in_low = "only_in_low"

[table]
override = "not_overridden"
only_in_low = "only_in_low"
        "#;

    const EXPECTED_RESULT: &str = r#"
override = "overridden"
only_in_high = "only_in_high"
only_in_low = "only_in_low"

[table]
override = "overridden"
only_in_high = "only_in_high"
only_in_low = "only_in_low"
        "#;

    #[test]
    fn merge_table_test() {
        let mut higher: Table = toml::from_str(HIGHER).unwrap();

        let lower: Table = toml::from_str(LOWER).unwrap();

        let expected_result: Table = toml::from_str(EXPECTED_RESULT).unwrap();

        merge_table(&mut higher, lower);

        assert!(expected_result == higher);
    }

    #[test]
    fn merge_option_table_some_none() {
        let mut higher: Option<Table> = Some(toml::from_str(HIGHER).unwrap());

        let expected_result: Option<Table> = Some(toml::from_str(HIGHER).unwrap());

        merge_option_table(&mut higher, None);

        assert!(expected_result == higher);
    }

    #[test]
    fn merge_option_table_none_some() {
        let mut higher: Option<Table> = None;
        let lower: Option<Table> = Some(toml::from_str(LOWER).unwrap());

        let expected_result: Option<Table> = Some(toml::from_str(LOWER).unwrap());

        merge_option_table(&mut higher, lower);

        assert!(expected_result == higher);
    }

    #[test]
    fn merge_option_table_both_none() {
        let mut higher: Option<Table> = None;

        merge_option_table(&mut higher, None);

        assert!(None == higher);
    }

    #[test]
    fn merge_option_table_both_some() {
        let mut higher: Option<Table> = Some(toml::from_str(HIGHER).unwrap());

        let lower: Option<Table> = Some(toml::from_str(LOWER).unwrap());

        let expected_result: Option<Table> = Some(toml::from_str(EXPECTED_RESULT).unwrap());

        merge_option_table(&mut higher, lower);

        assert!(expected_result == higher);
    }
}
