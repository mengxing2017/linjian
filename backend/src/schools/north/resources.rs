use crate::{
    business::{Actor, Error, Result, require},
    schools::south::{
        adapter::postgres::Repository,
        port::resources::{Command, Store},
    },
};
use serde_json::Value;
use uuid::Uuid;
#[derive(Clone)]
pub struct Service<R: Store = Repository>(pub R);
impl<R: Store> Service<R> {
    pub async fn command(&self, a: &Actor, s: Option<Uuid>, v: Command) -> Result<Value> {
        if let Command::Preview(input) = &v {
            input.validate()?;
        }
        self.0.command(a, s, v).await
    }
    pub async fn upload(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        b: Vec<u8>,
        files: crate::media::south::adapter::filesystem::Files,
    ) -> Result<Value> {
        self.upload_scoped(a, s, b, files, "resource.school.write")
            .await
    }
    pub async fn qr(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        url: &str,
        files: crate::media::south::adapter::filesystem::Files,
    ) -> Result<Value> {
        require(
            crate::schools::domain::https(url) && url.len() <= 2048,
            "二维码内容须为 HTTPS 业务链接",
        )?;
        let code = qrcode::QrCode::new(url).map_err(|_| Error::Invalid("二维码内容过长".into()))?;
        let image = code
            .render::<image::Luma<u8>>()
            .min_dimensions(384, 384)
            .build();
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageLuma8(image)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .map_err(|_| Error::Unavailable)?;
        self.upload(a, s, bytes.into_inner(), files).await
    }
    pub async fn list(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        page: crate::business::Page,
    ) -> Result<Vec<Value>> {
        self.list_scoped(a, s, page, "resource.school.read").await
    }
    pub async fn media(
        &self,
        a: &Actor,
        s: Option<Uuid>,
        id: Uuid,
        published: bool,
        files: crate::media::south::adapter::filesystem::Files,
    ) -> Result<(String, Vec<u8>)> {
        self.media_scoped(a, s, id, published, files, "resource.school.read")
            .await
    }

    pub async fn upload_scoped(
        &self,
        actor: &Actor,
        school: Option<Uuid>,
        bytes: Vec<u8>,
        files: crate::media::south::adapter::filesystem::Files,
        context: &str,
    ) -> Result<Value> {
        crate::schools::domain::permissions::artwork_context(context, true)?;
        self.0.upload(actor, school, bytes, context, files).await
    }
    pub async fn list_scoped(
        &self,
        actor: &Actor,
        school: Option<Uuid>,
        page: crate::business::Page,
        context: &str,
    ) -> Result<Vec<Value>> {
        self.0.media_list(actor, school, page, context).await
    }
    pub async fn media_scoped(
        &self,
        actor: &Actor,
        school: Option<Uuid>,
        id: Uuid,
        published: bool,
        files: crate::media::south::adapter::filesystem::Files,
        context: &str,
    ) -> Result<(String, Vec<u8>)> {
        self.0
            .media(actor, school, id, published, context, files)
            .await
    }
}
