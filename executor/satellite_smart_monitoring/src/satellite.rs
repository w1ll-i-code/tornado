use crate::config::SatelliteSmartMonitoringConfig;
use crate::error::{IcingaSatelliteError, SatelliteConfigurationError};
use crate::icinga2::model::config::{ConfigProvider, Host, Service};
use crate::icinga2::model::{
    Capabilities, CheckResultParams, IcingaHelloParams, RequestCertificate, UpdateCertificate,
    UpdateCertificateResult, UpdateObjectParams,
};
use crate::icinga2::model::{IcingaMethods, IcingaTimestamp, JsonRpc, LogPosition};
use crate::icinga2::object_store::IcingaObjectStore;
use certificates::{load_ca, load_cert, load_key, save_ca, save_cert, save_fpr};
use flume::{Receiver, Sender, TryRecvError};
use log::{debug, error, info, trace, warn};
use serde::Deserialize;
use std::convert::Infallible;
use std::fmt::{Debug, Formatter};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::fs::{read, read_to_string, write};
use tokio::io;
use tokio::io::{split, AsyncRead, AsyncWrite, AsyncWriteExt, ErrorKind, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::oneshot;
use tokio::time::sleep;
use tokio_netstring_trait::{AsyncNetstringRead, AsyncNetstringWrite};
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::{Certificate, ClientConfig, PrivateKey, RootCertStore, ServerName};

pub type IcingaPendingCall = (IcingaTimestamp, SetStateRequest);
pub type SenderInput = Sender<SetStateRequest>;

const ICINGA_VERSION: u32 = 21300;

#[derive(PartialEq, Deserialize)]
pub struct SetStateRequest {
    host: Host,
    service: Option<Service>,
    check_result: CheckResultParams,
}

impl Debug for SetStateRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.service {
            None => f.write_fmt(format_args!("SetStateRequest {{ {} }}", self.host,)),
            Some(service) => {
                f.write_fmt(format_args!("SetStateRequest {{ {}!{} }}", self.host, service.name))
            }
        }
    }
}

pub(crate) async fn start_connection(
    config: &SatelliteSmartMonitoringConfig,
) -> Result<TlsStream<TcpStream>, IcingaSatelliteError> {
    // setup tls config
    let tls_config = create_tls_config(config).await?;

    let address = match &config.endpoint.address {
        ServerName::DnsName(name) => name.as_ref().to_string(),
        ServerName::IpAddress(addr) => addr.to_string(),
        _ => {
            panic!("Non-Exhaustive match on the servername")
        }
    };

    // connect to icinga2
    let stream = TcpStream::connect(&format!("{}:{}", address, config.endpoint.port)).await?;

    // circumvent error where connection will return os error 104
    tokio::time::sleep(Duration::from_secs_f64(0.01)).await;

    // Setup tls connection
    let mut stream = tokio_rustls::TlsConnector::from(Arc::new(tls_config))
        .connect(config.endpoint.address.clone(), stream)
        .await?;

    // send icinga::Hello
    send_icinga_hello(&mut stream).await?;

    // receive hello from the other side
    receive_icinga_hello(&mut stream).await?;

    // try refreshing certificates
    send_request_cert(config, &mut stream).await?;

    // return connection
    Ok(stream)
}

async fn create_tls_config(
    config: &SatelliteSmartMonitoringConfig,
) -> Result<ClientConfig, SatelliteConfigurationError> {
    let config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(load_ca(config).await?)
        .with_client_auth_cert(load_cert(config).await?, load_key(config).await?)?;

    Ok(config)
}

async fn send_icinga_hello<Stream: AsyncWrite + Unpin + Send>(
    stream: &mut Stream,
) -> Result<(), IcingaSatelliteError> {
    let icinga_hello = JsonRpc::from(IcingaMethods::Hello(IcingaHelloParams {
        version: ICINGA_VERSION,
        capabilities: Capabilities::from(0),
    }));

    let data = serde_json::to_vec(&icinga_hello).expect("icinga hello should always serialize");
    stream.write_netstring(&data).await?;

    Ok(())
}

