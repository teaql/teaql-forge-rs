#[derive(Clone)]
pub struct SupportTicketExpression<'a> {
    result: teaql_core::eval::EvalResult<&'a crate::SupportTicket>,
    root_desc: std::sync::Arc<String>,
}

impl<'a> SupportTicketExpression<'a> {
    pub fn new(result: teaql_core::eval::EvalResult<&'a crate::SupportTicket>, root_desc: std::sync::Arc<String>) -> Self {
        Self { result, root_desc }
    }

    fn resolve(&self) -> Option<&'a crate::SupportTicket> {
        match &self.result {
            teaql_core::eval::EvalResult::Value(v) => Some(*v),
            teaql_core::eval::EvalResult::Null => None,
            teaql_core::eval::EvalResult::NotLoaded { failed_node, attempted_path } => {
                crate::trigger_logic_bug_panic(&self.root_desc, &failed_node, &attempted_path)
            }
        }
    }

    pub fn eval(&self) -> Option<&'a crate::SupportTicket> {
        self.resolve()
    }

    pub fn unwrap(&self) -> &'a crate::SupportTicket {
        self.resolve().expect("Relation was legitimately null in database!")
    }

    pub fn get_id(self) -> crate::ValueExpression<'a, u64> {
        let next = self.result.and_then("id", |entity| entity.eval_id());
        crate::ValueExpression::new(next, self.root_desc.clone())
    }

    pub fn get_title(self) -> crate::ValueExpression<'a, String> {
        let next = self.result.and_then("title", |entity| entity.eval_title());
        crate::ValueExpression::new(next, self.root_desc.clone())
    }

    pub fn get_create_time(self) -> crate::ValueExpression<'a, chrono::DateTime<chrono::Utc>> {
        let next = self.result.and_then("create_time", |entity| entity.eval_create_time());
        crate::ValueExpression::new(next, self.root_desc.clone())
    }

    pub fn get_update_time(self) -> crate::ValueExpression<'a, chrono::DateTime<chrono::Utc>> {
        let next = self.result.and_then("update_time", |entity| entity.eval_update_time());
        crate::ValueExpression::new(next, self.root_desc.clone())
    }

    pub fn get_version(self) -> crate::ValueExpression<'a, i64> {
        let next = self.result.and_then("version", |entity| entity.eval_version());
        crate::ValueExpression::new(next, self.root_desc.clone())
    }
    pub fn get_status_id(self) -> crate::ValueExpression<'a, u64> {
        let next = self.result.and_then("status_id", |entity| entity.eval_status_id());
        crate::ValueExpression::new(next, self.root_desc.clone())
    }
    pub fn get_status(self) -> crate::TicketStatusExpression<'a> {
        let next = self.result.and_then("status", |entity| entity.eval_status());
        crate::TicketStatusExpression::new(next, self.root_desc.clone())
    }
    pub fn status_is_pending(self) -> crate::ValueExpression<'a, bool> {
        let next = self.result.and_then("status_id", |entity| {
            if !entity.is_loaded("status_id") {
                teaql_core::eval::EvalResult::NotLoaded { failed_node: "status_id".to_string(), attempted_path: "status_id".to_string() }
            } else {
                teaql_core::eval::EvalResult::Value(entity.status_is_pending())
            }
        });
        crate::ValueExpression::new(next, self.root_desc.clone())
    }

    pub fn status_is_resolved(self) -> crate::ValueExpression<'a, bool> {
        let next = self.result.and_then("status_id", |entity| {
            if !entity.is_loaded("status_id") {
                teaql_core::eval::EvalResult::NotLoaded { failed_node: "status_id".to_string(), attempted_path: "status_id".to_string() }
            } else {
                teaql_core::eval::EvalResult::Value(entity.status_is_resolved())
            }
        });
        crate::ValueExpression::new(next, self.root_desc.clone())
    }
    pub fn get_customer_issue_list(self) -> crate::CustomerIssueListExpression<'a> {
        let next = self.result.and_then("customer_issue_list", |entity| entity.eval_customer_issue_list());
        crate::CustomerIssueListExpression::new(next, self.root_desc.clone())
    }
}

#[derive(Clone)]
pub struct SupportTicketListExpression<'a> {
    result: teaql_core::eval::EvalResult<&'a teaql_core::SmartList<crate::SupportTicket>>,
    root_desc: std::sync::Arc<String>,
}

impl<'a> SupportTicketListExpression<'a> {
    pub fn new(result: teaql_core::eval::EvalResult<&'a teaql_core::SmartList<crate::SupportTicket>>, root_desc: std::sync::Arc<String>) -> Self {
        Self { result, root_desc }
    }

    fn resolve(&self) -> Option<&'a teaql_core::SmartList<crate::SupportTicket>> {
        match &self.result {
            teaql_core::eval::EvalResult::Value(v) => Some(*v),
            teaql_core::eval::EvalResult::Null => None,
            teaql_core::eval::EvalResult::NotLoaded { failed_node, attempted_path } => {
                crate::trigger_logic_bug_panic(&self.root_desc, &failed_node, &attempted_path)
            }
        }
    }

    pub fn eval(&self) -> Option<&'a teaql_core::SmartList<crate::SupportTicket>> {
        self.resolve()
    }

    pub fn unwrap(&self) -> &'a teaql_core::SmartList<crate::SupportTicket> {
        self.resolve().expect("List relation was legitimately null in database!")
    }

    pub fn size(&self) -> crate::ValueExpression<'a, usize> {
        let next = self.result.clone().and_then("size", |list| teaql_core::eval::EvalResult::Value(list.len()));
        crate::ValueExpression::new(next, self.root_desc.clone())
    }

    pub fn first(&self) -> crate::SupportTicketExpression<'a> {
        let next = self.result.clone().and_then("first", |list| {
            if let Some(item) = list.first() {
                teaql_core::eval::EvalResult::Value(item)
            } else {
                teaql_core::eval::EvalResult::Null
            }
        });
        crate::SupportTicketExpression::new(next, self.root_desc.clone())
    }

    pub fn get(&self, index: usize) -> crate::SupportTicketExpression<'a> {
        let next = self.result.clone().and_then("get", |list| {
            if let Some(item) = list.get(index) {
                teaql_core::eval::EvalResult::Value(item)
            } else {
                teaql_core::eval::EvalResult::Null
            }
        });
        crate::SupportTicketExpression::new(next, self.root_desc.clone())
    }
}