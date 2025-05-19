# 2FA OTP Service API Documentation

## Endpoints

### 1. OTP Setup

**Endpoint**: `POST /2fa/otp/setup`

Sets up OTP authentication for a user.

**Request Body**:
```json
{
  "user_id": "string (required)",
  "issuer": "string (required) - Domain name (e.g., auexch.co)",
  "site": "string (required) - Base URL (e.g., https://dev-api.sadmincontrol.com)"
}

{
  "secret": "<OTP Key>",
  "qr_code_uri": "<URI to QR Image>",
  "provisioning_uri": "<Site URI>"
}
```

### 1. Verify

**Endpoint**: `POST /2fa/otp/verify`

Verify OTP for a user.

**Request Body**:
```json
{
  "user_id": "string (required)",
  "code": "string (required) - 6-digit OTP code"
}

{
  "verified": true,
  "message": "Verification successful"
}
```