async fn receive_icinga_hello<Stream: AsyncRead + Unpin + Send>(
    stream: &mut Stream,
) -> Result<(), IcingaSatelliteError> {
    // a icinga::Hello message is normally 86 bytes long.
    let mut buffer = [0; 128];
    let len = stream.read_netstring(&mut buffer).await?;
    let hello: JsonRpc = serde_json::from_slice(&buffer[..len])?;

    match hello.method {
        IcingaMethods::Hello(params) => {
            debug!("Received icinga::Hello from peer. {}", params);
            Ok(())
        }
        _ => Err(IcingaSatelliteError::ProtocolError(
            "Expected icinga::Hello as first message.".to_string(),
        )),
    }
}

async fn send_request_cert<Stream: AsyncWrite + Unpin + Send>(
    config: &SatelliteSmartMonitoringConfig,
    stream: &mut Stream,
) -> Result<(), IcingaSatelliteError> {
    let file_path = config.cert_path();
    let req = read_to_string(file_path).await?;

    let icinga_hello = JsonRpc::from(IcingaMethods::RequestCertificate(RequestCertificate {
        ticket: config.endpoint.ticket.clone(),
        cert_request: Some(req),
    }));

    let data = serde_json::to_vec(&icinga_hello).expect("icinga hello should always serialize");
    stream.write_netstring(&data).await?;

    Ok(())
}

