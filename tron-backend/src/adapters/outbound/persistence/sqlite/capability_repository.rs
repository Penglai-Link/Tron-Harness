use super::framework_storage::SharedDatabase;
use crate::application::capability::{CapabilityResource, CapabilityResourceRepository};
use crate::domain::computer_use;
use rusqlite::{Connection, params};
pub struct SqliteCapabilityRepository(SharedDatabase);
impl SqliteCapabilityRepository {
    pub fn connect(url: &str) -> Result<Self, String> {
        SharedDatabase::open(url)
            .map(Self)
            .map_err(|e| e.to_string())
    }
}
fn failure(e: rusqlite::Error) -> String {
    e.to_string()
}
fn resources(c: &Connection) -> Result<Vec<CapabilityResource>, String> {
    let mut q = c.prepare("SELECT pack_id,resource_kind,resource_code,available FROM capability_resource_ownership").map_err(failure)?;
    q.query_map([], |r| {
        Ok(CapabilityResource {
            pack_id: r.get(0)?,
            kind: r.get(1)?,
            code: r.get(2)?,
            available: r.get(3)?,
        })
    })
    .map_err(failure)?
    .collect::<Result<Vec<_>, _>>()
    .map_err(failure)
}
impl CapabilityResourceRepository for SqliteCapabilityRepository {
    fn list(&mut self) -> Result<Vec<CapabilityResource>, String> {
        self.0
            .call(|c| resources(c), || "capability storage unavailable".into())
    }
    fn register(&mut self, items: &[CapabilityResource]) -> Result<(), String> {
        let items = items.to_vec();
        self.0.call(move |c| {
            let tx=c.transaction().map_err(failure)?;
            let existing=resources(&tx)?;
            let controlled = items.iter().filter(|resource| resource.pack_id == computer_use::PACK_ID).cloned().collect::<Vec<_>>();
            let introduced = controlled.iter().filter(|resource| resource.available
                && !existing.iter().any(|old| old.kind == resource.kind && old.code == resource.code)).cloned().collect::<Vec<_>>();
            for r in items {
                if existing.iter().any(|e| e.kind==r.kind && e.code==r.code && e.pack_id!=r.pack_id) { return Err(format!("resource {} conflicts with another capability pack",r.code)); }
                tx.execute("INSERT INTO capability_resource_ownership VALUES(?1,?2,?3,?4) ON CONFLICT(resource_kind,resource_code) DO UPDATE SET available=excluded.available",params![r.pack_id,r.kind,r.code,r.available]).map_err(failure)?;
            }
            if !introduced.is_empty() { update_assignments(&tx, &introduced, true, false)?; }
            if !controlled.is_empty() { update_assignments(&tx, &controlled, true, true)?; }
            tx.commit().map_err(failure)
        },||"capability storage unavailable".into())
    }
    fn transition(&mut self, pack: &str, enabled: bool, agents: &[String]) -> Result<(), String> {
        let pack = pack.to_owned();
        let agents = agents.iter().filter(|agent| pack != computer_use::PACK_ID || computer_use::allows(agent)).cloned().collect::<Vec<_>>();
        self.0.call(move |c| {
            let tx=c.transaction().map_err(failure)?;
            let owned=resources(&tx)?.into_iter().filter(|r|r.pack_id==pack).collect::<Vec<_>>();
            let changed=owned.iter().any(|r|r.available!=enabled);
            tx.execute("UPDATE capability_resource_ownership SET available=?2 WHERE pack_id=?1",params![pack,enabled]).map_err(failure)?;
            for r in &owned { if r.kind=="mcp" { tx.execute("UPDATE platform_mcp_servers SET enabled=?2 WHERE server_code=?1",params![r.code,enabled]).map_err(failure)?; } }
            if !enabled || changed {
                update_assignments(&tx,&owned,enabled,false)?;
                tx.execute("UPDATE registry_skills SET available=?2 WHERE provider_pack_id=?1",params![pack,enabled]).map_err(failure)?;
                tx.execute("UPDATE agent_skill_assignments SET enabled=0 WHERE skill_name IN (SELECT skill_name FROM registry_skills WHERE provider_pack_id=?1)",[&pack]).map_err(failure)?;
                if enabled {for agent in &agents {tx.execute("INSERT INTO agent_skill_assignments SELECT ?1,skill_name,1 FROM registry_skills WHERE provider_pack_id=?2 ON CONFLICT(agent_code,skill_name) DO UPDATE SET enabled=1",params![agent,pack]).map_err(failure)?;}}
            }
            tx.commit().map_err(failure)
        },||"capability storage unavailable".into())
    }
    fn remove(&mut self, pack: &str) -> Result<(), String> {
        let pack = pack.to_owned();
        self.0.call(
            move |c| {
                let tx = c.transaction().map_err(failure)?;
                let owned = resources(&tx)?
                    .into_iter()
                    .filter(|r| r.pack_id == pack)
                    .collect::<Vec<_>>();
                update_assignments(&tx, &owned, false, false)?;
                for r in &owned {
                    match r.kind.as_str() {
                        "mcp" => {
                            tx.execute(
                                "DELETE FROM platform_mcp_servers WHERE server_code=?1",
                                [&r.code],
                            )
                            .map_err(failure)?;
                        }
                        "tool" => {
                            tx.execute(
                                "DELETE FROM platform_tool_permissions WHERE tool_code=?1",
                                [&r.code],
                            )
                            .map_err(failure)?;
                            tx.execute(
                                "DELETE FROM platform_discovered_tools WHERE tool_code=?1",
                                [&r.code],
                            )
                            .map_err(failure)?;
                        }
                        _ => {}
                    }
                }
                tx.execute(
                    "DELETE FROM capability_resource_ownership WHERE pack_id=?1",
                    [pack],
                )
                .map_err(failure)?;
                tx.commit().map_err(failure)
            },
            || "capability storage unavailable".into(),
        )
    }
}
fn update_assignments(
    c: &Connection,
    owned: &[CapabilityResource],
    enabled: bool,
    only_other_controllers: bool,
) -> Result<(), String> {
    let controlled = owned.iter().filter(|resource| resource.pack_id == computer_use::PACK_ID)
        .map(|resource| resource.code.as_str()).collect::<std::collections::HashSet<_>>();
    if !controlled.is_empty() {
        c.execute(
            "UPDATE workspace_internal_agent_policies
             SET selected_category_codes=(SELECT COALESCE(json_group_array(value),'[]') FROM json_each(selected_category_codes) WHERE value<>?1)
             WHERE agent_code NOT IN (?2,?3) AND EXISTS(SELECT 1 FROM json_each(selected_category_codes) WHERE value=?1)",
            params![computer_use::DOMAIN, computer_use::CONTROLLERS[0], computer_use::CONTROLLERS[1]],
        ).map_err(failure)?;
    }
    let tools = owned
        .iter()
        .filter(|r| r.kind == "tool")
        .map(|r| r.code.clone())
        .collect::<Vec<_>>();
    let mcp = owned
        .iter()
        .filter(|r| r.kind == "mcp")
        .map(|r| r.code.clone())
        .collect::<Vec<_>>();
    let mut q = c
        .prepare("SELECT agent_code,config_json FROM platform_internal_agent_configs")
        .map_err(failure)?;
    let configs = q
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(failure)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(failure)?;
    drop(q);
    for (agent, json) in configs {
        if only_other_controllers && computer_use::allows(&agent) { continue; }
        let mut config: serde_json::Value =
            serde_json::from_str(&json).map_err(|e| e.to_string())?;
        for (key, codes) in [("selectedToolCodes", &tools), ("selectedServerCodes", &mcp)] {
            let values = config[key].as_array().cloned().unwrap_or_default();
            let mut values = values
                .into_iter()
                .filter(|v| !v.as_str().is_some_and(|v| codes.iter().any(|c| c == v)))
                .collect::<Vec<_>>();
            if enabled {
                values.extend(codes.iter().filter(|code| !controlled.contains(code.as_str()) || computer_use::allows(&agent)).cloned().map(serde_json::Value::String));
            }
            config[key] = serde_json::Value::Array(values);
        }
        if !controlled.is_empty() && !computer_use::allows(&agent) {
            if let Some(categories) = config["selectedCategoryCodes"].as_array_mut() {
                categories.retain(|category| category.as_str() != Some(computer_use::DOMAIN));
            }
        }
        c.execute(
            "UPDATE platform_internal_agent_configs SET config_json=?2 WHERE agent_code=?1",
            params![agent, config.to_string()],
        )
        .map_err(failure)?;
    }
    for (table, column, codes) in [
        (
            "workspace_internal_agent_tool_assignments",
            "tool_code",
            &tools,
        ),
        (
            "workspace_internal_agent_mcp_server_assignments",
            "server_code",
            &mcp,
        ),
    ] {
        for code in codes {
            c.execute(&format!("DELETE FROM {table} WHERE {column}=?1 AND (?2=0 OR agent_code NOT IN (?3,?4))"), params![code,only_other_controllers,computer_use::CONTROLLERS[0],computer_use::CONTROLLERS[1]])
                .map_err(failure)?;
            if enabled && !only_other_controllers {
                c.execute(&format!("INSERT OR IGNORE INTO {table}(workspace_code,agent_code,{column},created_at,updated_at) SELECT workspace_code,agent_code,?1,?2,?2 FROM workspace_internal_agent_policies WHERE (?3=0 OR agent_code IN (?4,?5))"),params![code,chrono::Utc::now().timestamp_millis(),controlled.contains(code.as_str()),computer_use::CONTROLLERS[0],computer_use::CONTROLLERS[1]]).map_err(failure)?;
            }
        }
    }
    Ok(())
}
