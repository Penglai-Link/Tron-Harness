# API Contract: Ontology Store RDF Domain Contract

## API Overview

This feature defines internal Rust APIs for ontology operations. It adds `OntologyService` and `OntologyStore` contracts for internal callers, but it adds no HTTP route, controller, frontend API contract, SPARQL endpoint, or raw RDF endpoint.

## Endpoints

- none
- No REST, GraphQL, SPARQL, RDF, Turtle, or SHACL text endpoint is introduced.

## Internal Rust Contract

| type | layer | role |
| --- | --- | --- |
| `OntologyDomain` | domain | operations, development, test, design, or custom namespace domain |
| `OntologyClass` | domain | class resource |
| `OntologyProperty` | domain | property resource with domain and range |
| `OntologyConstraint` | domain | typed validation constraint |
| `OntologyShaclShape` | domain | typed SHACL shape |
| `OntologySearchQuery` | domain | typed ontology search filters |
| `OntologyStore` | application port | storage abstraction for ontology CRUD and search |
| `OntologyService` | application service | validates commands, resolves scope, and calls store port |

## Auth And Permissions

- No HTTP authentication or role authorization is added.
- Future callers remain responsible for permissions before calling ontology services.
- Ontology visibility uses the resolved graph scope passed to every store call.

## Error Contract

| error | condition |
| --- | --- |
| `InvalidOntologyCode` | resource code is blank or malformed |
| `InvalidOntologyDomain` | ontology domain is blank or invalid |
| `InvalidOntologyResource` | typed resource misses required fields |
| `DuplicateOntologyResource` | store reports duplicate resource |
| `MissingOntologyResource` | get, update, delete, or relationship reference misses resource |
| `RawRdfNotAllowed` | caller attempts raw RDF/Turtle/SHACL text input |
| `SparqlNotAllowed` | caller attempts raw SPARQL |
| `StorageFailure` | store port reports unrecoverable storage failure |

## Frontend Backend Boundary

- No frontend code changes.
- No frontend mock routes are converted to real backend routes.
- Browser evidence is not required.

## Database And Transaction Boundary

- No database transaction is opened by this feature.
- `OntologyService` validates before calling the store port.
- Future Grafeo adapter owns RDF persistence and transaction semantics.

## Mock Or Adapter Expectations

- Tests use fake `OntologyStore` and fake scope resolver implementations.
- The fake store must prove service-to-port calls include `ResolvedGraphScope` and typed ontology resources.
- Concrete Grafeo RDF implementation is deferred to `embedded-grafeo-adapter-foundation`.
