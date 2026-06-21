use teaql_core::Expr;

use crate::*;

pub struct PurposedQuery<T> {
    pub inner: T,
    pub purpose: String,
}

impl<T> PurposedQuery<T> {
    pub fn new(inner: T, purpose: impl Into<String>) -> Self {
        Self { inner, purpose: purpose.into() }
    }
}

pub struct Q;

impl Q {
    pub fn ticket_statuses() -> TicketStatusRequest {
        TicketStatusRequest::new()
            .select_self()
            .and_filter(Expr::gt("version", 0_i64))
    }

    pub fn ticket_statuses_minimal() -> TicketStatusRequest {
        TicketStatusRequest::new()
            .and_filter(Expr::gt("version", 0_i64))
    }

    pub fn ticket_statuses_with_children() -> TicketStatusRequest {
        TicketStatusRequest::new()
            .unlimited()
            .select_self_fields()
            .enhance_children_if_needed()
    }

    pub fn support_tickets() -> SupportTicketRequest {
        SupportTicketRequest::new()
            .select_self()
            .and_filter(Expr::gt("version", 0_i64))
    }

    pub fn support_tickets_minimal() -> SupportTicketRequest {
        SupportTicketRequest::new()
            .and_filter(Expr::gt("version", 0_i64))
    }

    pub fn support_tickets_with_children() -> SupportTicketRequest {
        SupportTicketRequest::new()
            .unlimited()
            .select_self_fields()
            .enhance_children_if_needed()
    }

    pub fn customer_issues() -> CustomerIssueRequest {
        CustomerIssueRequest::new()
            .select_self()
            .and_filter(Expr::gt("version", 0_i64))
    }

    pub fn customer_issues_minimal() -> CustomerIssueRequest {
        CustomerIssueRequest::new()
            .and_filter(Expr::gt("version", 0_i64))
    }

    pub fn customer_issues_with_children() -> CustomerIssueRequest {
        CustomerIssueRequest::new()
            .unlimited()
            .select_self_fields()
            .enhance_children_if_needed()
    }
}