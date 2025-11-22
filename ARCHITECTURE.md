# Clean Architecture Design

This document explains the Clean Architecture implementation in local_lambdas.

## Overview

The application follows Clean Architecture principles to achieve:
- **Independence from Frameworks**: Business logic doesn't depend on external frameworks
- **Testability**: Business rules can be tested without UI, database, or external elements
- **Independence from UI**: UI can change without changing the rest of the system
- **Independence from Database**: Business rules aren't bound to data storage
- **Independence from External Agency**: Business rules don't know about the outside world

## Architecture Layers

### 1. Domain Layer (Innermost - Core Business Logic)

**Location**: `src/domain/`

Contains pure business logic with no external dependencies.

#### Entities (`src/domain/entities.rs`)
- `Process`: Core business entity representing a managed process
- `ProcessId`, `Executable`, `Route`, `PipeName`: Value objects with validation
- `HttpRequest`, `HttpResponse`: Domain representations of HTTP communication
- `HttpMethod`: Enum for HTTP methods
- `DomainError`: Domain-specific errors

**Key Principles**:
- No dependencies on outer layers
- Pure business logic and validation
- Immutable value objects
- Self-contained validation logic

#### Repository Interfaces (`src/domain/repositories.rs`)
- `ProcessRepository`: Interface for loading process configurations
- `ProcessOrchestrationService`: Interface for managing process lifecycle
- `PipeCommunicationService`: Interface for named pipe communication

**Key Principles**:
- Define contracts without implementation (Dependency Inversion Principle)
- Use async traits for non-blocking operations
- Domain-specific error types

### 2. Use Cases Layer (Application Business Rules)

**Location**: `src/use_cases/mod.rs`

Contains application-specific business rules that orchestrate the flow of data.

#### Use Cases
- `InitializeSystemUseCase`: Load all process configurations
- `StartAllProcessesUseCase`: Start all registered processes
- `StopAllProcessesUseCase`: Stop all running processes
- `ProxyHttpRequestUseCase`: Route HTTP requests to appropriate processes

**Key Principles**:
- Depends only on domain layer
- Orchestrates domain entities and repository interfaces
- Application-specific business logic
- Use case errors separate from domain errors

### 3. Adapters Layer (Interface Adapters)

**Location**: `src/adapters/`

Converts data between formats most convenient for use cases and external agencies.

#### Config Adapter (`src/adapters/config/`)
- `XmlProcessRepository`: Implements `ProcessRepository` using XML files
- Converts XML DTOs to domain entities
- Handles file I/O and XML parsing

#### Process Adapter (`src/adapters/process/`)
- `TokioProcessOrchestrator`: Implements `ProcessOrchestrationService`
- Manages process lifecycle using tokio
- Converts domain entities to system process commands

#### HTTP Adapter (`src/adapters/http/`)
- `HttpServerState`: Axum-based HTTP server
- Converts Axum HTTP types to domain types
- Routes requests to use cases
- Converts domain responses back to Axum responses

**Key Principles**:
- Implement domain interfaces
- Convert between external formats and domain models
- No business logic (only translation)

### 4. Infrastructure Layer (Frameworks & Drivers)

**Location**: `src/infrastructure/`

Contains implementations using specific frameworks and tools.

#### Named Pipes (`src/infrastructure/pipes.rs`)
- `NamedPipeClient`: Platform-specific named pipe implementation
- Handles Unix domain sockets and Windows named pipes
- Implements `PipeCommunicationService`

**Key Principles**:
- Framework-specific code
- External agency integration
- Cross-platform abstractions

## Dependency Flow

```
main.rs (Entry Point)
    ↓
Infrastructure Layer (NamedPipeClient)
    ↓
Adapters Layer (XmlProcessRepository, TokioProcessOrchestrator, HttpServerState)
    ↓
Use Cases Layer (InitializeSystemUseCase, ProxyHttpRequestUseCase, etc.)
    ↓
Domain Layer (Entities, Repository Interfaces)
```

**Key Rule**: Dependencies point inward. Outer layers depend on inner layers, never the reverse.

## Dependency Inversion

The application uses **Dependency Inversion Principle**:

1. **Domain defines interfaces** (e.g., `ProcessRepository`, `ProcessOrchestrationService`)
2. **Adapters implement interfaces** (e.g., `XmlProcessRepository`, `TokioProcessOrchestrator`)
3. **Use cases depend on interfaces**, not implementations
4. **Main wires everything together** through dependency injection

Example in `main.rs`:
```rust
// Create infrastructure implementations
let process_repository = Arc::new(XmlProcessRepository::new(&manifest_path));
let pipe_service = Arc::new(NamedPipeClient::new());

// Inject into use cases
let init_use_case = InitializeSystemUseCase::new(process_repository.clone());
let proxy_use_case = ProxyHttpRequestUseCase::new(pipe_service, processes_arc);
```

## Benefits

1. **Testability**: Each layer can be tested independently with mocks
2. **Maintainability**: Changes in one layer don't affect others
3. **Flexibility**: Easy to swap implementations (e.g., XML → JSON config)
4. **Clarity**: Clear separation of concerns
5. **Independence**: Business logic independent of frameworks

## Testing Strategy

- **Domain Layer**: Unit tests for entities and value objects
- **Use Cases**: Unit tests with mocked repository interfaces
- **Adapters**: Integration tests with real implementations
- **System**: End-to-end tests for complete workflows

## Migration Path

The codebase maintains backward compatibility by keeping legacy modules:
- `src/config/` (legacy)
- `src/orchestrator/` (legacy)
- `src/pipes/` (legacy)
- `src/proxy/` (legacy)

These are gradually being phased out as the Clean Architecture implementation matures.

## Adding New Features

When adding new features, follow this process:

1. **Start with Domain**: Define entities and interfaces
2. **Create Use Case**: Implement application logic
3. **Build Adapter**: Implement interface for external agency
4. **Wire in Main**: Connect everything via dependency injection
5. **Test Each Layer**: Unit tests → Integration tests → E2E tests

## References

- [Clean Architecture by Robert C. Martin](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
- [Hexagonal Architecture](https://alistair.cockburn.us/hexagonal-architecture/)
- [Dependency Inversion Principle](https://en.wikipedia.org/wiki/Dependency_inversion_principle)
