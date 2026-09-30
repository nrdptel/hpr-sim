//! The HTTP transport: blocking HTTP/1.1 through `ureq`, TLS through rustls.

use std::io::Read;
use std::time::Duration;

use ureq::{Proxy, ProxyProtocol};

use crate::Transport;

/// How an [`Http`] transport behaves. Start from [`HttpConfig::default`] and change fields.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpConfig {
    /// The time a whole request may take: finding the host, connecting, every redirect and
    /// reading the body. Longer than [`Http::MAX_TIMEOUT`] counts as that. Default 60 s.
    pub timeout: Duration,
    /// The longest body accepted, counted after any gzip unpacking; one byte more is refused.
    /// Default 64 MiB, room for a forecast grid and far past any JSON answer.
    pub max_body_bytes: u64,
    /// Whether to use the proxy the environment names (`ALL_PROXY`, `HTTPS_PROXY` or
    /// `HTTP_PROXY`, the first set, in either case, for every URL, bar hosts in `NO_PROXY`).
    /// Default `true`.
    pub proxy_from_env: bool,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(60),
            max_body_bytes: 64 * 1024 * 1024,
            proxy_from_env: true,
        }
    }
}

/// A [`Transport`] that fetches over HTTP and HTTPS, behind the crate's `http` feature.
///
/// TLS is rustls with Mozilla's root certificates compiled in (the `webpki-roots` crate), so it
/// needs no OpenSSL and ignores the operating system's certificate store. It follows up to ten
/// redirects and sends `Accept-Encoding: gzip`, unpacking gzip bodies. Any status but 2xx is an
/// error, as are a timeout and a body that unpacks to more than the limit. An HTTP or HTTPS proxy
/// from the environment is used; a SOCKS one is refused with an error rather than bypassed, since
/// this build cannot speak SOCKS.
///
/// Cloning is cheap and the clones share one pool of connections.
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
#[derive(Debug, Clone)]
pub struct Http {
    agent: ureq::Agent,
    max_body_bytes: u64,
    /// A SOCKS proxy the environment names, which every URL it covers is refused under.
    socks: Option<Proxy>,
}

impl Http {
    /// The `User-Agent` header sent: the crate's name and version and the project's address, so a
    /// data provider can tell who is calling.
    pub const USER_AGENT: &str = concat!(
        "hpr-sim/",
        env!("CARGO_PKG_VERSION"),
        " (+https://github.com/nrdptel/hpr-sim)"
    );
    /// The longest timeout used: 30 days. A longer one, such as [`Duration::MAX`] for "none",
    /// counts as this, since a deadline past the clock's range would overflow.
    pub const MAX_TIMEOUT: Duration = Duration::from_secs(30 * 24 * 3600);

    /// A transport with [`HttpConfig::default`].
    pub fn new() -> Self {
        Self::with_config(HttpConfig::default())
    }

    /// A transport with `config`.
    pub fn with_config(config: HttpConfig) -> Self {
        let proxy = if config.proxy_from_env {
            Proxy::try_from_env()
        } else {
            None
        };
        Self::with_proxy(&config, proxy)
    }

    fn with_proxy(config: &HttpConfig, proxy: Option<Proxy>) -> Self {
        let socks = proxy
            .clone()
            .filter(|p| !matches!(p.protocol(), ProxyProtocol::Http | ProxyProtocol::Https));
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(config.timeout.min(Self::MAX_TIMEOUT)))
            .user_agent(Self::USER_AGENT)
            .proxy(proxy)
            .build()
            .new_agent();
        Self {
            agent,
            max_body_bytes: config.max_body_bytes,
            socks,
        }
    }
}

impl Default for Http {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for Http {
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        if let Some(socks) = &self.socks {
            // Without ureq's SOCKS support it would warn and connect directly, around the proxy.
            let covered = url
                .parse::<ureq::http::Uri>()
                .is_ok_and(|uri| !socks.is_no_proxy(&uri));
            if covered {
                return Err(format!(
                    "the environment's proxy {} is SOCKS, which hpr-net cannot use; name an HTTP \
                     proxy in HTTPS_PROXY, or add the host to NO_PROXY",
                    socks.uri()
                ));
            }
        }
        let mut response = self.agent.get(url).call().map_err(|e| e.to_string())?;
        // ureq errors on 4xx and 5xx only; a 304 or an unfollowed 3xx would read as an empty body.
        let status = response.status();
        if !status.is_success() {
            return Err(format!("http status: {}", status.as_u16()));
        }
        // The limit is on the unpacked body, read one byte past it to tell "at" from "over".
        // ureq's own limit counts bytes on the wire, inside its gzip decoder, so a small compressed
        // body could unpack past it. Wire bytes that unpack to nothing are bounded by the timeout.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_socks_proxy_is_refused_not_bypassed() {
        let socks = Proxy::new("socks5://127.0.0.1:9").unwrap();
        let http = Http::with_proxy(&HttpConfig::default(), Some(socks));
        let reason = http.get("http://127.0.0.1:1/x").unwrap_err();
        assert!(reason.contains("is SOCKS"), "{reason}");
    }

    #[test]
    fn an_http_proxy_is_used_not_refused() {
        for url in ["http://127.0.0.1:9", "https://127.0.0.1:9"] {
            let proxy = Proxy::new(url).unwrap();
            let http = Http::with_proxy(&HttpConfig::default(), Some(proxy));
            assert!(http.socks.is_none(), "{url}");
        }
    }
}
