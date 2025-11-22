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
pub mod config;
pub mod orchestrator;
pub mod pipes;
pub mod proxy;
