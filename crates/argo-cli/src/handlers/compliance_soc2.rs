//! Thin CLI shim for `argo compliance soc2`.

#[allow(unused_imports)]
pub use argo_compliance::soc2::{
    classify, CategoryAggregate, ComplianceSoc2Args, ReportEvent, ReportFilters, ReportFormat,
    ReportWindow, Soc2Category, Soc2Report, ACTION_CATEGORY,
};
use argo_store::SessionStore;
use argo_types::ArgoResult;

pub async fn run(args: ComplianceSoc2Args, store: &SessionStore) -> ArgoResult<i32> {
    argo_compliance::soc2::run(args, store).await
}