pub(crate) async fn handle_update_certificate(
    update_certificate: &UpdateCertificate,
    config: &SatelliteSmartMonitoringConfig,
) -> Result<(), IcingaSatelliteError> {
    match &update_certificate.result {
        UpdateCertificateResult::Error { error } => error!("{}", error),
        UpdateCertificateResult::Ok { ca, cert, fingerprint_request } => {
            save_ca(config, ca.as_bytes()).await?;
            save_cert(config, cert.as_bytes()).await?;
            save_fpr(config, fingerprint_request.as_bytes()).await?;
        }
    }

    Ok(())
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum UpdateObject {
    Update { name: String },
    Delete { name: String },
}

pub(crate) fn worker_io<T: AsyncRead + AsyncWrite + Unpin + Send>(
    connection: T,
    config: &SatelliteSmartMonitoringConfig,
) -> (SenderInput, SenderIo<T>, ReceiverIo<T>) {
    // SECURITY: Normally splitting a tls stream would not be safe, as it might come to write
    // requests during reads. However the current implementation of rustls should be save.
    // https://github.com/tokio-rs/tls/issues/40
    let (icinga_connection_reader, icinga_connection_writer) = split(connection);

    // The size of this channel has to be exactly zero to avoid dropping messages when the connection breaks.
    let (sender_input, sender_work_queue) = flume::bounded(0);
    let (pending_calls_queue, receiver_work_queue) = flume::bounded(config.max_parallel_send);
    let (object_update_sender, object_update_receiver) = object_update_channels();
    let (sender_abort_oneshot, abort_oneshot) = oneshot::channel();

    (
        sender_input,
        SenderIo {
            work_queue: sender_work_queue,
            pending_calls_queue,
            abort_oneshot,
            icinga_connection_writer,
            object_update_receiver,
        },
        ReceiverIo {
            work_queue: receiver_work_queue,
            sender_abort_oneshot,
            icinga_connection_reader,
            object_update_sender,
        },
    )
}

#[cfg(test)]
fn object_update_channels() -> (Sender<UpdateObject>, Receiver<UpdateObject>) {
    flume::bounded(0)
}

#[cfg(not(test))]
fn object_update_channels() -> (Sender<UpdateObject>, Receiver<UpdateObject>) {
    flume::bounded(10)
}

pub(crate) struct ReceiverIo<T: AsyncRead + Send> {
    work_queue: Receiver<IcingaPendingCall>,
    sender_abort_oneshot: oneshot::Sender<()>,
    icinga_connection_reader: ReadHalf<T>,
    object_update_sender: Sender<UpdateObject>,
}

impl<T: AsyncRead + Send> ReceiverIo<T> {
    pub async fn abort(self, stored: Option<IcingaPendingCall>) -> Vec<SetStateRequest> {
        let ReceiverIo { work_queue, sender_abort_oneshot, .. } = self;

        match sender_abort_oneshot.send(()) {
            Ok(_) => {}
            Err(_) => {
                warn!("Sender closed before the receiver")
            }
        };

        // collect all unprocessed messages in the queue.
        let mut remaining = vec![];

        if let Some(stored) = stored {
            remaining.push(stored.1);
        }

        while let Ok((_, method)) = work_queue.recv_async().await {
            remaining.push(method);
        }

        remaining
    }

    pub async fn read_message(
        &mut self,
        buffer: &mut [u8],
    ) -> Result<JsonRpc, IcingaSatelliteError> {
        let message = match self.icinga_connection_reader.read_netstring(buffer).await {
            Ok(size) => &buffer[..size],
            Err(err) => {
                if err.kind() == ErrorKind::BrokenPipe {
                    warn!("Received message to large for buffer. Dropping it.");
                }
                return Err(err.into());
            }
        };

        // parse message
        match serde_json::from_slice::<JsonRpc>(message) {
            Ok(json_rpc) => Ok(json_rpc),
            Err(err) => {
                match std::str::from_utf8(message) {
                    Ok(faulty_json) => {
                        error!(
                            "Received unknown message from icinga that I could not read: {}",
                            faulty_json
                        );
                    }
                    Err(_) => error!("Received Json that isn't valid utf8. {:?}", message),
                }
                Err(IcingaSatelliteError::JsonRpcError(err))
            }
        }
    }

    pub fn pop_from_work_queue(
        &mut self,
        stored: &mut Option<IcingaPendingCall>,
        log_position: LogPosition,
    ) -> Result<(), ()> {
        if let Some((ts, method)) = stored.take() {
            if ts >= log_position.ts() {
                *stored = Some((ts, method));
                return Ok(());
            }
        }

        // drop from queue if needed
        loop {
            match self.work_queue.try_recv() {
                Ok((ts, method)) => {
                    if ts >= log_position.ts() {
                        *stored = Some((ts, method));
                        return Ok(());
                    }
                }
                Err(TryRecvError::Empty) => return Ok(()),
                Err(TryRecvError::Disconnected) => return Err(()),
            }
        }
    }
}

pub(crate) struct SenderIo<Writer: AsyncWrite + Unpin> {
    work_queue: Receiver<SetStateRequest>,
    pending_calls_queue: Sender<IcingaPendingCall>,
    abort_oneshot: oneshot::Receiver<()>,
    icinga_connection_writer: WriteHalf<Writer>,
    object_update_receiver: Receiver<UpdateObject>,
}

pub(crate) enum SenderIoAction {
    Send(SetStateRequest),
    UpdateState(UpdateObject),
    Heartbeat,
    Abort,
}

impl<Writer: AsyncWrite + Unpin + Send> SenderIo<Writer> {
    pub async fn abort(self) -> io::Result<()> {
        let SenderIo { mut icinga_connection_writer, .. } = self;

        trace!("shutting down");

        icinga_connection_writer.flush().await?;
        icinga_connection_writer.shutdown().await
    }

    pub async fn send_request(
        &mut self,
        request: IcingaMethods,
    ) -> io::Result<Option<IcingaTimestamp>> {
        let json_rpc = JsonRpc::from(request);
        let msg = serde_json::to_vec(&json_rpc).expect("Json will always serialize.");
        self.icinga_connection_writer.write_netstring(&msg).await?;
        Ok(json_rpc.get_timestamp())
    }

    pub async fn queue_pending_call(&mut self, call: IcingaPendingCall) {
        // this should never fail
        let _ = self.pending_calls_queue.send_async(call).await;
    }

    pub async fn heartbeat(&mut self) -> io::Result<()> {
        let request = JsonRpc::from(IcingaMethods::HeartBeats {});
        let data = serde_json::to_string(&request).unwrap();
        self.icinga_connection_writer.write_netstring(data.as_bytes()).await?;
        self.icinga_connection_writer.flush().await
    }

    pub async fn get_next_action(&mut self, timeout: &mut Instant) -> Result<SenderIoAction, ()> {
        let SenderIo { work_queue, abort_oneshot, object_update_receiver, .. } = self;

        let res = select! {
            val = Self::wait_for_work(work_queue) => {
                trace!("Received message to send");
                val
            }
            val = Self::wait_for_abortion(abort_oneshot) => {
                trace!("Received instruction to close connection.");
                val
            }
            val = Self::wait_for_heartbeat(timeout) =>  {
                trace!("Next heartbeat.");
                val
            }
            val = Self::wait_for_update_object_message(object_update_receiver) => {
                trace!("Receive update object request");
                val
            }
        };

        res
    }

    async fn wait_for_work(queue: &mut Receiver<SetStateRequest>) -> Result<SenderIoAction, ()> {
        match queue.recv_async().await {
            Ok(request) => Ok(SenderIoAction::Send(request)),
            Err(_err) => Err(()),
        }
    }

    async fn wait_for_heartbeat(timeout: &mut Instant) -> Result<SenderIoAction, ()> {
        tokio::time::sleep(Duration::from_secs(5) - timeout.elapsed()).await;
        *timeout = Instant::now();
        Ok(SenderIoAction::Heartbeat)
    }

    async fn wait_for_abortion(oneshot: &mut oneshot::Receiver<()>) -> Result<SenderIoAction, ()> {
        match oneshot.await {
            Ok(()) => Ok(SenderIoAction::Abort),
            Err(_error) => Err(()),
        }
    }

    async fn wait_for_update_object_message(
        receiver: &mut Receiver<UpdateObject>,
    ) -> Result<SenderIoAction, ()> {
        match receiver.recv_async().await {
            Ok(update_object) => Ok(SenderIoAction::UpdateState(update_object)),
            Err(_) => Err(()),
        }
    }
}

pub async fn worker(
    config: SatelliteSmartMonitoringConfig,
    channel: Receiver<SetStateRequest>,
) -> Infallible {
    let mut dropped_messages: Option<Vec<SetStateRequest>> = None;
    let config = Arc::new(config);

    loop {
        let connection = match start_connection(&config).await {
            Ok(conn) => conn,
            Err(err) => {
                error!(
                    "Encountered error: {}. Retrying in {} seconds",
                    err,
                    config.retry_time.as_secs_f32()
                );
                sleep(config.retry_time).await;
                continue;
            }
        };

        let (sender_input, sender_io, receiver_io) = worker_io(connection, &config);

        let sender = tokio::spawn(sender(sender_io));

        let receiver = tokio::spawn(receiver(config.clone(), receiver_io));
        'send_messages: {
            if let Some(messages) = dropped_messages.take() {
                let mut iter = messages.into_iter();
                while let Some(request) = iter.next() {
                    if let Err(err) = sender_input.send_async(request).await {
                        let mut dropped = vec![err.0];
                        dropped.extend(iter);
                        dropped_messages = Some(dropped);
                        break 'send_messages;
                    }
                }
            }

            while let Ok(request) = channel.recv_async().await {
                match sender_input.send_async(request).await {
                    Ok(_) => {}
                    Err(err) => {
                        dropped_messages = Some(vec![err.0]);
                        break 'send_messages;
                    }
                }
            }
        }

        let _ = sender.await;
        match receiver.await {
            Ok(mut dropped) => match dropped_messages {
                None => dropped_messages = Some(dropped),
                Some(remaining) => {
                    dropped.extend(remaining);
                    dropped_messages = Some(dropped);
                }
            },
            Err(_) => dropped_messages = None,
        }

        sleep(config.retry_time).await;
    }
}

