use std::collections::BTreeMap;
use crate::TeaqlRuntime;
use crate::Q;
use teaql_core::Entity;
use crate::request_support::TeaqlUserContextExt;
use crate::request_support::AuditedSave;
pub trait IntoU64 {
    fn into_u64(self) -> u64;
}
impl IntoU64 for u64 {
    fn into_u64(self) -> u64 {
        self
    }
}
impl IntoU64 for Option<&teaql_core::Value> {
    fn into_u64(self) -> u64 {
        self.and_then(|v| v.try_u64()).unwrap_or_default()
    }
}
#[derive(Debug, Copy, Clone)]
pub enum SampleDataScale {
    Tiny,
    Small,
    Medium,
}
pub struct SampleDataPlan {
    pub scale: SampleDataScale,
    pub seed: u64,
}
impl SampleDataPlan {
    pub fn small() -> Self {
        Self {
            scale: SampleDataScale::Small,
            seed: 0,
        }
    }
}
pub struct SampleDataReport {
    pub generated: BTreeMap<&'static str, usize>,
    pub skipped: Vec<SampleDataSkipped>,
}
pub struct SampleDataSkipped {
    pub entity: &'static str,
    pub reason: String,
}
pub struct SampleDataState {
    pub plan: SampleDataPlan,
    pub references: BTreeMap<&'static str, Vec<u64>>,
    pub generated: BTreeMap<&'static str, usize>,
    pub skipped: Vec<SampleDataSkipped>,
}
impl SampleDataState {
    pub fn new(plan: SampleDataPlan) -> Self {
        Self {
            plan,
            references: BTreeMap::new(),
            generated: BTreeMap::new(),
            skipped: Vec::new(),
        }
    }
    pub fn add_reference(&mut self, entity: &'static str, id: u64) {
        self.references.entry(entity).or_default().push(id);
    }
    pub fn ids(&self, entity: &'static str) -> &[u64] {
        self.references.get(entity).map(|v| v.as_slice()).unwrap_or(&[])
    }
    pub fn pick_id(&self, entity: &'static str, salt: usize) -> Option<u64> {
        let ids = self.ids(entity);
        if ids.is_empty() {
            None
        } else {
            Some(ids[salt % ids.len()])
        }
    }
    pub fn pick_unused_id(&self, entity: &'static str, salt: usize, used: &std::collections::HashSet<u64>) -> Option<u64> {
        let ids = self.ids(entity);
        if ids.is_empty() {
            return None;
        }
        let best_id = ids[salt % ids.len()];
        if !used.contains(&best_id) {
            return Some(best_id);
        }
        for id in ids {
            if !used.contains(id) {
                return Some(*id);
            }
        }
        Some(best_id)
    }
    pub fn record_generated(&mut self, entity: &'static str) {
        *self.generated.entry(entity).or_default() += 1;
    }
    pub fn record_skipped(&mut self, entity: &'static str, reason: String) {
        self.skipped.push(SampleDataSkipped { entity, reason });
    }
    pub fn into_report(self) -> SampleDataReport {
        SampleDataReport {
            generated: self.generated,
            skipped: self.skipped,
        }
    }
}
pub async fn generate_sample_data<C>(
    ctx: &C,
    plan: SampleDataPlan,
) -> Result<SampleDataReport, String>
where
    C: TeaqlRuntime + ?Sized + crate::TeaqlRepositoryProvider,
{
    log::info!("Starting sample data generation. Scale: {:?}, Seed: {}", plan.scale, plan.seed);
    let mut state = SampleDataState::new(plan);
    load_root_ticket_statuses(ctx, &mut state).await?; //depth: 0
    ctx.user_context().transaction_data(|| async {
        Box::pin(generate_support_tickets(ctx, &mut state)).await.map_err(|e| {
            teaql_runtime::DataServiceError::Runtime(teaql_runtime::RuntimeError::Graph(e))
        })
    }).await.map_err(|e| e.to_string())?;
    ctx.user_context().transaction_data(|| async {
        Box::pin(generate_customer_issues(ctx, &mut state)).await.map_err(|e| {
            teaql_runtime::DataServiceError::Runtime(teaql_runtime::RuntimeError::Graph(e))
        })
    }).await.map_err(|e| e.to_string())?;
    let report = state.into_report();
    log::info!("Sample data generation completed successfully. Generated: {} tables, Skipped: {} tables.", report.generated.len(), report.skipped.len());
    Ok(report)
}
async fn load_root_ticket_statuses<C>(
    ctx: &C,
    state: &mut SampleDataState,
) -> Result<(), String>
where
    C: TeaqlRuntime + ?Sized + crate::TeaqlRepositoryProvider,
{
    let list = Q::ticket_statuses().purpose("Init Sample Data").execute_for_list(ctx).await.unwrap_or_default();
    for item in list {
        state.add_reference("Ticket Status", item.id().into_u64());
    }
    Ok(())
}
async fn generate_support_tickets<C>(
    ctx: &C,
    state: &mut SampleDataState,
) -> Result<(), String>
where
    C: TeaqlRuntime + ?Sized + crate::TeaqlRepositoryProvider,
{
        if state.ids("Ticket Status").is_empty() {
            state.record_skipped("Support Ticket", "Required dependency Ticket Status is missing in reference pool".to_string());
            log::info!("Skipped generating Support Ticket: Required dependency Ticket Status is missing in reference pool.");
            return Ok(());
        }
    let object_fields_count = 0 + 1;
    let base_fanout = std::cmp::max(1, object_fields_count) * 20;
    let fanout = match state.plan.scale {
        SampleDataScale::Tiny => base_fanout,
        SampleDataScale::Small => base_fanout * 5,
        SampleDataScale::Medium => base_fanout * 50,
    };
    log::info!("Generating sample data for Support Ticket (expected: {})...", fanout);
    for i in 0..fanout {
        let mut entity = Q::support_tickets().purpose("Init Sample Data").new_entity(ctx);
        let mut used_refs = std::collections::HashSet::new();
                if let Some(ref_id) = state.pick_unused_id("Ticket Status", i as usize, &used_refs) {
                    entity.update_status_id(ref_id);
                    used_refs.insert(ref_id);
                } else {
                    // Optional relation was missing in reference pool
                }
                entity.update_title(format!("{} {}", "Internet is slow", i + 1));
                {
                    let days = ((i as u64 + state.plan.seed) % (365 * 3)) as i64;
                    let past = chrono::Utc::now().naive_utc() - chrono::Duration::try_days(days).unwrap_or_default();
                    entity.update_create_time(past.format("%Y-%m-%d").to_string());
                }
                {
                    let days = ((i as u64 + state.plan.seed) % (365 * 3)) as i64;
                    let past = chrono::Utc::now().naive_utc() - chrono::Duration::try_days(days).unwrap_or_default();
                    entity.update_update_time(past.format("%Y-%m-%d").to_string());
                }
        let entity = entity.audit_as("Init Sample Data").save(ctx).await.map_err(|e| e.to_string())?;
        state.record_generated("Support Ticket");
        if i % 20 == 0 {
            log::info!("Generating Support Ticket: {}/{}", i, fanout);
        }
        state.add_reference("Support Ticket", entity.id().into_u64());
    }
    log::info!("Successfully generated sample records for Support Ticket.");
    Ok(())
}
async fn generate_customer_issues<C>(
    ctx: &C,
    state: &mut SampleDataState,
) -> Result<(), String>
where
    C: TeaqlRuntime + ?Sized + crate::TeaqlRepositoryProvider,
{
        if state.ids("Support Ticket").is_empty() {
            state.record_skipped("Customer Issue", "Required dependency Support Ticket is missing in reference pool".to_string());
            log::info!("Skipped generating Customer Issue: Required dependency Support Ticket is missing in reference pool.");
            return Ok(());
        }
    let object_fields_count = 0 + 1;
    let base_fanout = std::cmp::max(1, object_fields_count) * 20;
    let fanout = match state.plan.scale {
        SampleDataScale::Tiny => base_fanout,
        SampleDataScale::Small => base_fanout * 5,
        SampleDataScale::Medium => base_fanout * 50,
    };
    log::info!("Generating sample data for Customer Issue (expected: {})...", fanout);
    for i in 0..fanout {
        let mut entity = Q::customer_issues().purpose("Init Sample Data").new_entity(ctx);
        let mut used_refs = std::collections::HashSet::new();
                if let Some(ref_id) = state.pick_unused_id("Support Ticket", i as usize, &used_refs) {
                    entity.update_ticket_id(ref_id);
                    used_refs.insert(ref_id);
                } else {
                    // Optional relation was missing in reference pool
                }
                entity.update_description(format!("{} {}", "Unable to log in", i + 1));
                {
                    let days = ((i as u64 + state.plan.seed) % (365 * 3)) as i64;
                    let past = chrono::Utc::now().naive_utc() - chrono::Duration::try_days(days).unwrap_or_default();
                    entity.update_create_time(past.format("%Y-%m-%d").to_string());
                }
                {
                    let days = ((i as u64 + state.plan.seed) % (365 * 3)) as i64;
                    let past = chrono::Utc::now().naive_utc() - chrono::Duration::try_days(days).unwrap_or_default();
                    entity.update_update_time(past.format("%Y-%m-%d").to_string());
                }
entity.audit_as("Init Sample Data").save(ctx).await.map_err(|e| e.to_string())?;
        state.record_generated("Customer Issue");
        if i % 20 == 0 {
            log::info!("Generating Customer Issue: {}/{}", i, fanout);
        }
    }
    log::info!("Successfully generated sample records for Customer Issue.");
    Ok(())
}