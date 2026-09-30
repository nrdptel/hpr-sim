//! The HTTP transport: blocking HTTP/1.1 through `ureq`, TLS through rustls.

use std::io::Read;
use std::time::Duration;

use crate::Transport;

/// A [`Transport`] that fetches over HTTP and HTTPS, behind the crate's `http` feature.
///
/// TLS is rustls with Mozilla's root certificates compiled in (the `webpki-roots` crate), so it
/// needs no OpenSSL and ignores the operating system's certificate store. It follows up to ten
/// redirects, sends `Accept-Encoding: gzip` and unpacks gzip bodies, and uses a proxy named by the
/// `ALL_PROXY`, `HTTPS_PROXY` or `HTTP_PROXY` environment variables. A response whose status is
/// not 2xx is an error, as are a timeout and a body that unpacks to more than the limit.
///
/// ```no_run
/// use hpr_net::{Cache, Client, Http, Mode, Source};
///
/// let dir = Cache::platform_dir().expect("a home or local app data folder");
/// let client = Client::new(Http::new(), Cache::new(dir), Mode::Online);
/// let source = Source {
///     name: "Example".into(),
///     attribution: "Example data".into(),
///     ttl_s: 3600,
/// };
/// let now_s = std::time::SystemTime::now()
///     .duration_since(std::time::UNIX_EPOCH)
///     .map_or(0, |d| d.as_secs());
/// let answer = client.fetch(&source, "https://example.com/", now_s)?;
/// println!("{} bytes, {:?}", answer.body.len(), answer.freshness);
/// # Ok::<(), hpr_net::NetError>(())
/// ```
#[derive(Debug)]
pub struct Http {
    agent: ureq::Agent,
    max_body_bytes: u64,
}

impl Http {
    /// The time a whole request may take, connecting and reading the body included: 60 s.
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
    /// The largest body read: 64 MiB, room for a forecast grid, far past any JSON answer.
    pub const DEFAULT_MAX_BODY_BYTES: u64 = 64 * 1024 * 1024;

    /// A transport with [`Self::DEFAULT_TIMEOUT`] and [`Self::DEFAULT_MAX_BODY_BYTES`].
    pub fn new() -> Self {
        Self::with_limits(Self::DEFAULT_TIMEOUT, Self::DEFAULT_MAX_BODY_BYTES)
    }

    /// A transport that gives up on a request after `timeout` and refuses a body longer than
    /// `max_body_bytes` once unpacked.
    pub fn with_limits(timeout: Duration, max_body_bytes: u64) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .user_agent(Self::user_agent())
            .build()
            .new_agent();
        Self {
            agent,
            max_body_bytes,
        }
    }

    /// The `User-Agent` header sent: the crate's name and version and the project's address, so a
    /// data provider can tell who is calling.
    pub fn user_agent() -> String {
        format!(
            "hpr-sim/{} (+https://github.com/nrdptel/hpr-sim)",
            env!("CARGO_PKG_VERSION")
        )
    }
}

impl Default for Http {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for Http {
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        let mut response = self.agent.get(url).call().map_err(|e| e.to_string())?;
        // The limit is on the unpacked body. ureq's own limit counts the bytes on the wire, inside
        // the gzip decoder, so a small compressed body could unpack past it; reading one byte past
        // the limit from the unpacked stream bounds both.
        let unpacked = response.body_mut().with_config().limit(u64::MAX).reader();
        let mut body = Vec::new();
        unpacked
            .take(self.max_body_bytes.saturating_add(1))
            .read_to_end(&mut body)
            .map_err(|e| e.to_string())?;
        if body.len() as u64 > self.max_body_bytes {
            return Err(format!(
                "the body is longer than the {} bytes allowed",
                self.max_body_bytes
            ));
        }
        Ok(body)
    }
}
