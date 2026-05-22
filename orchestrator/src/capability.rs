use anyhow::{anyhow, Result};
use std::path::PathBuf;
use crate::manifest::{Manifest, AccessLevel, MemoryAccess};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CapabilityError {
    #[error("Capability denied: {0}")]
    Denied(String),
}

#[derive(Debug, Clone)]
pub enum Operation {
    NetConnect(String),
    FsRead(PathBuf),
    FsWrite(PathBuf),
    UserContextRead,
    UserContextWrite,
    LlmCall,
    MemoryRead,
    MemoryWrite,
}

pub fn enforce(manifest: &Manifest, op: &Operation) -> Result<()> {
    let res = match op {
        Operation::NetConnect(domain) => {
            if manifest.capabilities.net_domains.iter().any(|d| d == domain) {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied(format!(
                    "NetConnect({})",
                    domain
                ))))
            }
        }
        Operation::FsRead(path) => {
            // Path normalization prevents ".." bypass attacks by resolving lexical components.
            // Example: "/data/agent/../../etc/passwd" becomes "/etc/passwd".
            let requested_clean = path_clean::clean(path);
            let declared_clean_list: Vec<_> = manifest
                .capabilities
                .fs_paths
                .iter()
                .map(path_clean::clean)
                .collect();

            if declared_clean_list
                .iter()
                .any(|d| requested_clean.starts_with(d))
            {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied(format!(
                    "FsRead({})",
                    requested_clean.display()
                ))))
            }
        }
        Operation::FsWrite(path) => {
            // Path normalization prevents ".." bypass attacks by resolving lexical components.
            // Example: "/data/agent/../../etc/passwd" becomes "/etc/passwd".
            let requested_clean = path_clean::clean(path);
            let declared_clean_list: Vec<_> = manifest
                .capabilities
                .fs_paths
                .iter()
                .map(path_clean::clean)
                .collect();

            if declared_clean_list
                .iter()
                .any(|d| requested_clean.starts_with(d))
            {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied(format!(
                    "FsWrite({})",
                    requested_clean.display()
                ))))
            }
        }
        Operation::UserContextRead => {
            if manifest.capabilities.data_user_prefs == AccessLevel::Read
                || manifest.capabilities.data_user_prefs == AccessLevel::ReadWrite
            {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied(
                    "UserContextRead".to_string()
                )))
            }
        }
        Operation::UserContextWrite => {
            if manifest.capabilities.data_user_prefs == AccessLevel::ReadWrite {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied("UserContextWrite".to_string())))
            }
        }
        Operation::LlmCall => {
            if manifest.capabilities.llm {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied("LlmCall".to_string())))
            }
        }
        Operation::MemoryRead => {
            if manifest.capabilities.memory == MemoryAccess::Read ||
               manifest.capabilities.memory == MemoryAccess::ReadWrite {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied("MemoryRead".to_string())))
            }
        }
        Operation::MemoryWrite => {
            if manifest.capabilities.memory == MemoryAccess::ReadWrite {
                Ok(())
            } else {
                Err(anyhow!(CapabilityError::Denied("MemoryWrite".to_string())))
            }
        }
    };

    // Audit log
    let op_log = match op {
        Operation::FsRead(path) => Operation::FsRead(path_clean::clean(path)),
        Operation::FsWrite(path) => Operation::FsWrite(path_clean::clean(path)),
        _ => op.clone(),
    };

    match &res {
        Ok(_) => {
            tracing::info!(
                target: "ybos.audit",
                agent = %manifest.name,
                op = ?op_log,
                outcome = "allow",
                "Capability check"
            );
        }
        Err(e) => {
            let reason_str = e.to_string();
            tracing::warn!(
                target: "ybos.audit",
                agent = %manifest.name,
                op = ?op_log,
                outcome = "deny",
                reason = %reason_str,
                "Capability check denied"
            );
        }
    }

    res
}

