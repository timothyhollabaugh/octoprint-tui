use futures::Future;
use futures::Stream;

use hyper::client::HttpConnector;
use hyper::Body;
use hyper::Client;
use hyper::Request;
use serde::de::DeserializeOwned;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub enum Origin {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "sdcard")]
    SdCard,
}

#[derive(Deserialize, Debug, Clone)]
pub struct References {
    pub resource: String,
    pub download: Option<String>,
    pub model: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FileAbridged {
    pub name: Option<String>,
    pub display: Option<String>,
    pub path: Option<String>,
    pub origin: Option<Origin>,
    pub references: Option<References>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Filament {
    pub length: Option<f64>,
    pub volume: Option<f64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Job {
    pub file: FileAbridged,
    #[serde(rename = "estimatedPrintTime")]
    pub estimated_print_time: Option<f64>,
    #[serde(rename = "lastPrintTime")]
    pub last_print_time: Option<f64>,
    pub filament: Option<Filament>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Progress {
    pub completion: Option<f64>,
    pub filepos: Option<f64>,
    #[serde(rename = "printTime")]
    pub print_time: Option<f64>,
    #[serde(rename = "printTimeLeft")]
    pub print_time_left: Option<f64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TemperatureData {
    pub actual: f64,
    pub target: f64,
    pub offset: Option<f64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HistoricTemperatureData {
    pub time: u64,
    pub tool0: Option<TemperatureData>,
    pub tool1: Option<TemperatureData>,
    pub tool2: Option<TemperatureData>,
    pub bed: Option<TemperatureData>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TemperatureState {
    pub tool0: Option<TemperatureData>,
    pub tool1: Option<TemperatureData>,
    pub tool2: Option<TemperatureData>,
    pub bed: Option<TemperatureData>,
    pub history: Option<Vec<HistoricTemperatureData>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SdState {
    pub ready: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PrinterFlags {
    pub operational: bool,
    pub paused: bool,
    pub printing: bool,
    pub pausing: bool,
    pub cancelling: bool,
    #[serde(rename = "sdReady")]
    pub sd_ready: bool,
    pub error: bool,
    pub ready: bool,
    #[serde(rename = "closedOrError")]
    pub closed_or_error: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PrinterState {
    pub text: String,
    pub flags: PrinterFlags,
}

#[derive(Deserialize, Debug, Clone)]
pub struct StateResponse {
    pub temperature: Option<TemperatureState>,
    pub sd: Option<SdState>,
    pub state: Option<PrinterState>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct JobResponse {
    pub job: Job,
    pub progress: Progress,
}

#[derive(Debug)]
pub enum OctoprintError {
    Network(hyper::Error),
    Parse(serde_json::Error),
}

impl From<hyper::Error> for OctoprintError {
    fn from(err: hyper::Error) -> OctoprintError {
        OctoprintError::Network(err)
    }
}

impl From<serde_json::Error> for OctoprintError {
    fn from(err: serde_json::Error) -> OctoprintError {
        OctoprintError::Parse(err)
    }
}

#[derive(Clone)]
pub struct OctoprintClient {
    client: Client<HttpConnector, Body>,
    url: String,
    api_key: String,
}

impl OctoprintClient {
    pub fn new(url: String, api_key: String) -> OctoprintClient {
        let client = Client::new();
        OctoprintClient {
            client,
            url,
            api_key,
        }
    }

    fn send_request<R: DeserializeOwned>(
        &self,
        path: String,
    ) -> impl Future<Item = R, Error = OctoprintError> {
        let request = Request::builder()
            .uri(format!("{}/api/{}", self.url.clone(), path))
            .header("X-Api-Key", self.api_key.clone())
            .body(Body::empty())
            .expect(&format!(
                "Error building reqest with url {}, api_key {}, and path {}",
                self.url, self.api_key, path
            ));
        self.client
            .request(request)
            .and_then(|res| res.into_body().concat2())
            .from_err::<OctoprintError>()
            .and_then(|body| {
                let job = serde_json::from_slice(&body)?;
                Ok(job)
            })
            .from_err()
    }

    pub fn load_job(&mut self) -> impl Future<Item = JobResponse, Error = OctoprintError> {
        self.send_request("job".to_string())
    }

    pub fn load_state(&mut self) -> impl Future<Item = StateResponse, Error = OctoprintError> {
        self.send_request("printer".to_string())
    }
}
