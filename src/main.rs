use geo_types::Geometry;
use geozero::ToGeo;
use geozero::geojson::{GeoJson, GeoJsonReader, GeoJsonString};
use reqwest::{self, Client, header};
use std::error::Error;

mod wms;

/// Authentication methods for WFS servers
enum WfsAuth {
    /// Basic HTTP authentication
    Basic { username: String, password: String },
    /// Token-based authentication
    BearerToken(String),
    /// API key in query parameter
    ApiKey { param_name: String, key: String },
    /// Cookie-based authentication
    Cookie(String),
}

/// Client for accessing WFS services
struct WfsClient {
    client: Client,
    base_url: String,
    auth: Option<WfsAuth>,
}

impl WfsClient {
    /// Create a new WFS client
    pub fn new(base_url: &str, auth: Option<WfsAuth>) -> Result<Self, Box<dyn Error>> {
        let mut headers = header::HeaderMap::new();
        // Set common headers
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("rust-wfs-client/0.1.0"),
        );

        let client = Client::builder().default_headers(headers).build()?;

        Ok(WfsClient {
            client,
            base_url: base_url.to_string(),
            auth,
        })
    }

    /// Fetch features from the WFS server
    pub async fn fetch_features(
        &self,
        layer_name: &str,
        bbox: Option<&str>,
        max_features: Option<u32>,
    ) -> Result<Vec<Geometry>, Box<dyn Error>> {
        // Build base WFS request URL
        let mut url = format!(
            "{}?service=WFS&version=2.0.0&request=GetFeature&typeName={}&outputFormat=GEOJSON&srsname=EPSG:25832",
            self.base_url, layer_name
        );

        // Add optional parameters
        if let Some(b) = bbox {
            url.push_str(&format!("&bbox={}", b));
        }

        if let Some(max) = max_features {
            url.push_str(&format!("&count={}", max));
        }

        // Apply API key authentication if needed
        let mut final_url = url.clone();
        if let Some(WfsAuth::ApiKey { param_name, key }) = &self.auth {
            final_url = format!("{}&{}={}", url, param_name, key);
        }

        // Build the request with appropriate authentication
        let mut request = self.client.get(&final_url);

        // Apply authentication if configured
        if let Some(auth) = &self.auth {
            match auth {
                WfsAuth::Basic { username, password } => {
                    request = request.basic_auth(username, Some(password));
                }
                WfsAuth::BearerToken(token) => {
                    request = request.bearer_auth(token);
                }
                WfsAuth::Cookie(cookie_str) => {
                    request = request.header(header::COOKIE, cookie_str);
                }
                WfsAuth::ApiKey { .. } => {
                    // Already handled in URL construction
                }
            }
        }

        // Execute request
        let response = request.send().await?;

        // Check for success
        if !response.status().is_success() {
            return Err(format!("WFS request failed with status: {}", response.status()).into());
        }

        let body = response.text().await?;

        //dbg!(body.clone());

        // Parse GeoJSON response
        let geojson = GeoJsonString(body);

        // Convert to geo-types geometries
        let geometries = geojson.to_geo()?;

        Ok(vec![geometries])
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    /*// Example with basic auth
    let basic_auth_client = WfsClient::new(
        "https://secure-example.com/geoserver/wfs",
        Some(WfsAuth::Basic {
            username: "username".to_string(),
            password: "password".to_string(),
        }),
    )?;*/

    let client = WfsClient::new("https://www.geodatenportal.sachsen-anhalt.de/arcgis/services/LAGB/LAGB_Geophysik_G1_OpenData/MapServer/WFSServer", None)?;

    //pagingEnabled='true' preferCoordinatesForWfsT11='false' restrictToRequestBBOX='1' srsname='EPSG:25832' typename='LAGB_Geophysik_G1_OpenData:Isanomale_der_Bouguer-Schwerestörung__mGal_' url='https://www.geodatenportal.sachsen-anhalt.de/arcgis/services/LAGB/LAGB_Geophysik_G1_OpenData/MapServer/WFSServer' version='auto'

    // Fetch features using one of the clients
    let geometries = client
        .fetch_features("LAGB_Geophysik_G1_OpenData:Isanomale_der_Bouguer-Schwerestörung__mGal_", Some("645945.1,5796011.4,747959.0,5720831.6"), Some(10)) // Some("10.0,48.0,11.0,49.0")
        .await?;

    println!("Features: {:?}", geometries);

    // Process geometries...

    wms::fetch_wms_example().await?;

    Ok(())
}
