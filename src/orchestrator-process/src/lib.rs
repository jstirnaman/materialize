// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use std::collections::HashMap;
use std::env;
use std::fmt::Debug;
use std::fs::Permissions;
use std::future::Future;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{bail, Context};
use async_stream::stream;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;
use itertools::Itertools;
use libc::{SIGABRT, SIGBUS, SIGILL, SIGSEGV, SIGTRAP};
use scopeguard::defer;
use sha1::{Digest, Sha1};
use sysinfo::{Pid, PidExt, ProcessExt, ProcessRefreshKind, System, SystemExt};
use tokio::fs;
use tokio::process::{Child, Command};
use tokio::sync::broadcast::{self, Sender};
use tokio::time::{self, Duration};
use tracing::{error, info, warn};

use mz_orchestrator::{
    NamespacedOrchestrator, Orchestrator, Service, ServiceConfig, ServiceEvent, ServicePort,
    ServiceProcessMetrics, ServiceStatus,
};
use mz_ore::cast::{CastFrom, ReinterpretCast};
use mz_ore::netio::UnixSocketAddr;
use mz_ore::result::ResultExt;
use mz_ore::task::{AbortOnDropHandle, JoinHandleExt};
use mz_pid_file::PidFile;

pub mod secrets;

/// Configures a [`ProcessOrchestrator`].
#[derive(Debug, Clone)]
pub struct ProcessOrchestratorConfig {
    /// The directory in which the orchestrator should look for executable
    /// images.
    pub image_dir: PathBuf,
    /// Whether to supress output from spawned subprocesses.
    pub suppress_output: bool,
    /// The ID of the environment under orchestration.
    pub environment_id: String,
    /// The directory in which to store secrets.
    pub secrets_dir: PathBuf,
    /// A command to wrap the child command invocation
    pub command_wrapper: Vec<String>,
    /// Whether to crash this process if a child process crashes.
    pub propagate_crashes: bool,
}

/// An orchestrator backed by processes on the local machine.
///
/// **This orchestrator is for development only.** Due to limitations in the
/// Unix process API, it does not exactly conform to the documented semantics
/// of `Orchestrator`.
///
/// Processes launched by this orchestrator must support a `--pid-file-location`
/// command line flag which causes a PID file to be emitted at the specified
/// path.
#[derive(Debug)]
pub struct ProcessOrchestrator {
    image_dir: PathBuf,
    suppress_output: bool,
    namespaces: Mutex<HashMap<String, Arc<dyn NamespacedOrchestrator>>>,
    metadata_dir: PathBuf,
    secrets_dir: PathBuf,
    command_wrapper: Vec<String>,
    propagate_crashes: bool,
}

impl ProcessOrchestrator {
    /// Creates a new process orchestrator from the provided configuration.
    pub async fn new(
        ProcessOrchestratorConfig {
            image_dir,
            suppress_output,
            environment_id,
            secrets_dir,
            command_wrapper,
            propagate_crashes,
        }: ProcessOrchestratorConfig,
    ) -> Result<ProcessOrchestrator, anyhow::Error> {
        let metadata_dir = env::temp_dir().join(format!("environmentd-{environment_id}"));
        fs::create_dir_all(&metadata_dir)
            .await
            .context("creating metadata directory")?;
        fs::create_dir_all(&secrets_dir)
            .await
            .context("creating secrets directory")?;
        fs::set_permissions(&secrets_dir, Permissions::from_mode(0o700))
            .await
            .context("setting secrets directory permissions")?;

        Ok(ProcessOrchestrator {
            image_dir: fs::canonicalize(image_dir).await?,
            suppress_output,
            namespaces: Mutex::new(HashMap::new()),
            metadata_dir: fs::canonicalize(metadata_dir).await?,
            secrets_dir: fs::canonicalize(secrets_dir).await?,
            command_wrapper,
            propagate_crashes,
        })
    }
}

impl Orchestrator for ProcessOrchestrator {
    fn namespace(&self, namespace: &str) -> Arc<dyn NamespacedOrchestrator> {
        let (service_event_tx, _) = broadcast::channel(16384);
        let mut namespaces = self.namespaces.lock().expect("lock poisoned");
        Arc::clone(namespaces.entry(namespace.into()).or_insert_with(|| {
            Arc::new(NamespacedProcessOrchestrator {
                namespace: namespace.into(),
                image_dir: self.image_dir.clone(),
                suppress_output: self.suppress_output,
                secrets_dir: self.secrets_dir.clone(),
                metadata_dir: self.metadata_dir.clone(),
                command_wrapper: self.command_wrapper.clone(),
                services: Arc::new(Mutex::new(HashMap::new())),
                service_event_tx,
                system: Mutex::new(System::new()),
                propagate_crashes: self.propagate_crashes,
            })
        }))
    }
}

