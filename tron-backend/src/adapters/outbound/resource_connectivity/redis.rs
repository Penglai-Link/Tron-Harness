use redis::{
    Client, ConnectionAddr, ConnectionInfo, ProtocolVersion, RedisConnectionInfo,
    cluster::ClusterClient, cmd,
};

use crate::application::resource::{
    ConnectionEndpoint, ConnectionTlsMaterial, RedisConnectionTarget, RedisTopology,
};

pub(super) async fn verify(target: RedisConnectionTarget) -> Result<(), ()> {
    match target.topology {
        RedisTopology::Standalone => verify_standalone(&target).await,
        RedisTopology::Sentinel => verify_sentinel(&target).await,
        RedisTopology::Cluster => verify_cluster(&target).await,
    }
}

async fn verify_standalone(target: &RedisConnectionTarget) -> Result<(), ()> {
    for endpoint in &target.endpoints {
        let info = connection_info(
            endpoint,
            target.database,
            target.username.clone(),
            target.password.clone(),
            &target.tls,
        );
        let Ok(client) = Client::open(info) else {
            continue;
        };
        let Ok(mut connection) = client.get_multiplexed_async_connection().await else {
            continue;
        };
        if cmd("PING")
            .query_async::<String>(&mut connection)
            .await
            .is_ok()
        {
            return Ok(());
        }
    }
    Err(())
}

async fn verify_sentinel(target: &RedisConnectionTarget) -> Result<(), ()> {
    let sentinel = target.sentinel.as_ref().ok_or(())?;
    for endpoint in &target.endpoints {
        let info = connection_info(
            endpoint,
            0,
            sentinel.username.clone(),
            sentinel.password.clone(),
            &sentinel.tls,
        );
        let Ok(client) = Client::open(info) else {
            continue;
        };
        let Ok(mut connection) = client.get_multiplexed_async_connection().await else {
            continue;
        };
        let master: Result<Vec<String>, _> = cmd("SENTINEL")
            .arg("get-master-addr-by-name")
            .arg(&sentinel.master_name)
            .query_async(&mut connection)
            .await;
        let Ok(master) = master else {
            continue;
        };
        if master.len() != 2 {
            continue;
        }
        let Ok(port) = master[1].parse::<u16>() else {
            continue;
        };
        let master_target = RedisConnectionTarget {
            topology: RedisTopology::Standalone,
            endpoints: vec![ConnectionEndpoint {
                host: master[0].clone(),
                port,
            }],
            database: target.database,
            username: target.username.clone(),
            password: target.password.clone(),
            tls: target.tls.clone(),
            sentinel: None,
        };
        if verify_standalone(&master_target).await.is_ok() {
            return Ok(());
        }
    }
    Err(())
}

async fn verify_cluster(target: &RedisConnectionTarget) -> Result<(), ()> {
    let nodes = target
        .endpoints
        .iter()
        .map(|endpoint| {
            connection_info(
                endpoint,
                0,
                target.username.clone(),
                target.password.clone(),
                &target.tls,
            )
        })
        .collect::<Vec<_>>();
    let mut builder = ClusterClient::builder(nodes);
    if let Some(username) = &target.username {
        builder = builder.username(username.clone());
    }
    if let Some(password) = &target.password {
        builder = builder.password(password.clone());
    }
    let client = builder.build().map_err(|_| ())?;
    let mut connection = client.get_async_connection().await.map_err(|_| ())?;
    cmd("PING")
        .query_async::<String>(&mut connection)
        .await
        .map(|_| ())
        .map_err(|_| ())
}

fn connection_info(
    endpoint: &ConnectionEndpoint,
    database: i64,
    username: Option<String>,
    password: Option<String>,
    tls: &ConnectionTlsMaterial,
) -> ConnectionInfo {
    let addr = if tls.mode == "DISABLED" {
        ConnectionAddr::Tcp(endpoint.host.clone(), endpoint.port)
    } else {
        ConnectionAddr::TcpTls {
            host: endpoint.host.clone(),
            port: endpoint.port,
            insecure: tls.mode == "REQUIRED",
            tls_params: None,
        }
    };
    ConnectionInfo {
        addr,
        redis: RedisConnectionInfo {
            db: database,
            username,
            password,
            protocol: ProtocolVersion::RESP2,
        },
    }
}
