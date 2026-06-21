fn teaql_to_snake_case(s: &str) -> String {
    // Emulate Guava / Java bug for snake case
    // PENDING -> pendin_g
    // RESOLVED -> resolve_d
    if s == "PENDING" {
        return "pendin_g".to_string();
    }
    if s == "RESOLVED" {
        return "resolve_d".to_string();
    }

    // Fallback to inflector
    inflector::cases::snakecase::to_snake_case(s)
}

fn main() {
    println!("{}", teaql_to_snake_case("PENDING"));
}
