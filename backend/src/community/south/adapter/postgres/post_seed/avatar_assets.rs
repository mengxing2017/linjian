use crate::{
    business::{Result, require},
    community::domain::post_seed::AVATAR_COUNT,
};

const AVATARS: [&[u8]; AVATAR_COUNT as usize] = [
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-01-default.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-02.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-03.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-04.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-05.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-06.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-07.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-08.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-09.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-10.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-11.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-12.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-13.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-14.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-15.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-16.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-17.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-18.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-19.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-20.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-21.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-22.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-23.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-24.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-25.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-26.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-27.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-28.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-29.png"),
    include_bytes!("../../../../../../assets/simulated-avatars/student-avatar-30.png"),
];

pub fn avatar_png(slot: u8) -> Result<Vec<u8>> {
    require(slot < AVATAR_COUNT, "内置头像无效")?;
    Ok(AVATARS[usize::from(slot)].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::south::adapter::filesystem::digest;

    #[test]
    fn migrated_avatars_are_distinct_compressed_pngs() {
        let mut digests = std::collections::HashSet::new();
        for slot in 0..AVATAR_COUNT {
            let bytes = avatar_png(slot).unwrap();
            assert!(bytes.len() < 64 * 1024);
            assert_eq!(
                image::guess_format(&bytes).unwrap(),
                image::ImageFormat::Png
            );
            let decoded = image::load_from_memory(&bytes).unwrap();
            assert_eq!((decoded.width(), decoded.height()), (128, 128));
            assert!(digests.insert(digest(&bytes)));
        }
        assert_eq!(digests.len(), 30);
        assert!(avatar_png(AVATAR_COUNT).is_err());
        assert!(avatar_png(u8::MAX).is_err());
    }
}