#[derive(Debug)]
struct NamespacedProcessOrchestrator {
    namespace: String,
    image_dir: PathBuf,
    suppress_output: bool,
    secrets_dir: PathBuf,
    metadata_dir: PathBuf,
    command_wrapper: Vec<String>,
    services: Arc<Mutex<HashMap<String, Vec<ProcessState>>>>,
    service_event_tx: Sender<ServiceEvent>,
    system: Mutex<System>,
    propagate_crashes: bool,
}

#[async_trait]
impl NamespacedOrchestrator for NamespacedProcessOrchestrator {
    async fn fetch_service_metrics(
        &self,
        id: &str,
    ) -> Result<Vec<ServiceProcessMetrics>, anyhow::Error> {
        let pids: Vec<_> = {
            let services = self.services.lock().expect("lock poisoned");
            let Some(service) = services.get(id) else {
                bail!("unknown service {id}")
            };
            service.iter().map(|p| p.pid()).collect()
        };

        let mut system = self.system.lock().expect("lock poisoned");
        let mut metrics = vec![];
        for pid in pids {
            let (cpu_nano_cores, memory_bytes) = match pid {
                None => (None, None),
                Some(pid) => {
                    system.refresh_process_specifics(pid, ProcessRefreshKind::new().with_cpu());
                    match system.process(pid) {
                        None => (None, None),
                        Some(process) => {
                            // TODO(benesch): find a way to express this that
                            // does not involve using `as`.
                            #[allow(clippy::as_conversions)]
                            let cpu = (process.cpu_usage() * 10_000_000.0) as u64;
                            let memory = process.memory();
                            (Some(cpu), Some(memory))
                        }
                    }
                }
            };
            metrics.push(ServiceProcessMetrics {
                cpu_nano_cores,
                memory_bytes,
            });
        }
        Ok(metrics)
    }

    async fn ensure_service(
        &self,
        id: &str,
        ServiceConfig {
            image,
            init_container_image: _,
            args,
            ports,
            memory_limit: _,
            cpu_limit: _,
            scale,
            labels: _,
            availability_zone: _,
            anti_affinity: _,
        }: ServiceConfig<'_>,
    ) -> Result<Box<dyn Service>, anyhow::Error> {
        let full_id = format!("{}-{}", self.namespace, id);

        let run_dir = self.metadata_dir.join(&full_id);
        fs::create_dir_all(&run_dir)
            .await
            .context("creating run directory")?;

        let mut services = self.services.lock().expect("lock poisoned");
        let process_states = services.entry(id.to_string()).or_default();

        // Drop the state for any processes we no longer need.
        process_states.truncate(scale.get());

        // Create the state for new processes.
        for i in process_states.len()..scale.get() {
            let handle = mz_ore::task::spawn(
                || format!("process-orchestrator:{full_id}-{i}"),
                self.supervise_service_process(ServiceProcessConfig {
                    id: id.to_string(),
                    run_dir: run_dir.clone(),
                    i,
                    image: image.clone(),
                    args,
                    ports: ports.clone(),
                }),
            );
            process_states.push(ProcessState {
                _handle: handle.abort_on_drop(),
                status: ProcessStatus::NotReady,
                status_time: Utc::now(),
            });
        }

        Ok(Box::new(ProcessService {
            run_dir,
            scale: scale.get(),
        }))
    }

    async fn drop_service(&self, id: &str) -> Result<(), anyhow::Error> {
        let mut supervisors = self.services.lock().expect("lock poisoned");
        supervisors.remove(id);
        Ok(())
    }

    async fn list_services(&self) -> Result<Vec<String>, anyhow::Error> {
        let supervisors = self.services.lock().expect("lock poisoned");
        Ok(supervisors.keys().cloned().collect())
    }

