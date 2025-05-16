// main.rs
// use axum::{
//     extract::{Path, State},
//     http::StatusCode,
//     routing::{get, post},
//     Json, Router,
// };
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use axum::extract::Query;
use axum::{
    extract::{Path, State},
    http::{
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
        HeaderMap, HeaderName, HeaderValue, Method, StatusCode,
    },
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use bf_common_utils::get_bf_auth::make_request;
use std::io::Cursor;
use tower_http::cors::{ AllowOrigin, CorsLayer };
use image::{Luma, ImageBuffer};

use std::sync::Arc;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Mutex;
use hmac::Mac;
//use qrcode::QrCode;
use qrcodegen::{QrCode, QrCodeEcc};
use image::ImageOutputFormat;
use axum::response::Response;
#[derive(Deserialize)]
struct QrParams {
    data: String,
    #[serde(default = "default_size")]
    size: u32,
}

fn default_size() -> u32 {
    150
}
async fn generate_qr(Query(params): Query<HashMap<String, String>>) -> Response {
    let data = match params.get("data") {
        Some(d) => d,
        None => return (StatusCode::BAD_REQUEST, "Missing `data` param").into_response(),
    };

    // Generate the QR code
    let qr = match QrCode::encode_text(data, QrCodeEcc::Medium) {
        Ok(code) => code,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid data").into_response(),
    };

    // Convert to image
    let size = qr.size();
    let scale = 2; // pixels per module
    let img_size = size * scale;

    let mut img = ImageBuffer::<Luma<u8>, Vec<u8>>::new(img_size as u32, img_size as u32);
    for y in 0..size {
        for x in 0..size {
            let color = if qr.get_module(x, y) { 0 } else { 255 };
            for dy in 0..scale {
                for dx in 0..scale {
                    img.put_pixel(
                        (x * scale + dx) as u32,
                        (y * scale + dy) as u32,
                        Luma([color]),
                    );
                }
            }
        }
    }

    // Encode PNG to bytes
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, ImageOutputFormat::Png).unwrap();

    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "image/png".parse().unwrap());

    (headers, buf.into_inner()).into_response()
}
// pub async fn generate_qr(Query(params): Query<QrParams>) -> impl IntoResponse {
//     let code = QrCode::new(params.data.as_bytes()).unwrap();

//     let scale = params.size / code.width() as u32;
//     let qr_width = (code.width() as u32) * scale;

//     // Create a blank white image
//     let mut img = ImageBuffer::from_pixel(qr_width, qr_width, Luma([255u8]));

//     for y in 0..code.width() {
//         for x in 0..code.width() {
//             if code[(x, y)] == Color::Dark {
//                 for dy in 0..scale {
//                     for dx in 0..scale {
//                         img.put_pixel(
//                             x as u32 * scale + dx,
//                             y as u32 * scale + dy,
//                             Luma([0u8]),
//                         );
//                     }
//                 }
//             }
//         }
//     }

//     let mut png_bytes = Vec::new();
//     let mut cursor = Cursor::new(&mut png_bytes);
//     image::DynamicImage::ImageLuma8(img)
//         .write_to(&mut cursor, image::ImageFormat::Png)
//         .unwrap();

//     let mut headers = HeaderMap::new();
//     headers.insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));

//     (headers, png_bytes)
// }

// For fingerprint auth simulation
struct FingerprintStore {
    fingerprints: Mutex<HashMap<String, Vec<u8>>>,
}

// App state
struct AppState {
    api_server_url: String,
    // pool: Pool,
    fingerprint_store: FingerprintStore,
}

// TOTP Request and Response models
#[derive(Serialize, Deserialize)]
struct TotpSetupRequest {
    user_id: String,
    issuer: String, // e.g., "YourApp"
}

#[derive(Serialize, Deserialize)]
struct TotpSetupResponse {
    secret: String,
    provisioning_uri: String,
    qr_code_url: String,
}

#[derive(Serialize, Deserialize)]
struct TotpVerifyRequest {
    user_id: String,
    code: String,
}