async fn sender<Writer: AsyncWrite + Unpin + Send>(mut sender_io: SenderIo<Writer>) {
    let mut internal_state = IcingaObjectStore::new();

    loop {
        let mut timeout = Instant::now();
        match sender_io.get_next_action(&mut timeout).await {
            Ok(SenderIoAction::Send(request)) => {
                if !internal_state.exists(&request.host.name) {
                    let params = match UpdateObjectParams::create_host(&request.host.name) {
                        Ok(mut builder) => {
                            builder.config = Some(request.host.configuration());
                            builder.zone = request.host.zone.clone();
                            builder.modified_attributes = request.host.vars.clone();
                            builder.build()
                        }
                        Err(err) => {
                            error!("{}", err);
                            continue;
                        }
                    };

                    if let Err(err) =
                        sender_io.send_request(IcingaMethods::UpdateObject(params)).await
                    {
                        error!(
                            "Could not create host \"{}\"in icinga2. Error: {}",
                            request.host.name, err
                        );
                    }
                }

                if let Some(service) = &request.service {
                    if !internal_state.exists_service(&request.host.name, &service.name) {
                        let params = match UpdateObjectParams::create_service(
                            &request.host.name,
                            &service.name,
                        ) {
                            Ok(mut builder) => {
                                builder.config = Some(request.host.configuration());
                                builder.modified_attributes = request.host.vars.clone();
                                builder.build()
                            }
                            Err(err) => {
                                error!("{}", err);
                                continue;
                            }
                        };

                        if let Err(err) =
                            sender_io.send_request(IcingaMethods::UpdateObject(params)).await
                        {
                            error!(
                                "Could not create host \"{}!{}\"in icinga2. Error: {}",
                                request.host.name, service.name, err
                            );
                        }
                    }
                }

                let result = sender_io
                    .send_request(IcingaMethods::CheckResult(request.check_result.clone()))
                    .await;

                let ts = match &result {
                    Ok(Some(ts)) => *ts,
                    _ => IcingaTimestamp::default(),
                };

                let error_message = result
                    .map_err(|err| {
                        format!(
                            "Could not set check_result for object \"{}\" in icinga2. Error: {}",
                            request.check_result.name(),
                            err
                        )
                    })
                    .err();

                sender_io.queue_pending_call((ts, request)).await;

                if let Some(err) = error_message {
                    error!("{}", err);
                    break;
                }
            }
            Ok(SenderIoAction::Heartbeat) => {
                if let Err(err) = sender_io.heartbeat().await {
                    error!("Could not send heartbeat to icinga2. Error: {}", err);
                    break;
                }
            }
            Ok(SenderIoAction::Abort) => {
                info!("The worker was signaled to stop. Aborting connection.");
                if let Err(err) = sender_io.abort().await {
                    info!("An error occurred while aborting the connection: {}", err)
                }
                break;
            }
            Ok(SenderIoAction::UpdateState(UpdateObject::Update { name })) => {
                trace!("Inserting object {} into known objects", name);
                internal_state.insert(name);
                println!("{:?}", internal_state)
            }
            Ok(SenderIoAction::UpdateState(UpdateObject::Delete { name })) => {
                internal_state.delete(&name);
            }
            Err(_) => {
                error!("The connection closed prematurely. Aborting worker.");
                break;
            }
        }
    }
}