#[cfg(test)]
mod tests {
    // Audit-log tests live here (in-crate unit tests) rather than in
    // `tests/capability_hardening.rs` because the library's
    // `tracing::info!` / `tracing::warn!` calls are not reliably captured
    // by `tracing-test` when invoked from a separate integration-test crate
    // (its MockWriter has trouble with custom targets like `ybos.audit`).
    //
    // Instead we install a small custom `Layer` that records every event
    // it sees into a Vec we can inspect directly. This gives a fully
    // deterministic assertion on field values, target, and level.
    //
    // Path-normalization behaviour stays in `tests/capability_hardening.rs`
    // because it does NOT depend on log capture.

    use super::*;
    use crate::manifest::Capabilities;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use tracing::field::{Field, Visit};
    use tracing::{Event, Subscriber};
    use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
    use tracing_subscriber::util::SubscriberInitExt;

    #[derive(Debug, Clone)]
    struct CapturedEvent {
        target: String,
        level: tracing::Level,
        fields: HashMap<String, String>,
    }

    #[derive(Default, Clone)]
    struct CaptureLayer {
        events: Arc<Mutex<Vec<CapturedEvent>>>,
    }

    struct FieldVisitor<'a>(&'a mut HashMap<String, String>);

    impl<'a> Visit for FieldVisitor<'a> {
        fn record_str(&mut self, field: &Field, value: &str) {
            self.0.insert(field.name().to_string(), value.to_string());
        }
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.0.insert(field.name().to_string(), format!("{:?}", value));
        }
    }

    impl<S: Subscriber> Layer<S> for CaptureLayer {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let mut fields = HashMap::new();
            event.record(&mut FieldVisitor(&mut fields));
            self.events.lock().unwrap().push(CapturedEvent {
                target: event.metadata().target().to_string(),
                level: *event.metadata().level(),
                fields,
            });
        }
    }

    fn audit_events(layer: &CaptureLayer) -> Vec<CapturedEvent> {
        layer
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.target == "ybos.audit")
            .cloned()
            .collect()
    }

    #[test]
    fn audit_log_emits_allow_event() {
        let layer = CaptureLayer::default();
        let _guard = tracing_subscriber::registry()
            .with(layer.clone())
            .set_default();

        let manifest = Manifest {
            name: "test-agent".to_string(),
            version: "0.1.0".to_string(),
            capabilities: Capabilities {
                llm: true,
                ..Default::default()
            },
        };

        enforce(&manifest, &Operation::LlmCall).expect("LlmCall should be allowed");

        let events = audit_events(&layer);
        assert_eq!(events.len(), 1, "expected one audit event on allow");
        let ev = &events[0];
        assert_eq!(ev.level, tracing::Level::INFO, "allow should log at INFO");
        assert_eq!(ev.fields.get("outcome").map(String::as_str), Some("allow"));
        assert_eq!(ev.fields.get("agent").map(String::as_str), Some("test-agent"));
        assert!(
            ev.fields.get("op").is_some(),
            "expected op field in audit event"
        );
    }

    #[test]
    fn audit_log_emits_deny_event_with_reason() {
        let layer = CaptureLayer::default();
        let _guard = tracing_subscriber::registry()
            .with(layer.clone())
            .set_default();

        let manifest = Manifest {
            name: "test-agent".to_string(),
            version: "0.1.0".to_string(),
            capabilities: Default::default(),
        };

        let err = enforce(&manifest, &Operation::LlmCall).unwrap_err();
        assert!(err.to_string().contains("LlmCall"));

        let events = audit_events(&layer);
        assert_eq!(events.len(), 1, "expected one audit event on deny");
        let ev = &events[0];
        assert_eq!(ev.level, tracing::Level::WARN, "deny should log at WARN");
        assert_eq!(ev.fields.get("outcome").map(String::as_str), Some("deny"));
        assert_eq!(ev.fields.get("agent").map(String::as_str), Some("test-agent"));
        let reason = ev
            .fields
            .get("reason")
            .expect("expected reason field on deny");
        assert!(
            reason.contains("LlmCall"),
            "reason should mention denied operation: {}",
            reason
        );
    }
}
