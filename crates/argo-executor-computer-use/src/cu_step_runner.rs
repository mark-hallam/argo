//! Platform-neutral step-runner seam for the CU loop.
//!
//! The loop owns Anthropic turn-taking, budgeting, audit rows, and
//! restart decisions. A [`CuStepRunner`] owns the platform-specific
//! side effects: preparing the target surface, capturing screenshots,
//! applying one computer-use action, and recovering after transient
//! failures.
//!
//! ```no_run
//! use argo_executor_computer_use::CuStepRunner;
//! use argo_executor_computer_use::e2b::{E2bClient, E2bSandbox};
//!
//! # async fn demo(client: E2bClient, sandbox: E2bSandbox)
//! #     -> Result<(), argo_executor_computer_use::CuError> {
//! client.preflight().await?;
//! let screenshot = client.warm_up(&sandbox).await?;
//! assert!(!screenshot.is_empty());
//! # Ok(())
//! # }
//! ```

use crate::e2b::{ActionResult, CuAction, E2bClient, E2bSandbox};
use crate::executor_loop_exit::handle_restart;
use crate::profile::Profile;
use crate::restart::RestartCoordinator;
use crate::CuError;
use async_trait::async_trait;

/// Platform-neutral action runner used by the shared CU loop.
#[async_trait]
pub trait CuStepRunner: Send + Sync {
    /// Per-dispatch platform handle. E2B uses an [`E2bSandbox`];
    /// native executors can use their own app/window session type.
    type Session: Clone + Send + Sync;

    /// Check that the platform is usable before the first model step.
    async fn preflight(&self) -> Result<(), CuError> {
        Ok(())
    }

    /// Prepare the platform and return the first screenshot.
    async fn warm_up(&self, session: &Self::Session) -> Result<Vec<u8>, CuError>;

    /// Capture a screenshot after one or more actions have run.
    async fn capture_screenshot(&self, session: &Self::Session) -> Result<Vec<u8>, CuError>;

    /// Execute one model-requested computer-use action.
    async fn execute_step_recorded(
        &self,
        session: &Self::Session,
        action: &CuAction,
    ) -> Result<ActionResult, CuError>;

    /// Recover or hand off after a transient platform/model failure.
    #[allow(clippy::too_many_arguments)]
    async fn recover_after_error(
        &self,
        err: &CuError,
        stage: &'static str,
        profile: &Profile,
        coordinator: &mut RestartCoordinator,
        session: &mut Self::Session,
        screenshot: &mut Vec<u8>,
        system_prompt: &mut String,
        base_system_prompt: &str,
    ) -> Result<Option<String>, CuError>;
}

#[async_trait]
impl CuStepRunner for E2bClient {
    type Session = E2bSandbox;

    async fn warm_up(&self, session: &Self::Session) -> Result<Vec<u8>, CuError> {
        self.screenshot(session).await
    }

    async fn capture_screenshot(&self, session: &Self::Session) -> Result<Vec<u8>, CuError> {
        self.screenshot(session).await
    }

    async fn execute_step_recorded(
        &self,
        session: &Self::Session,
        action: &CuAction,
    ) -> Result<ActionResult, CuError> {
        self.execute_action(session, action).await
    }

    async fn recover_after_error(
        &self,
        err: &CuError,
        stage: &'static str,
        profile: &Profile,
        coordinator: &mut RestartCoordinator,
        session: &mut Self::Session,
        screenshot: &mut Vec<u8>,
        system_prompt: &mut String,
        base_system_prompt: &str,
    ) -> Result<Option<String>, CuError> {
        handle_restart(
            self,
            err,
            stage,
            profile,
            coordinator,
            session,
            screenshot,
            system_prompt,
            base_system_prompt,
        )
        .await
    }
}
