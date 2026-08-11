-- Store raw API key secrets for password-gated reveal (agent wizard connect info).
ALTER TABLE api_keys ADD COLUMN secret TEXT;