// Fingerprint Auth models
#[derive(Serialize, Deserialize)]
struct FingerprintSetupRequest {
    user_id: String,
    fingerprint_data: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct FingerprintVerifyRequest {
    user_id: String,
    fingerprint_data: Vec<u8>,
}

// Basic response model
#[derive(Serialize, Deserialize)]
struct ApiResponse {
    status: bool,
    message: String
}

// Database schema setup function
// async fn setup_database(client: &Client) -> Result<(), PgError> {
//     client
//         .batch_execute(
//             "
//             CREATE TABLE IF NOT EXISTS users (
//                 id VARCHAR(36) PRIMARY KEY,
//                 username VARCHAR(255) NOT NULL UNIQUE,
//                 created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
//             );
            
//             CREATE TABLE IF NOT EXISTS totp_secrets (
//                 id SERIAL PRIMARY KEY,
//                 user_id VARCHAR(36) NOT NULL REFERENCES users(id),
//                 secret VARCHAR(255) NOT NULL,
//                 issuer VARCHAR(255) NOT NULL,
//                 created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
//             );
            
//             CREATE TABLE IF NOT EXISTS fingerprint_auth (
//                 id SERIAL PRIMARY KEY,
//                 user_id VARCHAR(36) NOT NULL REFERENCES users(id),
//                 fingerprint_hash VARCHAR(255) NOT NULL,
//                 created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
//             );
//             "
//         )
//         .await?;
    
//     Ok(())
// }
// Updated TOTP functions for totp-rs 5.7.0

// Generate a new TOTP secret
fn generate_totp_secret() -> String {
    let mut rng = thread_rng();
    let secret_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    URL_SAFE_NO_PAD.encode(&secret_bytes)
}
// Create URL manually since the library API has changed
fn create_totp_uri(secret_base32: &str, account_name: &str, issuer: &str) -> String {
    // Format according to the KeyURI format: https://github.com/google/google-authenticator/wiki/Key-Uri-Format
    format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm=SHA1&digits=6&period=30",
        urlencoding::encode(issuer),
        urlencoding::encode(account_name),
        secret_base32,
        urlencoding::encode(issuer)
    )
}

// Generate a base32 string for TOTP secret
fn generate_base32_secret() -> String {
    // Generate random bytes
    let mut rng = thread_rng();
    let secret_bytes: Vec<u8> = (0..20).map(|_| rng.gen()).collect();
    
    // Base32 encode the bytes - note we're doing our own implementation since the library
    // doesn't expose what we need
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut result = String::new();
    
    for chunk in secret_bytes.chunks(5) {
        let mut buffer = [0u8; 5];
        for (i, &byte) in chunk.iter().enumerate() {
            buffer[i] = byte;
        }
        
        // Convert 5 bytes to 8 base32 characters
        if chunk.len() >= 1 {
            result.push(alphabet[(buffer[0] >> 3) as usize].into());
        }
        if chunk.len() >= 1 {
            result.push(alphabet[(((buffer[0] & 0x07) << 2) | (buffer[1] >> 6)) as usize].into());
        }
        if chunk.len() >= 2 {
            result.push(alphabet[((buffer[1] & 0x3F) >> 1) as usize].into());
        }
        if chunk.len() >= 2 {
            result.push(alphabet[(((buffer[1] & 0x01) << 4) | (buffer[2] >> 4)) as usize].into());
        }
        if chunk.len() >= 3 {
            result.push(alphabet[(((buffer[2] & 0x0F) << 1) | (buffer[3] >> 7)) as usize].into());
        }
        if chunk.len() >= 4 {
            result.push(alphabet[((buffer[3] & 0x7F) >> 2) as usize].into());
        }
        if chunk.len() >= 4 {
            result.push(alphabet[(((buffer[3] & 0x03) << 3) | (buffer[4] >> 5)) as usize].into());
        }
        if chunk.len() >= 5 {
            result.push(alphabet[(buffer[4] & 0x1F) as usize].into());
        }
    }
    
    result
}

// Convert base32 string back to bytes
fn base32_decode(input: &str) -> Result<Vec<u8>, String> {
    let input = input.replace(" ", "").to_uppercase();
    let mut result = Vec::new();
    let mut buffer = 0u64;
    let mut bits_left = 0;
    
    for c in input.chars() {
        let value = match c {
            'A'..='Z' => (c as u8 - b'A') as u64,
            '2'..='7' => (c as u8 - b'2' + 26) as u64,
            _ => return Err(format!("Invalid base32 character: {}", c)),
        };
        
        buffer = (buffer << 5) | value;
        bits_left += 5;
        
        if bits_left >= 8 {
            bits_left -= 8;
            result.push((buffer >> bits_left) as u8);
            buffer &= (1 << bits_left) - 1;
        }
    }
    
    Ok(result)
}

