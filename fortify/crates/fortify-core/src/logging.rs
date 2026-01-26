use std::fs::{File, OpenOptions};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Timelike, Utc};
use sysinfo::System;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

const ROTATION_SECONDS: u64 = 4 * 60 * 60; // 4-hour blocks
const DEFAULT_LOG_DIR: &str = "/tmp/fortify/logs";

#[derive(Clone)]
struct RotatingFileWriter {
    state: Arc<RotatingState>,
}

struct RotatingState {
    dir: PathBuf,
    prefix: String,
    inner: Mutex<InnerWriter>,
}

struct InnerWriter {
    block: u64,
    writer: BufWriter<File>,
}

impl RotatingFileWriter {
    fn new(dir: PathBuf, prefix: String) -> Self {
        let block = current_block();
        let writer = open_writer(&dir, &prefix, block);

        Self {
            state: Arc::new(RotatingState {
                dir,
                prefix,
                inner: Mutex::new(InnerWriter { block, writer }),
            }),
        }
    }
}

impl std::io::Write for RotatingFileWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let block = current_block();
        let mut guard = self.state.inner.lock().unwrap();

        if block != guard.block {
            guard.writer = open_writer(&self.state.dir, &self.state.prefix, block);
            guard.block = block;
        }

        guard.writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut guard = self.state.inner.lock().unwrap();
        guard.writer.flush()
    }
}

fn current_block() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() / ROTATION_SECONDS)
        .unwrap_or(0)
}

fn open_writer(dir: &Path, prefix: &str, block: u64) -> BufWriter<File> {
    let _ = std::fs::create_dir_all(dir);
    let block_start = block * ROTATION_SECONDS;
    let ts = DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_secs(block_start));
    let filename = format!("{}-{}-{:02}h.log", prefix, ts.format("%Y%m%d"), ts.hour());
    let path = dir.join(filename);

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("failed to open log file");

    BufWriter::new(file)
}

pub fn init_logging(service_name: &str) {
    let log_dir = PathBuf::from(DEFAULT_LOG_DIR);
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let file_writer = RotatingFileWriter::new(log_dir, service_name.to_string());
    let file_layer = fmt::layer()
        .with_writer(move || file_writer.clone())
        .with_ansi(false);

    let stdout_layer = fmt::layer();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();
}

pub fn start_resource_logger(service_name: &'static str, interval: Duration) {
    std::thread::spawn(move || {
        let mut sys = System::new();
        let mut peak_cpu: f32 = 0.0;
        let mut sample_count = 0u32;

        loop {
            if let Ok(pid) = sysinfo::get_current_pid() {
                // sysinfo 0.38: refresh_processes with specific process list
                sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), false);
                sys.refresh_memory();

                if let Some(proc) = sys.process(pid) {
                    let cpu = proc.cpu_usage();
                    let mem = proc.memory();
                    let virt = proc.virtual_memory();

                    // Track peak CPU between log intervals
                    if cpu > peak_cpu {
                        peak_cpu = cpu;
                    }

                    sample_count += 1;

                    // Log every 5 samples (15 seconds at 3s interval)
                    if sample_count >= 5 {
                        tracing::info!(
                            target: "resource",
                            service = service_name,
                            cpu_percent = format!("{:.2}", cpu),
                            cpu_peak = format!("{:.2}", peak_cpu),
                            memory_kb = mem,
                            virtual_kb = virt,
                            "resource-usage"
                        );

                        // Reset peak tracker
                        peak_cpu = 0.0;
                        sample_count = 0;
                    }
                }
            }

            std::thread::sleep(interval);
        }
    });
}
