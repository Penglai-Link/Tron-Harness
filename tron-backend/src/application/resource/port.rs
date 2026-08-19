use crate::domain::resource::{
    DataSourceConnectionRecord, ResourceError, ResourceFileRecord, ServerAccountRecord,
    ServerRecord,
};

pub trait ResourceRepository: Send {
    fn list_files(&mut self) -> Result<Vec<ResourceFileRecord>, ResourceError>;
    fn insert_file(&mut self, record: &ResourceFileRecord) -> Result<(), ResourceError>;
    fn update_file(&mut self, record: &ResourceFileRecord) -> Result<bool, ResourceError>;
    fn delete_file(
        &mut self,
        workspace_code: &str,
        resource_code: &str,
    ) -> Result<Option<ResourceFileRecord>, ResourceError>;

    fn list_data_source_connections(
        &mut self,
    ) -> Result<Vec<DataSourceConnectionRecord>, ResourceError>;
    fn insert_data_source_connection(
        &mut self,
        record: &DataSourceConnectionRecord,
    ) -> Result<(), ResourceError>;
    fn update_data_source_connection(
        &mut self,
        record: &DataSourceConnectionRecord,
    ) -> Result<bool, ResourceError>;
    fn delete_data_source_connection(
        &mut self,
        workspace_code: &str,
        resource_code: &str,
    ) -> Result<Option<DataSourceConnectionRecord>, ResourceError>;

    fn list_servers(&mut self) -> Result<Vec<ServerRecord>, ResourceError>;
    fn insert_server(&mut self, record: &ServerRecord) -> Result<(), ResourceError>;
    fn update_server(&mut self, record: &ServerRecord) -> Result<bool, ResourceError>;
    fn delete_server(
        &mut self,
        workspace_code: &str,
        resource_code: &str,
    ) -> Result<Option<ServerRecord>, ResourceError>;

    fn list_server_accounts(&mut self) -> Result<Vec<ServerAccountRecord>, ResourceError>;
    fn insert_server_account(&mut self, record: &ServerAccountRecord) -> Result<(), ResourceError>;
    fn update_server_account(&mut self, record: &ServerAccountRecord) -> Result<bool, ResourceError>;
    fn delete_server_account(
        &mut self,
        workspace_code: &str,
        resource_code: &str,
    ) -> Result<Option<ServerAccountRecord>, ResourceError>;
}

pub trait ObjectStorage: Send + Sync {
    fn put_file(&self, object_key: &str, source: &std::path::Path) -> Result<u64, ResourceError>;
    fn put_reader(
        &self,
        object_key: &str,
        source: &mut dyn std::io::Read,
    ) -> Result<u64, ResourceError>;
    fn get_file(&self, object_key: &str, target: &std::path::Path) -> Result<u64, ResourceError>;
    fn get_writer(
        &self,
        object_key: &str,
        target: &mut dyn std::io::Write,
    ) -> Result<u64, ResourceError>;
    fn delete(&self, object_key: &str) -> Result<(), ResourceError>;
    fn exists(&self, object_key: &str) -> Result<bool, ResourceError>;
}