// Calculate TOTP code
fn calculate_totp(secret: &[u8], time: u64) -> Result<String, &'static str> {
    // Calculate the counter value (RFC 6238)
    let counter = time / 30; // 30-second interval
    
    // Convert counter to bytes (big-endian)
    let counter_bytes = counter.to_be_bytes();
    
    // Calculate HMAC-SHA1
    let mut mac = hmac::Hmac::<sha1::Sha1>::new_from_slice(secret)
        .map_err(|_| "Invalid key length")?;
    mac.update(&counter_bytes);
    let result = mac.finalize().into_bytes();
    
    // Dynamic truncation
    let offset = (result[19] & 0x0f) as usize;
    let bin_code = ((result[offset] & 0x7f) as u32) << 24
        | (result[offset + 1] as u32) << 16
        | (result[offset + 2] as u32) << 8
        | (result[offset + 3] as u32);
    
    // Modulo to get the proper number of digits
    let code = bin_code % 1_000_000; // 6 digits
    
    // Format as a 6-digit string with leading zeros
    Ok(format!("{:06}", code))
}

// Setup TOTP (Google/Microsoft Authenticator)
async fn setup_totp(
    State(state): State<Arc<AppState>>,
    Json(request): Json<TotpSetupRequest>,
) -> Result<Json<TotpSetupResponse>, (StatusCode, Json<ApiResponse>)> {
    // let client = state.pool.get().await.map_err(|e| {
    //     (
    //         StatusCode::INTERNAL_SERVER_ERROR,
    //         Json(ApiResponse {
    //             success: false,
    //             message: format!("Database error: {}", e),
    //         }),
    //     )
    // })?;

    // Generate new TOTP secret
    println!("Secret");
    let secret_base32 = generate_base32_secret();

    println!("Secret {}", secret_base32);
    // Generate provisioning URI for QR code
    let provisioning_uri = create_totp_uri(&secret_base32, &request.user_id, &request.issuer);

    let data = serde_json::json!({
        "USER_ID": &request.user_id,
        "SECRET": &secret_base32,
        "ISSUER": &request.issuer,
    });
    let end_point = format!("{}/v1/auth2fa/otp_secrets/setup", state.api_server_url);
    api_server(&end_point, data).await;

    // Store the secret in the database
    // client
    //     .execute(
    //         "INSERT INTO totp_secrets (user_id, secret, issuer) VALUES ($1, $2, $3)",
    //         &[&request.user_id, &secret_base32, &request.issuer],
    //     )
    //     .await
    //     .map_err(|e| {
    //         (
    //             StatusCode::INTERNAL_SERVER_ERROR,
    //             Json(ApiResponse {
    //                 success: false,
    //                 message: format!("Database error: {}", e),
    //             }),
    //         )
    //     })?;

        // "https://api.qrserver.com/v1/create-qr-code/?data={}&size=200x200",
    // QR code URL (in a real app, you would generate a QR code)
    let qr_code_url = format!(
    "http://localhost:8080/2fa/qr?data={}", //&size=200x200",
    urlencoding::encode(&provisioning_uri)
    );

    Ok(Json(TotpSetupResponse {
        secret: secret_base32,
        provisioning_uri,
        qr_code_url,
    }))
}

