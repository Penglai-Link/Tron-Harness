# Feature Evolution Log: RAG Skill Graph Policy

## Summary

- feature_slug: rag-skill-graph-policy
- upstream_feature: rag-skill-graph-corpus
- status: done
- change_type: feature_evolution

## Change History

| change_id | description | status |
| --- | --- | --- |
| CH-RSGP-01 | Moved related graph kind and relationship policy from production defaults to profile config. | done |
| CH-RSGP-02 | Added field-mapped graph business property write and search return behavior. | done |
| CH-RSGP-03 | Added edge scalar property write and expansion return behavior. | done |

## Downstream Artifact Migration

- Existing `skill-graph` docs remain valid for base corpus capability.
- This slice adds separate policy acceptance rows and evidence.
- No frontend, API endpoint, or database table artifact migration is required.

## Downstream Impact

- Existing `skill` corpus remains SQLite-backed.
- Existing memory and project-doc corpora remain unchanged.
- New behavior only affects graph-backed RAG corpus writes and graph expansion metadata.

## Residual Risks

- Label allowlist remains out of scope by user decision.
- Rich filter operators for graph properties remain future work.
