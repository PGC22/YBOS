//! `ReflexService.RequestAction` — placeholder pentru S6.5.
//!
//! In S6.4 acceptam call-ul si raspundem cu `ok=false, error="not implemented"`.
//! L1 poate testa wiring-ul end-to-end fara sa actioneze hardware-ul.
//! Implementarea concreta (cpufreq, brightness, fan curve, suspend) vine in S6.5.

use tonic::{Request, Response, Status};
use tracing::info;

use super::pb::action_request::Action;
use super::pb::reflex_service_server::ReflexService;
use super::pb::{ActionRequest, ActionResponse};

#[derive(Default)]
pub struct ReflexSvc;

#[tonic::async_trait]
impl ReflexService for ReflexSvc {
    async fn request_action(
        &self,
        req: Request<ActionRequest>,
    ) -> Result<Response<ActionResponse>, Status> {
        let action = req.into_inner().action;
        let name = match action {
            Some(Action::SetCpuGovernor(_)) => "set_cpu_governor",
            Some(Action::SetBrightness(_)) => "set_brightness",
            Some(Action::SetFanCurve(_)) => "set_fan_curve",
            Some(Action::Suspend(_)) => "suspend",
            None => "<empty>",
        };
        info!("[L0/grpc] ReflexService.RequestAction({}) — S6.4 stub", name);
        Ok(Response::new(ActionResponse {
            ok: false,
            error: format!("reflex action '{}' not implemented (S6.5)", name),
        }))
    }
}
