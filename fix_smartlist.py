with open("generate_q_template.py", "r") as f:
    text = f.read()
text = text.replace("Ok(smart_list)", "Ok(teaql_core::SmartList { data: entities, facets: smart_list.facets, aggregations: smart_list.aggregations, summary: smart_list.summary, total_count: smart_list.total_count })")
with open("generate_q_template.py", "w") as f:
    f.write(text)
