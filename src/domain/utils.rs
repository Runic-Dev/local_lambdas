//! Utility functions for communication addressing
//! These functions generate consistent addresses for different communication modes

/// Generate a deterministic HTTP port from a pipe name
/// Uses ports in the range 9000-9999
pub fn get_http_port_from_name(pipe_name: &str) -> u16 {
    let hash = pipe_name.bytes().fold(0u32, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(b as u32)
    });
    9000 + (hash % 1000) as u16
}

/// Generate HTTP address from pipe name
pub fn get_http_address_from_name(pipe_name: &str) -> String {
    let port = get_http_port_from_name(pipe_name);
    format!("127.0.0.1:{}", port)
}

/// Generate pipe address from pipe name based on platform
pub fn get_pipe_address_from_name(pipe_name: &str) -> String {
    #[cfg(windows)]
    {
        format!(r"\\.\pipe\{}", pipe_name)
    }

    #[cfg(unix)]
    {
        format!("/tmp/{}", pipe_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_port_generation_deterministic() {
        let port1 = get_http_port_from_name("test_pipe");
        let port2 = get_http_port_from_name("test_pipe");
        assert_eq!(port1, port2, "Port generation should be deterministic");
    }

    #[test]
    fn test_http_port_in_range() {
        let port = get_http_port_from_name("test_pipe");
        assert!(port >= 9000 && port < 10000, "Port should be in range 9000-9999");
    }

    #[test]
    fn test_different_names_different_ports() {
        let port1 = get_http_port_from_name("pipe_a");
        let port2 = get_http_port_from_name("pipe_b");
        // While not guaranteed, different names should usually produce different ports
        // This is a probabilistic test
        assert_ne!(port1, port2, "Different pipe names should likely produce different ports");
    }

    #[test]
    fn test_http_address_format() {
        let addr = get_http_address_from_name("test");
        assert!(addr.starts_with("127.0.0.1:"));
        let port_str = addr.split(':').nth(1).unwrap();
        let port: u16 = port_str.parse().unwrap();
        assert!(port >= 9000 && port < 10000, "Port should be in 9000-9999 range");
    }
}
