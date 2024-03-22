use std::collections::HashMap;

use ginepro::LoadBalancedChannel;
use tonic::{transport::ClientTlsConfig, Request, Response, Status, Streaming};
use tracing::{debug, instrument};

use crate::{pb::fmaas::{
    generation_service_client::GenerationServiceClient,
    generation_service_server::GenerationService, BatchedGenerationRequest,
    BatchedGenerationResponse, BatchedTokenizeRequest, BatchedTokenizeResponse,
    GenerationResponse, ModelInfoRequest, ModelInfoResponse, SingleGenerationRequest,
}, create_clients, ServiceAddr};

#[derive(Debug, Default)]
pub struct GenerationServicer {
    clients: HashMap<String, GenerationServiceClient<LoadBalancedChannel>>,
}

impl GenerationServicer {
    pub async fn new(
        default_target_port: u16,
        client_tls: Option<&ClientTlsConfig>,
        model_map: &HashMap<String, ServiceAddr>,
    ) -> Self {
        let clients = create_clients(
            default_target_port, client_tls, model_map, GenerationServiceClient::new
        ).await;
        Self { clients }
    }

    async fn client(
        &self,
        model_id: &str,
    ) -> Result<GenerationServiceClient<LoadBalancedChannel>, Status> {
        Ok(self
            .clients
            .get(model_id)
            .ok_or_else(|| Status::not_found(format!("Unrecognized model_id: {model_id}")))?
            .clone())
    }
}

#[tonic::async_trait]
impl GenerationService for GenerationServicer {
    #[instrument(skip_all)]
    async fn generate(
        &self,
        request: Request<BatchedGenerationRequest>,
    ) -> Result<Response<BatchedGenerationResponse>, Status> {
        let br = request.get_ref();
        if br.requests.is_empty() {
            return Ok(Response::new(BatchedGenerationResponse {
                responses: vec![],
            }));
        }
        debug!("Routing generation request for Model ID {}", &br.model_id);
        self.client(&br.model_id).await?.generate(request).await
    }

    type GenerateStreamStream = Streaming<GenerationResponse>;

    #[instrument(skip_all)]
    async fn generate_stream(
        &self,
        request: Request<SingleGenerationRequest>,
    ) -> Result<Response<Self::GenerateStreamStream>, Status> {
        let sr = request.get_ref();
        if sr.request.is_none() {
            return Err(Status::invalid_argument("missing request"));
        }
        debug!(
            "Routing streaming generation request for Model ID {}",
            &sr.model_id
        );
        self.client(&sr.model_id)
            .await?
            .generate_stream(request)
            .await
    }

    #[instrument(skip_all)]
    async fn tokenize(
        &self,
        request: Request<BatchedTokenizeRequest>,
    ) -> Result<Response<BatchedTokenizeResponse>, Status> {
        let br = request.get_ref();
        if br.requests.is_empty() {
            return Ok(Response::new(BatchedTokenizeResponse { responses: vec![] }));
        }
        debug!("Routing tokenization request for Model ID {}", &br.model_id);
        self.client(&br.model_id).await?.tokenize(request).await
    }

    #[instrument(skip_all)]
    async fn model_info(
        &self,
        request: Request<ModelInfoRequest>,
    ) -> Result<Response<ModelInfoResponse>, Status> {
        debug!(
            "Routing model info request for Model ID {}",
            &request.get_ref().model_id
        );
        self.client(&request.get_ref().model_id)
            .await?
            .model_info(request)
            .await
    }
}
