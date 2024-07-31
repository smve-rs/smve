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
