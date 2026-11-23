use crate::{
    business::{Actor, Error, Result},
    miniapp_users::{
        domain::phone::{BindPhoneInput, PhoneBinding},
        south::{
            adapter::{phone_cipher::PhoneCipher, postgres::Repository},
            port::{PhoneBindingStore, PhoneProvider, PhoneReservation},
        },
    },
};

#[derive(Clone)]
pub struct Service<
    Store: PhoneBindingStore = Repository,
    Provider: PhoneProvider = crate::integrations::south::adapter::miniapp_http::Client,
> {
    pub store: Store,
    pub provider: Provider,
    pub cipher: Option<PhoneCipher>,
}

impl<Store: PhoneBindingStore, Provider: PhoneProvider> Service<Store, Provider> {
    pub async fn bind(&self, actor: &Actor, input: BindPhoneInput) -> Result<PhoneBinding> {
        if actor.is_admin_browser() {
            return Err(Error::Forbidden);
        }
        input.validate()?;
        let cipher = self
            .cipher
            .as_ref()
            .ok_or(Error::Phone("phone_provider_unavailable"))?;
        match self
            .store
            .reserve_phone(actor, self.provider.app_id(), &input)
            .await?
        {
            PhoneReservation::Completed(result) => Ok(result),
            PhoneReservation::Reserved { openid } => {
                let phone = match self.provider.verify_phone(&input.code, &openid).await {
                    Ok(phone) => phone,
                    Err(error) => {
                        self.store.fail_phone(actor, input.request_id).await?;
                        return Err(error);
                    }
                };
                let ciphertext = cipher.encrypt(actor.id, &phone.number)?;
                self.store
                    .finish_phone(actor, &input, &phone, &ciphertext, &cipher.version)
                    .await
            }
        }
    }
}
