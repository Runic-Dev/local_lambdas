/// Infrastructure layer - external frameworks and tools
pub mod pipes;
pub mod http_client;

pub use pipes::NamedPipeClient;
pub use http_client::HttpClient;
