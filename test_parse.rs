fn main() {
    let val = teaql_core::Value::I64(0);
    let b: bool = val.try_bool().unwrap_or(false);
    println!("Parsed bool: {}", b);
}
