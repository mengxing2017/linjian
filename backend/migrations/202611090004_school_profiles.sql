ALTER TABLE schools
 ADD COLUMN short_name text NOT NULL DEFAULT '' CHECK(length(short_name)<=50),
 ADD COLUMN province text NOT NULL DEFAULT '' CHECK(length(province)<=100),
 ADD COLUMN city text NOT NULL DEFAULT '' CHECK(length(city)<=100),
 ADD COLUMN area text NOT NULL DEFAULT '' CHECK(length(area)<=100),
 ADD COLUMN latitude numeric(9,6) CHECK(latitude BETWEEN -90 AND 90),
 ADD COLUMN longitude numeric(9,6) CHECK(longitude BETWEEN -180 AND 180),
 ADD COLUMN logo_asset_id uuid,
 ADD COLUMN header_bg_asset_id uuid,
 ADD COLUMN theme_primary text NOT NULL DEFAULT '#6A5AF9' CHECK(theme_primary ~ '^#[0-9A-F]{6}$'),
 ADD COLUMN theme_secondary text NOT NULL DEFAULT '#8F6BFF' CHECK(theme_secondary ~ '^#[0-9A-F]{6}$'),
 ADD COLUMN theme_text_color text NOT NULL DEFAULT '#FFFFFF' CHECK(theme_text_color ~ '^#[0-9A-F]{6}$'),
 ADD COLUMN updated_at timestamptz NOT NULL DEFAULT now(),
 ADD CONSTRAINT school_coordinates_pair CHECK((latitude IS NULL)=(longitude IS NULL)),
 ADD CONSTRAINT school_name_length CHECK(length(btrim(name)) BETWEEN 1 AND 100);
ALTER TABLE operational_media ADD CONSTRAINT operational_media_school_id UNIQUE(school_id,id);
ALTER TABLE schools
 ADD FOREIGN KEY(id,logo_asset_id) REFERENCES operational_media(school_id,id),
 ADD FOREIGN KEY(id,header_bg_asset_id) REFERENCES operational_media(school_id,id);
UPDATE schools SET config=config-'theme_color';
CREATE INDEX schools_status_region ON schools(active,province,city,area,name,id);
