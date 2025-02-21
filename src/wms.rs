use reqwest::{self, Client, header};
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Authentication methods for WMS servers (reusing from your WFS implementation)
enum WmsAuth {
    /// Basic HTTP authentication
    Basic { username: String, password: String },
    /// Token-based authentication
    BearerToken(String),
    /// API key in query parameter
    ApiKey { param_name: String, key: String },
    /// Cookie-based authentication
    Cookie(String),
}

/// Client for accessing WMS services
struct WmsClient {
    client: Client,
    base_url: String,
    auth: Option<WmsAuth>,
}

impl WmsClient {
    /// Create a new WMS client
    pub fn new(base_url: &str, auth: Option<WmsAuth>) -> Result<Self, Box<dyn Error>> {
        let mut headers = header::HeaderMap::new();
        // Set common headers
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("rust-wms-client/0.1.0"),
        );

        let client = Client::builder().default_headers(headers).build()?;

        Ok(WmsClient {
            client,
            base_url: base_url.to_string(),
            auth,
        })
    }

    /// Fetch a map tile from WMS server as PNG
    pub async fn fetch_map_tile(
        &self,
        layers: &str,
        bbox: &str,
        width: u32,
        height: u32,
        srs: &str,
        format: &str,
        transparent: bool,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        // Build base WMS request URL
        let mut url = format!(
            "{}?SERVICE=WMS&VERSION=1.3.0&REQUEST=GetMap&LAYERS={}&BBOX={}&WIDTH={}&HEIGHT={}&CRS={}&FORMAT={}&TRANSPARENT={}&styles=default",
            self.base_url, layers, bbox, width, height, srs, format, transparent
        );

        // Apply API key authentication if needed
        let mut final_url = url.clone();
        if let Some(WmsAuth::ApiKey { param_name, key }) = &self.auth {
            final_url = format!("{}&{}={}", url, param_name, key);
        }

        // Build the request with appropriate authentication
        let mut request = self.client.get(&final_url);

        // Apply authentication if configured
        if let Some(auth) = &self.auth {
            match auth {
                WmsAuth::Basic { username, password } => {
                    request = request.basic_auth(username, Some(password));
                }
                WmsAuth::BearerToken(token) => {
                    request = request.bearer_auth(token);
                }
                WmsAuth::Cookie(cookie_str) => {
                    request = request.header(header::COOKIE, cookie_str);
                }
                WmsAuth::ApiKey { .. } => {
                    // Already handled in URL construction
                }
            }
        }

        // Execute request
        let response = request.send().await?;

        // Check for success
        if !response.status().is_success() {
            return Err(format!("WMS request failed with status: {}", response.status()).into());
        }

        // Get binary response
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Save a fetched tile to a file
    pub fn save_tile_to_file(data: &[u8], filepath: &str) -> Result<(), Box<dyn Error>> {
        let path = Path::new(filepath);
        let mut file = File::create(path)?;
        file.write_all(data)?;
        Ok(())
    }
}

// Example usage in an async function:
pub async fn fetch_wms_example() -> Result<(), Box<dyn Error>> {
    // Create a WMS client
    let wms_client = WmsClient::new(
        "https://www.geodatenportal.sachsen-anhalt.de/arcgis/services/LAGB/LAGB_Geophysik_G1_OpenData/MapServer/WMSServer",
        None,
    )?;

    // Fetch a map tile
    let tile_data = wms_client
        .fetch_map_tile(
            "Temperaturverteilung_in_2000_m_Tiefe__°C_9363", // Layer name/id
            "645945.1,5720831.6,747959.0,5796011.4", // BBOX: minx,miny,maxx,maxy
            1024, // Width
            768,  // Height
            "EPSG:25832", // Coordinate reference system
            "image/png", // Format
            true,        // Transparent background
        )
        .await?;

    //dbg!(tile_data.clone());

    // Save the tile
    WmsClient::save_tile_to_file(&tile_data, "map_tile.png")?;
    println!("Tile saved as map_tile.png");

    Ok(())
}

