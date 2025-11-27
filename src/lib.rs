// Library exports following Clean Architecture principles

// Domain layer (core business logic)
pub mod domain;

// Use cases layer (application business rules)
pub mod use_cases;

// Adapters layer (interface adapters)
pub mod adapters;

// Infrastructure layer (frameworks & drivers)
pub mod infrastructure;

// Legacy modules for backward compatibility
#[allow(dead_code)]
pub mod config;
#[allow(dead_code)]
pub mod orchestrator;
#[allow(dead_code)]
pub mod pipes;
#[allow(dead_code)]
pub mod proxy;