async fn receiver<Reader: AsyncRead + Send>(
    icinga_config: Arc<SatelliteSmartMonitoringConfig>,
    mut receiver_io: ReceiverIo<Reader>,
) -> Vec<SetStateRequest> {
    info!("Spawned receiver-worker");

    let mut buffer = vec![0u8; icinga_config.intern_buffer_size];
    let mut stored = None;

    let mut last_heartbeat = Instant::now();

    loop {
        let sleep = tokio::time::sleep(Duration::from_secs_f64(
            60.0 - last_heartbeat.elapsed().as_secs_f64(),
        ));

        let json_rpc_result = select! {
            val = receiver_io.read_message(&mut buffer) => { val },
            _ = sleep => {
                break
            }
        };

        let json_rpc = match json_rpc_result {
            Ok(json_rpc) => json_rpc,
            Err(IcingaSatelliteError::JsonRpcError(err)) => {
                trace!("Error {}. Skipping message.", err);
                continue;
            }
            Err(err) => {
                trace!("Error {}. Closing connection", err);
                break;
            }
        };

        match json_rpc.method {
            IcingaMethods::SetLogPosition(log_position) => {
                let result = receiver_io.pop_from_work_queue(&mut stored, log_position);
                if result.is_err() {
                    break;
                }
            }
            IcingaMethods::UpdateObject(object) => {
                trace!("Should update object {}", object.name);

                let result = receiver_io
                    .object_update_sender
                    .send_async(UpdateObject::Update { name: object.name })
                    .await;
                if result.is_err() {
                    break;
                }
            }
            IcingaMethods::DeleteObject(object) => {
                trace!("Should delete object {}", object.name);

                let result = receiver_io
                    .object_update_sender
                    .send_async(UpdateObject::Delete { name: object.name })
                    .await;
                if result.is_err() {
                    break;
                }
            }
            IcingaMethods::UpdateCertificate(request) => {
                if let Err(err) = handle_update_certificate(&request, &icinga_config).await {
                    error!("Could not update the certificates for the satellite. Error: {}", err)
                };
                break;
            }
            IcingaMethods::HeartBeats {} => last_heartbeat = Instant::now(),
            _ => {}
        }
    }

    receiver_io.abort(stored).await
}

mod certificates {
    use super::*;

    async fn save(path: &Path, data: &[u8]) -> Result<(), SatelliteConfigurationError> {
        write(path, data).await.map_err(|error| SatelliteConfigurationError::FileIoError {
            file: path.to_string_lossy().to_string(),
            error,
        })
    }

    pub(crate) async fn save_ca(
        config: &SatelliteSmartMonitoringConfig,
        data: &[u8],
    ) -> Result<(), SatelliteConfigurationError> {
        let path = config.ca_path();
        save(&path, data).await
    }