// Verify TOTP code
async fn verify_totp(
    State(state): State<Arc<AppState>>,
    Json(request): Json<TotpVerifyRequest>,
) -> Result<Json<ApiResponse>, (StatusCode, Json<ApiResponse>)> {

    // let client = state.pool.get().await.map_err(|e| {
    //     (
    //         StatusCode::INTERNAL_SERVER_ERROR,
    //         Json(ApiResponse {
    //             success: false,
    //             message: format!("Database error: {}", e),
    //         }),
    //     )
    // })?;

    // Get the stored secret for this user
    // let row = client
    //     .query_one(
    //         "SELECT secret, issuer FROM totp_secrets WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1",
    //         &[&request.user_id],
    //     )
    //     .await
    //     .map_err(|e| {
    //         (
    //             StatusCode::NOT_FOUND,
    //             Json(ApiResponse {
    //                 success: false,
    //                 message: format!("User not found or TOTP not set up: {}", e),
    //             }),
    //         )
    //     })?;


    let data = serde_json::json!({
        "USER_ID": &request.user_id
    });
    let end_point = format!("{}/v1/auth2fa/totp_secrets/verify", state.api_server_url);
    let res = api_server(&end_point, data).await;
    println!("Res {:#?}", res);
    let mut secret_base32: Option<&str> = None;
    if let Some(result_array) = res["result"].as_array() {
        if result_array.len() == 1 {
            secret_base32 = result_array[0]["SECRET"].as_str();
        }
    }
    match secret_base32 {
        Some(secret) => {
        },
        None => {
            return Ok(Json(ApiResponse {
                status: false,
                message: "TOTP NOT SETUP".to_string(),
            }))
        }
    };

    // Decode base32 secret
    let secret_bytes = base32_decode(secret_base32.unwrap()).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse {
                status: false,
                message: format!("Invalid secret format: {}", e)
            }),
        )
    })?;

    // Get current time
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    status: false,
                    message: format!("System time error")
                }),
            )
        })?
        .as_secs();

    // Check with current time and allow a window of +/- 1 step (30 seconds)
    let times_to_check = [-30, 0, 30];
    let mut is_valid = false;
    
    for time_offset in times_to_check {
        let check_time = current_time as i64 + time_offset as i64;
        if check_time < 0 {
            continue;
        }
        
        let expected_code = calculate_totp(&secret_bytes, check_time as u64).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse {
                    status: false,
                    message: format!("TOTP calculation error: {}", e),
                }),
            )
        })?;
        
        if request.code == expected_code {
            is_valid = true;
            break;
        }
    }

    if is_valid {
        Ok(Json(ApiResponse {
            status: true,
            message: "TOTP code verified successfully".to_string(),
        }))
    } else {
        Ok(Json(ApiResponse {
            status: true,
            message: "Invalid TOTP code".to_string(),
        }))
    }
}

fn get_headers() -> HashMap<String, String> {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    return headers;
}

