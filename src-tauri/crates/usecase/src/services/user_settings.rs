use foundation::AppResult;
use foundation::models::{UserSettingsDto, UserSettingsUpdateInputDto};

#[derive(Debug, Clone, Copy, Default)]
pub struct UserSettingsApplicationService;

impl UserSettingsApplicationService {
    pub fn load_or_init(self) -> AppResult<UserSettingsDto> {
        crate::user_settings::load_or_init_user_settings()
    }

    pub fn update(self, input: UserSettingsUpdateInputDto) -> AppResult<UserSettingsDto> {
        crate::user_settings::update_user_settings(input)
    }

    pub fn update_locale_preference(self, preference: &str) -> AppResult<UserSettingsDto> {
        crate::user_settings::update_locale_preference(preference)
    }
}
