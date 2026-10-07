impl<'a> SqliteRagRepositorySql<'a> {
    pub(crate) fn hydrate_graph_tree_rows_from_text_units(
        &self,
        corpus: &crate::domain::rag::RagCorpusRecord,
        rows: Vec<RagSearchRow>,
    ) -> Result<Vec<RagSearchRow>, RagError> {
        let plan = compile_sql_corpus_plan(self, corpus)?;
        let mut hydrated = Vec::new();
        for row in rows {
            hydrated.extend(hydrate_graph_tree_row_from_text_units(self, &plan, row)?);
        }
        Ok(dedup_tree_rows(hydrated))
    }
}

fn hydrate_graph_tree_row_from_text_units(
    repository: &SqliteRagRepositorySql<'_>,
    plan: &SqliteRagCorpusPlan,
    row: RagSearchRow,
) -> Result<Vec<RagSearchRow>, RagError> {
    let covered_codes = covered_text_unit_codes(&row);
    if covered_codes.is_empty() {
        return Ok(Vec::new());
    }
    let table = quote_identifier(&plan.text_unit_table)?;
    let status_column = quote_identifier(&plan.text_unit_status_column)?;
    let search_text_expr = if plan.text_unit_columns.contains("search_text") {
        "tu.search_text".to_string()
    } else {
        "tu.text".to_string()
    };
    let metadata_expr = plan
        .text_unit_metadata_column
        .as_deref()
        .map(|column| quote_identifier(column).map(|column| format!("tu.{column}")))
        .transpose()?
        .unwrap_or_else(|| "'{}'".into());
    let placeholders = covered_codes
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT
            tu.text_unit_code,
            tu.text,
            {search_text_expr},
            tu.heading_path,
            tu.ordinal,
            tu.token_count,
            {metadata_expr}
         FROM {table} tu
         WHERE tu.source_code = ?
           AND tu.source_version = ?
           AND tu.scope_type = ?
           AND tu.scope_code = ?
           AND tu.{status_column} = 'ACTIVE'
           AND tu.text_unit_code IN ({placeholders})
         ORDER BY tu.ordinal"
    );
    let mut params = vec![
        SqlValue::Text(row.source_code.clone()),
        SqlValue::Text(row.source_version.clone()),
        SqlValue::Text(row.scope_type.clone()),
        SqlValue::Text(row.scope_code.clone()),
    ];
    params.extend(covered_codes.into_iter().map(SqlValue::Text));
    let mut statement = repository.connection.prepare(&sql).map_err(storage_error)?;
    statement
        .query_map(params_from_iter(params), |unit| {
            let mut hydrated = row.clone();
            hydrated.target_code = unit.get(0)?;
            hydrated.text = unit.get(1)?;
            hydrated.search_text = unit.get(2)?;
            hydrated.heading_path = split_heading_path(&unit.get::<_, String>(3)?);
            hydrated.ordinal = unit.get(4)?;
            hydrated.token_count = unit.get::<_, Option<i64>>(5)?.unwrap_or(0);
            hydrated.embedding = None;
            hydrated.text_metadata =
                parse_metadata_json(unit.get::<_, Option<String>>(6)?.as_deref().unwrap_or("{}"));
            hydrated.field_values = tree_output_field_values(&row);
            Ok(hydrated)
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)
}

fn covered_text_unit_codes(row: &RagSearchRow) -> Vec<String> {
    row.field_values
        .get("coveredTextUnitCodes")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn tree_output_field_values(row: &RagSearchRow) -> Map<String, Value> {
    let mut values = row.field_values.clone();
    values.remove("coveredTextUnitCodes");
    values
}

impl SqliteRagRepository {
    pub(crate) fn hydrate_graph_tree_rows_from_text_units(
        &self,
        corpus: &crate::domain::rag::RagCorpusRecord,
        rows: Vec<RagSearchRow>,
    ) -> Result<Vec<RagSearchRow>, RagError> {
        let corpus = corpus.to_owned();
        self.connection.call(
            move |connection| {
                SqliteRagRepositorySql { connection }
                    .hydrate_graph_tree_rows_from_text_units(&corpus, rows)
            },
            || storage_error(rusqlite::Error::InvalidQuery),
        )
    }
}
