use minijinja::Environment;

fn main() {
    let mut env = Environment::new();
    env.add_template(
        "test",
        r#"            query: SelectQuery::new("TicketStatus")
{%- for f in fields %}
{%- if f == "id" or f == "version" %}
                .project("{{ f }}")
{%- endif -%}
{% endfor %},"#,
    )
    .unwrap();

    let template = env.get_template("test").unwrap();
    let result = template
        .render(minijinja::context! {
            fields => vec!["id", "name", "version", "other"]
        })
        .unwrap();
    println!("---");
    println!("{}", result);
    println!("---");
}