    pub(crate) async fn save_cert(
        config: &SatelliteSmartMonitoringConfig,
        data: &[u8],
    ) -> Result<(), SatelliteConfigurationError> {
        let path = config.cert_path();
        save(&path, data).await
    }

    pub(crate) async fn save_fpr(
        config: &SatelliteSmartMonitoringConfig,
        data: &[u8],
    ) -> Result<(), SatelliteConfigurationError> {
        let path = config.fpr_path();
        save(&path, data).await
    }

    async fn load(path: &Path) -> Result<Vec<u8>, SatelliteConfigurationError> {
        read(path).await.map_err(|error| SatelliteConfigurationError::FileIoError {
            file: path.to_string_lossy().to_string(),
            error,
        })
    }

    pub(crate) async fn load_ca(
        config: &SatelliteSmartMonitoringConfig,
    ) -> Result<RootCertStore, SatelliteConfigurationError> {
        let ca_path = config.ca_path();
        let data = load(&ca_path).await?;
        let certs = rustls_pemfile::certs(&mut data.as_ref())?;

        let mut root_cert_store = RootCertStore::empty();
        let (added, ignored) = root_cert_store.add_parsable_certificates(&certs);

        if ignored > 0 || added == 0 {
            return Err(SatelliteConfigurationError::ConfigurationError(format!(
                "Could not load root ca for the icinga satellite. Added: {}, Ignored: {}",
                added, ignored
            )));
        }

        Ok(root_cert_store)
    }

    pub(crate) async fn load_cert(
        config: &SatelliteSmartMonitoringConfig,
    ) -> Result<Vec<Certificate>, SatelliteConfigurationError> {
        let cert_path = config.cert_path();
        let content = load(&cert_path).await?;
        let cert_chain = rustls_pemfile::certs(&mut content.as_slice())?;
        Ok(cert_chain.into_iter().map(Certificate).collect())
    }

