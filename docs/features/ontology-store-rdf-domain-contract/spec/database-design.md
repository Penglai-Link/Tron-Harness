# Database Design: Ontology Store RDF Domain Contract

## Database Decision

- status: not_applicable
- mandatory_database_change: no
- rationale: this feature defines typed RDF ontology contracts only; concrete Grafeo RDF persistence belongs to the embedded adapter feature.

## Existing Schema Impact

- Existing SQLite tables remain unchanged.
- Existing graph scope and LPG contracts remain unchanged.
- No Grafeo RDF graph, triple store, index, or named graph is created here.

## Proposed Schema

- no SQL table
- no SQL migration
- no Grafeo dependency
- no RDF storage object

Future adapter work will map typed ontology resources into Grafeo RDF/LPG capability after embedded API verification.

## Relationships And Cardinality

- One resolved scope contains many ontology resources.
- One ontology property references one or more class domains and one or more ranges.
- One SHACL shape targets one class and contains one or more property constraints.
- One ontology search query can filter by resource type, ontology domain, code, label, or target class.

## Migration Plan

- No migration is required.
- Future Grafeo adapter work must add RDF capability verification and storage contract tests.

## Query And Performance Plan

- This feature defines query shape only.
- Search queries include limits so future adapter behavior can be bounded.
- Typed filters allow the adapter to map fields to RDF or property graph indexes later.

## Data Governance

- Raw RDF, Turtle, RDF/XML, SHACL text, and SPARQL strings are not accepted.
- Ontology resource codes and labels are safe to log, but full future RDF payloads are not introduced here.
- Scope fields continue to enforce namespace, workspace lineage, app, and user boundaries.

## Test Data And Fixtures

- Unit tests use fake scope resolver and fake ontology store.
- Fixtures should include class, property, domain, range, constraint, SHACL shape, duplicate, missing, invalid, and search cases.
- No SQL or Grafeo fixture is required.

## Validation Commands

- `CMD-BE-ONTOLOGY-UNIT` proves typed ontology validation, port calls, scope propagation, duplicate/missing mapping, and raw query shielding.
- `CMD-BE-ONTOLOGY-GUARD` proves no HTTP, Grafeo, raw RDF/SPARQL, or `inheritable` surface is added.
- `CMD-BE-CHECK` proves backend compile through Makefile.
- `CMD-BE-FMT` proves formatting.
