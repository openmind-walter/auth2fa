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
  "status": true,
  "code": "OTP_VERIFIED"
}
{
  "status": false,
  "code": "EOTP_*"
}

```

(base) uqapp@Gunasekarans-Mac-mini auth2fa % curl -vv http://localhost:8080/2fa/otp/setup -H 'Content-Type':application/json -XPOST -d '{"user_id": "p2rO", "issuer": "abc.com"}'
Note: Unnecessary use of -X or --request, POST is already inferred.
* Host localhost:8080 was resolved.
* IPv6: ::1
* IPv4: 127.0.0.1
*   Trying [::1]:8080...
* connect to ::1 port 8080 from ::1 port 62287 failed: Connection refused
*   Trying 127.0.0.1:8080...
* Connected to localhost (127.0.0.1) port 8080
> POST /2fa/otp/setup HTTP/1.1
> Host: localhost:8080
> User-Agent: curl/8.7.1
> Accept: */*
> Content-Type:application/json
> Content-Length: 40
> 
* upload completely sent off: 40 bytes
< HTTP/1.1 200 OK
< content-type: application/json
< vary: origin, access-control-request-method, access-control-request-headers
< access-control-allow-credentials: true
< content-length: 380
< date: Fri, 16 May 2025 08:39:08 GMT
< 
* Connection #0 to host localhost left intact
{"secret":"WCJIEMKXV2F4TTNC4WG2OMM36QR4CNIU","provisioning_uri":"otpauth://totp/abc.com:p2rO?secret=WCJIEMKXV2F4TTNC4WG2OMM36QR4CNIU&issuer=abc.com&algorithm=SHA1&digits=6&period=30","qr_code_url":"http://localhost:8080/2fa/qr?data=otpauth%3A%2F%2Ftotp%2Fabc.com%3Ap2rO%3Fsecret%3DWCJIEMKXV2F4TTNC4WG2OMM36QR4CNIU%26issuer%3Dabc.com%26algorithm%3DSHA1%26digits%3D6%26period%3D30"}%                                                                                                                                                                                 
(base) uqapp@Gunasekarans-Mac-mini auth2fa % curl -vv http://localhost:8080/2fa/otp/verify -H 'Content-Type':application/json -XPOST -d '{"user_id": "p2rO", "code": "338739"}' 
Note: Unnecessary use of -X or --request, POST is already inferred.
* Host localhost:8080 was resolved.
* IPv6: ::1
* IPv4: 127.0.0.1
*   Trying [::1]:8080...
* connect to ::1 port 8080 from ::1 port 62457 failed: Connection refused
*   Trying 127.0.0.1:8080...
* Connected to localhost (127.0.0.1) port 8080
> POST /2fa/otp/verify HTTP/1.1
> Host: localhost:8080
> User-Agent: curl/8.7.1
> Accept: */*
> Content-Type:application/json
> Content-Length: 37
> 
* upload completely sent off: 37 bytes
< HTTP/1.1 200 OK
< content-type: application/json
< vary: origin, access-control-request-method, access-control-request-headers
< access-control-allow-credentials: true
< content-length: 43
< date: Fri, 16 May 2025 08:39:45 GMT
< 
* Connection #0 to host localhost left intact
{"status":false,"message":"TOTP NOT SETUP"}%                                                                                                                                              
(base) uqapp@Gunasekarans-Mac-mini auth2fa % 