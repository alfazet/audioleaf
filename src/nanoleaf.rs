use expanduser::expanduser;
use palette::{FromColor, Hwb, Srgb};
use std::cmp::Ordering;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::path::Path;
use url::Url;

const NL_API_PORT: u16 = 16021;
const NL_UDP_PORT: u16 = 60222;

#[derive(Debug, Clone, Copy)]
pub struct Panel {
    id: u16,
    pub x: i16,
    pub y: i16,
}

#[derive(Debug)]
pub struct Nanoleaf {
    panels: Vec<Panel>,
    socket: UdpSocket,
}

impl Nanoleaf {
    /// Return a handle to a Nanoleaf device at the given ip address, auth token stored at the given path and
    /// a UDP socket bound to the given port
    pub fn new(ip: &str, port: u16, token_file_path: &str) -> Result<Self, Box<dyn Error>> {
        let ip = ip.parse::<Ipv4Addr>()?;
        let token_file_path_expanded = expanduser(token_file_path)?.display().to_string();
        let token_file_path = Path::new(&token_file_path_expanded);
        let token = Self::get_token(&ip, token_file_path)?;
        let panels = Self::get_panels(&ip, &token)?;
        Self::request_udp_control(&ip, &token)?;
        let socket = Self::enable_udp_socket(&ip, port)?;

        Ok(Nanoleaf { panels, socket })
    }

    fn get_token(ip: &Ipv4Addr, token_file_path: &Path) -> Result<String, Box<dyn Error>> {
        if !Path::try_exists(token_file_path)? {
            Self::generate_new_token(ip, token_file_path)?;
        }

        Self::get_saved_token(token_file_path)
    }

    /// Generate a new auth token for this Nanoleaf device and save it to a file
    fn generate_new_token(ip: &Ipv4Addr, token_file_path: &Path) -> Result<(), Box<dyn Error>> {
        let url = Url::parse(&format!("http://{}:{}/api/v1/new", ip, NL_API_PORT))?;
        let req_client = reqwest::blocking::Client::new();
        let res = req_client
            .post(url)
            .send()?
            .error_for_status()
            .map_err(|err| Box::new(err) as Box<dyn Error>)?;
        let res_text = res.text()?;
        let res_json: serde_json::Value = serde_json::from_str(&res_text)?;
        let token = res_json["auth_token"]
            .as_str()
            .unwrap()
            .trim_end()
            .to_string();

        let mut token_file = File::create(token_file_path)?;
        token_file.write_all(token.as_bytes())?;

        Ok(())
    }

    /// Get the token from a file
    fn get_saved_token(path: &Path) -> Result<String, Box<dyn Error>> {
        let mut token_file = File::open(path)?;
        let mut token = String::new();
        token_file.read_to_string(&mut token)?;

        Ok(token)
    }

    /// Get data about this device's panels
    fn get_panels(ip: &Ipv4Addr, token: &str) -> Result<Vec<Panel>, Box<dyn Error>> {
        let url = Url::parse(&format!(
            "http://{}:16021/api/v1/{}/panelLayout/layout",
            ip, token
        ))?;
        let req_client = reqwest::blocking::Client::new();
        let res = req_client
            .get(url)
            .send()?
            .error_for_status()
            .map_err(|err| Box::new(err) as Box<dyn Error>)?;
        let res_text = res.text()?;
        let res_json: serde_json::Value = serde_json::from_str(&res_text)?;
        let res_panels = res_json["positionData"].as_array().unwrap();
        let mut panels = Vec::new();
        for panel in res_panels.iter() {
            let id = panel["panelId"].as_u64().unwrap() as u16;
            let (x, y) = (
                panel["x"].as_i64().unwrap() as i16,
                panel["y"].as_i64().unwrap() as i16,
            );
            panels.push(Panel { id, x, y });
        }

        Ok(panels)
    }

    fn request_udp_control(ip: &Ipv4Addr, token: &str) -> Result<(), Box<dyn Error>> {
        let url = Url::parse(&format!("http://{}:16021/api/v1/{}/effects", ip, token))?;
        let data_json = &serde_json::json!({"write": {r"command":  "display", "animType": "extControl", "extControlVersion": "v2"}});
        let req_client = reqwest::blocking::Client::new();
        req_client
            .put(url)
            .json(data_json)
            .send()?
            .error_for_status()
            .map_err(|err| Box::new(err) as Box<dyn Error>)?;

        Ok(())
    }

    fn enable_udp_socket(ip: &Ipv4Addr, port: u16) -> Result<UdpSocket, Box<dyn Error>> {
        let socket_addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port);
        let socket = UdpSocket::bind(socket_addr)?;
        let nl_addr = SocketAddrV4::new(*ip, NL_UDP_PORT);
        socket.connect(nl_addr)?;

        Ok(socket)
    }

    /// Sort panels by comp_fn
    pub fn sort_panels<F>(&mut self, comp_fn: F)
    where
        F: FnMut(&Panel, &Panel) -> Ordering,
    {
        self.panels.sort_by(comp_fn);
    }

    /// Run commands by sending bytes through UDP, see Nanoleaf API docs, section 3.2.6.2
    pub fn run_commands(&self, commands: &[Command]) -> Result<(), Box<dyn Error>> {
        let split_into_bytes = |x: u16| -> (u8, u8) {
            // split a u16 into two bytes (in big endian), e.g. 651 -> (2, 139) because 651 = 2 * 256 + 139
            ((x / 256) as u8, (x % 256) as u8)
        };

        let n_panels = commands.len();
        let mut buf = vec![0; 2];
        (buf[0], buf[1]) = split_into_bytes(n_panels as u16);
        for command in commands.iter() {
            let Command {
                panel_no,
                color: color_hwb,
                trans_time,
            } = command;
            if *panel_no > self.panels.len() - 1 {
                return Err(format!("Tried to access panel {}, but there are only {} panels available.", panel_no, self.panels.len()).into());
            }
            let color_rgb = Srgb::from_color(*color_hwb).into_format::<u8>();
            let Srgb {
                red,
                green,
                blue,
                standard: _,
            } = color_rgb;

            let mut sub_buf = [0u8; 8];
            (sub_buf[0], sub_buf[1]) = split_into_bytes(self.panels[*panel_no].id);
            (sub_buf[2], sub_buf[3], sub_buf[4], sub_buf[5]) = (red, green, blue, 0);
            (sub_buf[6], sub_buf[7]) = split_into_bytes(*trans_time);
            buf.extend(sub_buf);
        }
        self.socket.send(&buf)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct Command {
    pub panel_no: usize,
    pub color: Hwb,
    pub trans_time: u16,
}