async fn api_server(endpoint: &str, data: Value) -> Value {
    let method = Method::POST;
    match make_request::<Value>(endpoint, method, Some(data.clone()), Some(get_headers())).await {
        Ok(response) => {
            if let Some(json) = response.json {
                return json;
            } else {
                println!("{} {}", endpoint, data);
                eprintln!("Status: {}", response.status);
                return json!({});

                // println!("No JSON returned.");
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            return json!({});
        }
    }
}

// // Setup fingerprint authentication
// async fn setup_fingerprint(
//     State(state): State<Arc<AppState>>,
//     Json(request): Json<FingerprintSetupRequest>,
// ) -> Result<Json<ApiResponse>, (StatusCode, Json<ApiResponse>)> {
//     // In a real-world scenario, you would:
//     // 1. Process the fingerprint data and generate a secure hash/template
//     // 2. Store it securely in your database
//     // 3. Never store raw fingerprint data
    
//     // Simulating fingerprint processing with a hash
//     let fingerprint_hash = format!("{:x}", md5::compute(&request.fingerprint_data));
    
//     let client = state.pool.get().await.map_err(|e| {
//         (
//             StatusCode::INTERNAL_SERVER_ERROR,
//             Json(ApiResponse {
//                 success: false,
//                 message: format!("Database error: {}", e),
//             }),
//         )
//     })?;

//     // Store the fingerprint hash in the database
//     client
//         .execute(
//             "INSERT INTO fingerprint_auth (user_id, fingerprint_hash) VALUES ($1, $2)",
//             &[&request.user_id, &fingerprint_hash],
//         )
//         .await
//         .map_err(|e| {
//             (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 Json(ApiResponse {
//                     success: false,
//                     message: format!("Database error: {}", e),
//                 }),
//             )
//         })?;

//     // Store in memory for our example
//     let mut fingerprints = state.fingerprint_store.fingerprints.lock().unwrap();
//     fingerprints.insert(request.user_id.clone(), request.fingerprint_data);

//     Ok(Json(ApiResponse {
//         success: true,
//         message: "Fingerprint registered successfully".to_string(),
//     }))
// }

// // Verify fingerprint
// async fn verify_fingerprint(
//     State(state): State<Arc<AppState>>,
//     Json(request): Json<FingerprintVerifyRequest>,
// ) -> Result<Json<ApiResponse>, (StatusCode, Json<ApiResponse>)> {
//     // In a real application, you would:
//     // 1. Compare the submitted fingerprint against stored template
//     // 2. Use a proper biometric matching algorithm
//     // 3. Consider security implications
    
//     // Simple simulation for demo purposes
//     let fingerprints = state.fingerprint_store.fingerprints.lock().unwrap();
//     let stored_fingerprint = fingerprints.get(&request.user_id);
    
//     match stored_fingerprint {
//         Some(stored_data) if stored_data == &request.fingerprint_data => {
//             Ok(Json(ApiResponse {
//                 success: true,
//                 message: "Fingerprint verified successfully".to_string(),
//             }))
//         }
//         Some(_) => {
//             Ok(Json(ApiResponse {
//                 success: false,
//                 message: "Fingerprint does not match".to_string(),
//             }))
//         }
//         None => {
//             Err((
//                 StatusCode::NOT_FOUND,
//                 Json(ApiResponse {
//                     success: false,
//                     message: "User not found or fingerprint not registered".to_string(),
//                 })
//             ))
//         }
//     }
// }

// Create a user
// async fn create_user(
//     State(state): State<Arc<AppState>>,
//     Json(payload): Json<serde_json::Value>,
// ) -> Result<Json<ApiResponse>, (StatusCode, Json<ApiResponse>)> {
//     let username = payload.get("username").and_then(|u| u.as_str()).ok_or_else(|| {
//         (
//             StatusCode::BAD_REQUEST,
//             Json(ApiResponse {
//                 success: false,
//                 message: "Username is required".to_string(),
//             }),
//         )
//     })?;

//     let user_id = Uuid::new_v4().to_string();
    
//     let client = state.pool.get().await.map_err(|e| {
//         (
//             StatusCode::INTERNAL_SERVER_ERROR,
//             Json(ApiResponse {
//                 success: false,
//                 message: format!("Database error: {}", e),
//             }),
//         )
//     })?;

//     // Create the user
//     match client
//         .execute(
//             "INSERT INTO users (id, username) VALUES ($1, $2)",
//             &[&user_id, &username],
//         )
//         .await {
//             Ok(_) => {
//                 Ok(Json(ApiResponse {
//                     success: true,
//                     message: format!("User created with ID: {}", user_id),
//                 }))
//             }
//             Err(e) => {
//                 if e.to_string().contains("duplicate key") {
//                     Err((
//                         StatusCode::CONFLICT,
//                         Json(ApiResponse {
//                             success: false,
//                             message: "Username already exists".to_string(),
//                         })
//                     ))
//                 } else {
//                     Err((
//                         StatusCode::INTERNAL_SERVER_ERROR,
//                         Json(ApiResponse {
//                             success: false,
//                             message: format!("Database error: {}", e),
//                         })
//                     ))
//                 }
//             }
//         }
// }

// Health check endpoint
async fn health_check() -> &'static str {
    "Authentication service is running!"
}

// // Updated router setup
// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     // Load environment variables from .env file if it exists
//     dotenv::dotenv().ok();

//     // Configure PostgreSQL connection
//     let mut cfg = Config::new();
//     cfg.host = env::var("PG_HOST").ok();
//     cfg.port = env::var("PG_PORT").ok().and_then(|p| p.parse().ok());
//     cfg.user = env::var("PG_USER").ok();
//     cfg.password = env::var("PG_PASSWORD").ok();
//     cfg.dbname = env::var("PG_DBNAME").ok();

//     // Create the connection pool
//     let pool = cfg.create_pool(None, NoTls)?;
    
//     // Test the connection and set up the database
//     let client = pool.get().await?;
//     setup_database(&client).await?;
//     println!("Database connected and initialized");

//     // Initialize fingerprint store
//     let fingerprint_store = FingerprintStore {
//         fingerprints: Mutex::new(HashMap::new()),
//     };

//     // Create shared application state
//     let state = Arc::new(AppState {
//         pool,
//         fingerprint_store,
//     });

