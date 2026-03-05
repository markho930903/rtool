use rtool_contracts::AppResult;
use rtool_contracts::models::{SettingsDto, SettingsUpdateInputDto};
use rtool_data::db::DbConn;

#[derive(Debug, Clone)]
pub struct SettingsApplicationService {
    db_conn: DbConn,
}

impl SettingsApplicationService {
    pub fn new(db_conn: DbConn) -> Self {
        Self { db_conn }
    }

    pub async fn load_or_init(&self) -> AppResult<SettingsDto> {
        rtool_settings::load_or_init_settings(&self.db_conn).await
    }

    pub async fn update(&self, input: SettingsUpdateInputDto) -> AppResult<SettingsDto> {
        rtool_settings::update_settings(&self.db_conn, input).await
    }

    pub async fn update_locale_preference(&self, preference: &str) -> AppResult<SettingsDto> {
        rtool_settings::update_locale_preference(&self.db_conn, preference).await
    }
}
