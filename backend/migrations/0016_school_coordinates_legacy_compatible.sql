ALTER TABLE schools ALTER COLUMN latitude DROP NOT NULL;
ALTER TABLE schools ALTER COLUMN longitude DROP NOT NULL;

ALTER TABLE schools ADD CONSTRAINT schools_coordinates_required_on_write
    CHECK (latitude IS NOT NULL AND longitude IS NOT NULL) NOT VALID;
