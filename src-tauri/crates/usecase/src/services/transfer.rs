use foundation::AppResult;
use foundation::models::{
    TransferClearHistoryInputDto, TransferHistoryFilterDto, TransferHistoryPageDto,
    TransferPairingCodeDto, TransferPeerDto, TransferRuntimeStatusDto, TransferSendFilesInputDto,
    TransferSessionDto, TransferSettingsDto, TransferUpdateSettingsInputDto,
};
use domain::service::TransferService;

#[derive(Clone)]
pub struct TransferApplicationService {
    service: TransferService,
}

impl TransferApplicationService {
    pub fn new(service: TransferService) -> Self {
        Self { service }
    }

    pub fn domain_service(&self) -> &TransferService {
        &self.service
    }

    pub fn get_settings(&self) -> TransferSettingsDto {
        self.service.get_settings()
    }

    pub async fn update_settings(
        &self,
        input: TransferUpdateSettingsInputDto,
    ) -> AppResult<TransferSettingsDto> {
        self.service.update_settings(input).await
    }

    pub fn generate_pairing_code(&self) -> TransferPairingCodeDto {
        self.service.generate_pairing_code()
    }

    pub fn start_discovery(&self) -> AppResult<()> {
        self.service.ensure_bootstrapped()?;
        self.service.start_discovery()
    }

    pub fn stop_discovery(&self) {
        self.service.stop_discovery();
    }

    pub async fn list_peers(&self) -> AppResult<Vec<TransferPeerDto>> {
        self.service.ensure_bootstrapped()?;
        self.service.list_peers().await
    }

    pub async fn send_files(
        &self,
        input: TransferSendFilesInputDto,
    ) -> AppResult<TransferSessionDto> {
        self.service.ensure_bootstrapped()?;
        self.service.send_files(input).await
    }

    pub async fn pause_session(&self, session_id: &str) -> AppResult<()> {
        self.service.pause_session(session_id).await
    }

    pub async fn resume_session(&self, session_id: &str) -> AppResult<()> {
        self.service.resume_session(session_id).await
    }

    pub async fn cancel_session(&self, session_id: &str) -> AppResult<()> {
        self.service.cancel_session(session_id).await
    }

    pub async fn retry_session(&self, session_id: &str) -> AppResult<TransferSessionDto> {
        self.service.ensure_bootstrapped()?;
        self.service.retry_session(session_id).await
    }

    pub async fn list_history(
        &self,
        filter: TransferHistoryFilterDto,
    ) -> AppResult<TransferHistoryPageDto> {
        self.service.list_history(filter).await
    }

    pub async fn clear_history(&self, input: TransferClearHistoryInputDto) -> AppResult<()> {
        self.service.clear_history(input).await
    }

    pub fn runtime_status(&self) -> TransferRuntimeStatusDto {
        self.service.runtime_status()
    }
}
