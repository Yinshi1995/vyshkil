ALTER TABLE org_contact ADD COLUMN kind text NOT NULL DEFAULT 'personal';
CREATE UNIQUE INDEX idx_org_contact_org_phone ON org_contact (org_id, phone);