    pub(crate) async fn load_key(
        config: &SatelliteSmartMonitoringConfig,
    ) -> Result<PrivateKey, SatelliteConfigurationError> {
        let key_path = config.key_path();
        let content = load(&key_path).await?;

        let key = rustls_pemfile::pkcs8_private_keys(&mut content.as_slice())?;

        match key.into_iter().map(PrivateKey).next() {
            None => Err(SatelliteConfigurationError::ConfigurationError(format!(
                "Could not find a rsa key in the file '{}'",
                key_path.to_string_lossy()
            ))),
            Some(key) => Ok(key),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{EndPointConfig, SatelliteSmartMonitoringConfig};
    use crate::satellite::{
        receive_icinga_hello, receiver, send_icinga_hello, send_request_cert, sender, worker_io,
        SetStateRequest, UpdateObject,
    };
    use icinga::model::config::Host;
    use icinga::model::{
        CheckResultParams, HostCheckResult, HostState, IcingaMethods, IcingaTimestamp, JsonRpc,
        UpdateObjectParams,
    };
    use std::fs::read;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio_rustls::rustls::ServerName;
    use tokio_test::io::Builder;

    fn test_config() -> SatelliteSmartMonitoringConfig {
        SatelliteSmartMonitoringConfig {
            endpoint: EndPointConfig {
                address: ServerName::try_from("icinga-master").unwrap(),
                port: 5665,
                host_name: "icinga-rust-satellite".to_string(),
                ticket: "2e37eb916e2aef528b517bc7c49e530b70582329".to_string(),
            },
            cert_root_path: PathBuf::from("./test_data/"),
            retry_time: Duration::from_secs(5),
            max_parallel_send: 10,
            intern_buffer_size: 4096,
        }
    }

    fn set_host_state_test_request() -> SetStateRequest {
        let params = CheckResultParams::for_host(
            "myHost".to_string(),
            HostCheckResult { state: HostState::UP, cr: Box::default() },
        );

        SetStateRequest {
            host: Host {
                name: "myHost".to_string(),
                import: None,
                check_command: Some("dummy".to_string().into()),
                zone: None,
                groups: None,
                address: None,
                address6: None,
                max_check_attempts: None,
                check_interval: None,
                retry_interval: None,
                enable_active_checks: None,
                vars: None,
            },
            service: None,
            check_result: params,
        }
    }

    fn logger() {
        logger();
    }

    #[tokio::test]
    async fn should_send_icinga_hello() {
        logger();

        let data = r#"86:{"jsonrpc":"2.0","method":"icinga::Hello","params":{"capabilities":0,"version":21300}},"#.as_bytes();
        let mut builder = Builder::new();
        builder.write(data);
        let mut mock = builder.build();

        send_icinga_hello(&mut mock).await.unwrap()
    }

    #[tokio::test]
    async fn should_receive_icinga_hello() {
        logger();

        let data = r#"86:{"jsonrpc":"2.0","method":"icinga::Hello","params":{"capabilities":0,"version":21300}},"#.as_bytes();
        let mut builder = Builder::new();
        builder.read(data);
        let mut mock = builder.build();

        receive_icinga_hello(&mut mock).await.unwrap()
    }

    #[tokio::test]
    async fn should_send_request_certificate() {
        logger();

        let config = test_config();

        let data = read("./test_data/expected/request_certificate.json.netstring").unwrap();
        let mut builder = Builder::new();
        builder.write(&data);
        let mut mock = builder.build();

        send_request_cert(&config, &mut mock).await.unwrap()
    }

    #[tokio::test]
    async fn test_create_object_before_check_result() {
        logger();

        let config = test_config();
        let create_object_request = r#"174:{"jsonrpc":"2.0","method":"config::UpdateObject","params":{"name":"myHost","type":"Host","version":0.0,"config":"object Host \"myHost\" {\n    check_command = \"dummy\"\n}"}},"#.as_bytes();
        let set_state_request = r#"136:{"jsonrpc":"2.0","method":"event::CheckResult","params":{"host":"myHost","cr":{"state":0.0,"type":"CheckResult","performance_data":[]}}},"#.as_bytes();

        let mut builder = Builder::new();
        builder.write(create_object_request);
        builder.write(set_state_request);
        let mock = builder.build();

        let (input, sender_io, receiver_io) = worker_io(mock, &config);

        let _handle = tokio::spawn(sender(sender_io));

        let request = set_host_state_test_request();
        input.send_async(request).await.unwrap();
        dbg!(receiver_io.work_queue.recv_async().await.unwrap());
    }

    #[tokio::test]
    async fn test_sender_check_result_on_existing_object() {
        logger();

        let config = test_config();
        let set_state_request = r#"136:{"jsonrpc":"2.0","method":"event::CheckResult","params":{"host":"myHost","cr":{"state":0.0,"type":"CheckResult","performance_data":[]}}},"#.as_bytes();

        let mut builder = Builder::new();
        builder.write(set_state_request);
        let mock = builder.build();

        let (input, sender_io, receiver_io) = worker_io(mock, &config);

        let _handle = tokio::spawn(sender(sender_io));

        let request = set_host_state_test_request();

        receiver_io
            .object_update_sender
            .send_async(UpdateObject::Update { name: "myHost".to_string() })
            .await
            .unwrap();

        input.send_async(request).await.unwrap();
        dbg!(receiver_io.work_queue.recv_async().await.unwrap());
    }

    #[tokio::test]
    async fn test_receiver_handle_update_object_messages() {
        logger();

        let config = Arc::new(test_config());

        let update_object_request = JsonRpc::from(IcingaMethods::UpdateObject(
            UpdateObjectParams::create_host("myHost").unwrap().build(),
        ));
        let update_object_message = serde_json::to_string(&update_object_request).unwrap();
        let update_object_message =
            format!("{}:{},", update_object_message.len(), update_object_message);

        let mut builder = Builder::new();
        builder.read(update_object_message.as_bytes());
        let mock = builder.build();

        let (_input, sender_io, receiver_io) = worker_io(mock, &config);

        let _handle = tokio::spawn(receiver(config, receiver_io));

        assert_eq!(
            UpdateObject::Update { name: "myHost".to_string() },
            sender_io.object_update_receiver.recv_async().await.unwrap()
        );
    }

    #[tokio::test]
    async fn test_receiver_return_dropped_messages() {
        let mut config = test_config();
        config.max_parallel_send = 0;

        let config = Arc::new(config);

        let mut builder = Builder::new();
        let mock = builder.build();

        let (_input, mut sender_io, receiver_io) = worker_io(mock, &config);
        let handle_receiver = tokio::spawn(receiver(config, receiver_io));
        sender_io.queue_pending_call((IcingaTimestamp::now(), set_host_state_test_request())).await;

        sender_io.abort().await.unwrap();

        let remaining = handle_receiver.await;

        assert_eq!(vec![set_host_state_test_request()], remaining.unwrap());
    }
}