//     let allowed_origins = "http://localhost:4200"
//     .split(',')
//     .map(|origin| origin.trim().parse::<HeaderValue>().unwrap())
//     .collect::<Vec<_>>();

//     // let allowed_origins = ["http://localhost:4200"];

//     let cors = CorsLayer::new()
//     .allow_origin(AllowOrigin::list(allowed_origins))
//     .allow_credentials(true) // Allow credentials
//     .allow_methods([Method::GET, Method::POST])
//     .allow_headers([
//         AUTHORIZATION,
//         ACCEPT,
//         CONTENT_TYPE,
//         HeaderName::from_static("refresh-token"),
//     ]);

//     // Fixed router setup with correct handler types
//     let app = Router::new()
//         .route("/health", get(health_check))
//         .route("/api/users", post(create_user))
//         // Fix the routes with explicit handler types
//         .route(
//             "/2fa/auth/totp/setup", 
//             post(|state, json| setup_totp(state, json))
//         )
//         .route(
//             "/2fa/auth/totp/verify", 
//             post(|state, json| verify_totp(state, json))
//         )
//         .route(
//             "/2fa/auth/fingerprint/setup",
//             post(|state, json| setup_fingerprint(state, json))
//         )
//         .route(
//             "/2fa/auth/fingerprint/verify",
//             post(|state, json| verify_fingerprint(state, json))
//         )
//         .layer(cors)
//         .with_state(state);

//         let ip_address = "0.0.0.0";
//         let port = 3000;
    
//         let addr: SocketAddr = format!("{}:{}", ip_address, port)
//             .parse()
//             .expect("Invalid address format");
    
//         let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
//         // tracing::debug!("listening on {:?}", listener);
//         println!("🚀 Server {}", addr);
//         axum::serve(listener, app).await.unwrap()

//     // Ok(())
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    // let mut cfg = Config::new();
    // cfg.host = env::var("PG_HOST").ok();
    // cfg.port = env::var("PG_PORT").ok().and_then(|p| p.parse().ok());
    // cfg.user = env::var("PG_USER").ok();
    // cfg.password = env::var("PG_PASSWORD").ok();
    // cfg.dbname = env::var("PG_DBNAME").ok();
    let api_server_url = env::var("API_SERVER_URL").expect("API_SERVER_URL not set");

    // let pool = cfg.create_pool(None, NoTls)?;
    // let client = pool.get().await?;
    // setup_database(&client).await?;
    // println!("Database connected and initialized");

    let fingerprint_store = FingerprintStore {
        fingerprints: Mutex::new(HashMap::new()),
    };

    let state = Arc::new(AppState {
        api_server_url,
        // pool,
        fingerprint_store,
    });

    let allowed_origins = env::var("ALLOWED_ORIGINS").expect("ALLOWED_ORIGINS not set")
        .split(',')
        .map(|origin| origin.trim().parse::<HeaderValue>().unwrap())
        .collect::<Vec<_>>();

    // let cors = CorsLayer::new()
    //     .allow_origin(AllowOrigin::list(allowed_origins))
    //     .allow_credentials(true)
    //     .allow_methods([Method::GET, Method::POST])
    //     .allow_headers([
    //         AUTHORIZATION,
    //         ACCEPT,
    //         CONTENT_TYPE,
    //         HeaderName::from_static("refresh-token"),
    //     ]);

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_credentials(true) // Allow credentials
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([
            AUTHORIZATION,
            ACCEPT,
            CONTENT_TYPE,
            HeaderName::from_static("refresh-token"),
        ]);


    let app = Router::new()
        .route("/health", get(health_check))
        // .route("/api/users", post(create_user))
        .route("/2fa/qr", get(generate_qr))
        .route("/2fa/otp/setup", post(|state, json| setup_totp(state, json)))
        .route("/2fa/otp/verify", get(|state, json| verify_totp(state, json)))
        // .route("/2fa/fingerprint/setup", post(|state, json| setup_fingerprint(state, json)))
        // .route("/2fa/fingerprint/verify", post(|state, json| verify_fingerprint(state, json)))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("🚀 Server listening on http://{}", addr);

    axum::serve(tokio::net::TcpListener::bind(&addr).await?, app).await?;

    Ok(())
}