    fn watch_services(&self) -> BoxStream<'static, Result<ServiceEvent, anyhow::Error>> {
        let mut initial_events = vec![];
        let mut service_event_rx = {
            let services = self.services.lock().expect("lock poisoned");
            for (service_id, process_states) in &*services {
                for (process_id, process_state) in process_states.iter().enumerate() {
                    initial_events.push(ServiceEvent {
                        service_id: service_id.clone(),
                        process_id: u64::cast_from(process_id),
                        status: process_state.status.into(),
                        time: process_state.status_time,
                    });
                }
            }
            self.service_event_tx.subscribe()
        };
        Box::pin(stream! {
            for event in initial_events {
                yield Ok(event);
            }
            loop {
                yield service_event_rx.recv().await.err_into();
            }
        })
    }
}

impl NamespacedProcessOrchestrator {
    fn supervise_service_process(
        &self,
        ServiceProcessConfig {
            id,
            run_dir,
            i,
            image,
            args,
            ports,
        }: ServiceProcessConfig,
    ) -> impl Future<Output = ()> {
        let suppress_output = self.suppress_output;
        let propagate_crashes = self.propagate_crashes;
        let command_wrapper = self.command_wrapper.clone();
        let image = self.image_dir.join(image);
        let pid_file = run_dir.join(format!("{i}.pid"));
        let full_id = format!("{}-{}", self.namespace, id);

        let state_updater = ProcessStateUpdater {
            namespace: self.namespace.clone(),
            id,
            i,
            services: Arc::clone(&self.services),
            service_event_tx: self.service_event_tx.clone(),
        };

        let listen_addrs = ports
            .into_iter()
            .map(|p| {
                let addr = socket_path(&run_dir, &p.name, i);
                (p.name, addr)
            })
            .collect();
        let mut args = args(&listen_addrs);
        args.push(format!("--pid-file-location={}", pid_file.display()));
        args.push("--secrets-reader=process".into());
        args.push(format!(
            "--secrets-reader-process-dir={}",
            self.secrets_dir.display()
        ));

        async move {
            supervise_existing_process(&state_updater, &pid_file).await;

            loop {
                for path in listen_addrs.values() {
                    if let Err(e) = fs::remove_file(path).await {
                        warn!("unable to remove {path} while launching {full_id}-{i}: {e}")
                    }
                }

                let mut cmd = if command_wrapper.is_empty() {
                    let mut cmd = Command::new(&image);
                    cmd.args(&args);
                    cmd
                } else {
                    let mut cmd = Command::new(&command_wrapper[0]);
                    cmd.args(
                        command_wrapper[1..]
                            .iter()
                            .map(|part| interpolate_command(part, &full_id, &listen_addrs)),
                    );
                    cmd.arg(&image);
                    cmd.args(&args);
                    cmd
                };
                info!(
                    "launching {full_id}-{i} via {}...",
                    cmd.as_std()
                        .get_args()
                        .map(|arg| arg.to_string_lossy())
                        .join(" ")
                );
                if suppress_output {
                    cmd.stdout(Stdio::null());
                    cmd.stderr(Stdio::null());
                }
                match spawn_process(&state_updater, cmd).await {
                    Ok(status) => {
                        if propagate_crashes && did_process_crash(status) {
                            panic!("{full_id}-{i} crashed; aborting because propagate_crashes is enabled");
                        }
                        error!("{full_id}-{i} exited: {:?}; relaunching in 5s", status);
                    }
                    Err(e) => {
                        error!("{full_id}-{i} failed to spawn: {}; relaunching in 5s", e);
                    }
                };
                state_updater.update_state(ProcessStatus::NotReady);
                time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

struct ServiceProcessConfig<'a> {
    id: String,
    run_dir: PathBuf,
    i: usize,
    image: String,
    args: &'a (dyn Fn(&HashMap<String, String>) -> Vec<String> + Send + Sync),
    ports: Vec<ServicePort>,
}

/// Supervises an existing process, if it exists.
async fn supervise_existing_process(state_updater: &ProcessStateUpdater, pid_file: &Path) {
    let name = format!(
        "{}-{}-{}",
        state_updater.namespace, state_updater.id, state_updater.i
    );

    let Ok(pid) = PidFile::read(pid_file) else {
        return;
    };

    let pid = Pid::from_u32(u32::reinterpret_cast(pid));
    let mut system = System::new();
    system.refresh_process_specifics(pid, ProcessRefreshKind::new());
    let Some(process) = system.process(pid) else {
        return;
    };

    info!(%pid, "discovered existing process for {name}");
    state_updater.update_state(ProcessStatus::Ready { pid });

    // Kill the process if the future is dropped.
    let need_kill = AtomicBool::new(true);
    defer! {
        state_updater.update_state(ProcessStatus::NotReady);
        if need_kill.load(Ordering::SeqCst) {
            info!(%pid, "terminating existing process for {name}");
            process.kill();
        }
    }

    // Periodically check if the process has terminated.
    let mut system = System::new();
    while system.refresh_process_specifics(pid, ProcessRefreshKind::new()) {
        time::sleep(Duration::from_secs(5)).await;
    }

    // The process has crashed. Exit the function without attempting to
    // kill it.
    warn!(%pid, "process for {name} has crashed; will reboot");
    need_kill.store(false, Ordering::SeqCst)
}

fn interpolate_command(
    command_part: &str,
    full_id: &str,
    ports: &HashMap<String, String>,
) -> String {
    let mut command_part = command_part.replace("%N", full_id);
    for (endpoint, port) in ports {
        command_part = command_part.replace(&format!("%P:{endpoint}"), port);
    }
    command_part
}

async fn spawn_process(
    state_updater: &ProcessStateUpdater,
    mut cmd: Command,
) -> Result<ExitStatus, anyhow::Error> {
    struct KillOnDropChild(Child);

    impl Drop for KillOnDropChild {
        fn drop(&mut self) {
            let _ = self.0.start_kill();
        }
    }

    let mut child = KillOnDropChild(cmd.spawn()?);
    state_updater.update_state(ProcessStatus::Ready {
        pid: Pid::from_u32(child.0.id().unwrap()),
    });
    Ok(child.0.wait().await?)
}

fn did_process_crash(status: ExitStatus) -> bool {
    // Likely not exhaustive. Feel free to add additional tests for other
    // indications of a crashed child process, as those conditions are
    // discovered.
    matches!(
        status.signal(),
        Some(SIGABRT | SIGBUS | SIGSEGV | SIGTRAP | SIGILL)
    )
}

struct ProcessStateUpdater {
    namespace: String,
    id: String,
    i: usize,
    services: Arc<Mutex<HashMap<String, Vec<ProcessState>>>>,
    service_event_tx: Sender<ServiceEvent>,
}

impl ProcessStateUpdater {
    fn update_state(&self, status: ProcessStatus) {
        let mut services = self.services.lock().expect("lock poisoned");
        let Some(process_states) = services.get_mut(&self.id) else {
            return;
        };
        let Some(process_state) = process_states.get_mut(self.i) else {
            return;
        };
        let status_time = Utc::now();
        process_state.status = status;
        process_state.status_time = status_time;
        let _ = self.service_event_tx.send(ServiceEvent {
            service_id: self.id.to_string(),
            process_id: u64::cast_from(self.i),
            status: status.into(),
            time: status_time,
        });
    }
}

#[derive(Debug)]
struct ProcessState {
    _handle: AbortOnDropHandle<()>,
    status: ProcessStatus,
    status_time: DateTime<Utc>,
}

impl ProcessState {
    fn pid(&self) -> Option<Pid> {
        match &self.status {
            ProcessStatus::NotReady => None,
            ProcessStatus::Ready { pid } => Some(*pid),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ProcessStatus {
    NotReady,
    Ready { pid: Pid },
}

impl From<ProcessStatus> for ServiceStatus {
    fn from(status: ProcessStatus) -> ServiceStatus {
        match status {
            ProcessStatus::NotReady => ServiceStatus::NotReady,
            ProcessStatus::Ready { .. } => ServiceStatus::Ready,
        }
    }
}

fn socket_path(run_dir: &Path, port: &str, process: usize) -> String {
    let desired = run_dir
        .join(format!("{port}-{process}"))
        .to_string_lossy()
        .into_owned();
    if UnixSocketAddr::from_pathname(&desired).is_err() {
        // Unix socket addresses have a very low maximum length of around 100
        // bytes on most platforms.
        env::temp_dir()
            .join(hex::encode(Sha1::digest(desired)))
            .display()
            .to_string()
    } else {
        desired
    }
}

#[derive(Debug, Clone)]
struct ProcessService {
    run_dir: PathBuf,
    scale: usize,
}

impl Service for ProcessService {
    fn addresses(&self, port: &str) -> Vec<String> {
        (0..self.scale)
            .map(|i| socket_path(&self.run_dir, port, i))
            .collect()
    }
}
