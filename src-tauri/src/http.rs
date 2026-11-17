//! Blocking HTTP client for the usage endpoints (honours HTTPS_PROXY and the OS trust store).

use agentpets_core::usage::{Http, HttpResponse};
use std::time::Duration;

pub struct UreqHttp {
    agent: ureq::Agent,
}

impl UreqHttp {
    pub fn new() -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(15))
            .try_proxy_from_env(true)
            .user_agent("agent-pets-usage-monitor/0.1")
            .build();
        Self { agent }
    }
}

impl Http for UreqHttp {
    fn request(
        &self,
        method: &str,
        url: &str,
        headers: &[(&str, String)],
        body: Option<String>,
    ) -> Result<HttpResponse, String> {
        let mut req = self.agent.request(method, url);
        for (k, v) in headers {
            req = req.set(k, v);
        }
        let res = match body {
            Some(b) => req.send_string(&b),
            None => req.call(),
        };
        match res {
            Ok(r) => {
                let status = r.status();
                let body = r.into_string().map_err(|e| e.to_string())?;
                Ok(HttpResponse { status, body })
            }
            Err(ureq::Error::Status(status, r)) => Ok(HttpResponse {
                status,
                body: r.into_string().unwrap_or_default(),
            }),
            Err(e) => Err(e.to_string()),
        }
    }
}
