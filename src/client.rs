use anyhow::Result;
use tonic::transport::ClientTlsConfig;
use yellowstone_grpc_client::GeyserGrpcClient;
use crate::config::SingleConfig;

pub struct GrpcClient {
    config: SingleConfig,
}

impl GrpcClient {
    pub fn new(config: SingleConfig) -> Self {
        Self { config }
    }
    
    pub async fn connect(self) -> Result<GeyserGrpcClient<impl tonic::service::Interceptor>> {
        let mut builder = GeyserGrpcClient::build_from_shared(self.config.endpoint)?;
        
        if let Some(token) = self.config.access_token {
            builder = builder.x_token(Some(token))?;
        }
        
        let client = builder
            .tls_config(ClientTlsConfig::new().with_native_roots())?
            .connect()
            .await?;
            
        Ok(client)
    }
}