use js_sys::futures::JsFuture;
use js_sys::{Map, Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

use crate::{Fetcher, Result};

#[derive(Debug)]
pub struct Container {
    pub(super) inner: worker_sys::Container,
}

impl Container {
    pub fn running(&self) -> bool {
        self.inner.running()
    }

    pub fn start(&self, options: Option<ContainerStartupOptions>) -> Result<()> {
        let options = match options {
            Some(o) => o.into(),
            None => JsValue::undefined(),
        };
        self.inner.start(&options).map_err(|e| e.into())
    }

    pub async fn wait_for_exit(&self) -> Result<()> {
        let promise = self.inner.monitor();
        JsFuture::from(promise).await?;
        Ok(())
    }

    pub async fn destroy(&self, error: Option<&str>) -> Result<()> {
        let promise = self.inner.destroy(error);
        JsFuture::from(promise).await?;
        Ok(())
    }

    pub fn signal(&self, signo: i32) -> Result<()> {
        self.inner.signal(signo).map_err(|e| e.into())
    }

    pub fn get_tcp_port(&self, port: u16) -> Result<Fetcher> {
        self.inner
            .get_tcp_port(port)
            .map(|f| f.into())
            .map_err(|e| e.into())
    }

    /// Starts another process inside an already-running container. `cmd` is
    /// the executable followed by its arguments; it does not start a shell,
    /// so shell syntax (pipes, redirects, expansion) is not interpreted.
    /// Does not start a stopped container: call this only after confirming
    /// [`Container::running`].
    pub async fn exec(
        &self,
        cmd: &[&str],
        options: Option<ContainerExecOptions>,
    ) -> Result<ExecProcess> {
        let cmd: Vec<JsValue> = cmd.iter().map(|s| JsValue::from_str(s)).collect();
        let options = match options {
            Some(o) => o.into(),
            None => JsValue::undefined(),
        };
        let promise = self.inner.exec(cmd, &options)?;
        let process = JsFuture::from(promise).await?;
        Ok(ExecProcess {
            inner: process.unchecked_into(),
        })
    }
}

unsafe impl Sync for Container {}
unsafe impl Send for Container {}

impl From<worker_sys::Container> for Container {
    fn from(inner: worker_sys::Container) -> Self {
        Self { inner }
    }
}

impl AsRef<JsValue> for Container {
    fn as_ref(&self) -> &JsValue {
        &self.inner
    }
}

impl From<Container> for JsValue {
    fn from(container: Container) -> Self {
        JsValue::from(container.inner)
    }
}

impl JsCast for Container {
    fn instanceof(val: &JsValue) -> bool {
        val.is_instance_of::<worker_sys::Container>()
    }

    fn unchecked_from_js(val: JsValue) -> Self {
        Self { inner: val.into() }
    }

    fn unchecked_from_js_ref(val: &JsValue) -> &Self {
        unsafe { &*(val as *const JsValue as *const Self) }
    }
}

#[wasm_bindgen(getter_with_clone)]
#[derive(Debug, Clone)]
pub struct ContainerStartupOptions {
    pub entrypoint: Vec<String>,
    #[wasm_bindgen(js_name = "enableInternet")]
    pub enable_internet: Option<bool>,
    pub env: Map,
    /// Image reference to start, e.g. a value from `ctx.container.images` or
    /// the `cloudflare/debian-trixie` managed image. `durable_object`
    /// scheduling policy only; required unless `container_snapshot` is set.
    /// Setting both `image` and `container_snapshot` makes `start()` throw a
    /// `TypeError`.
    pub image: Option<String>,
    /// Named instance size (`"lite"`, `"standard-1"`, `"standard-2"`,
    /// `"standard-3"`, or `"standard-4"`) or a custom size object. Defaults
    /// to `"lite"` when unset. `durable_object` scheduling policy only.
    ///
    /// The runtime rejects `"basic"` and the legacy `"dev"`/`"standard"`
    /// aliases here: those names are only valid as a `default`-policy
    /// Wrangler `instance_type`, not as a per-call `durable_object` size.
    /// Source: <https://developers.cloudflare.com/containers/configuration/scheduling-policy/#choose-an-instance-size-at-runtime>,
    /// checked 2026-10-02.
    pub instance: Option<JsValue>,
    /// Snapshot handle to restore before startup, in place of `image`.
    /// `durable_object` scheduling policy only.
    #[wasm_bindgen(js_name = "containerSnapshot")]
    pub container_snapshot: Option<JsValue>,
    /// Up to 10 labels that `inspect()` returns. Label names must contain 1
    /// to 16 bytes; values up to 64 bytes.
    pub labels: Map,
}

impl ContainerStartupOptions {
    pub fn new() -> ContainerStartupOptions {
        ContainerStartupOptions {
            entrypoint: Vec::new(),
            enable_internet: None,
            env: Map::new(),
            image: None,
            instance: None,
            container_snapshot: None,
            labels: Map::new(),
        }
    }

    pub fn set_entrypoint(&mut self, entrypoint: &[&str]) {
        self.entrypoint = entrypoint.iter().map(|s| s.to_string()).collect();
    }

    pub fn enable_internet(&mut self, enable_internet: bool) {
        self.enable_internet = Some(enable_internet);
    }

    pub fn add_env(&mut self, key: &str, value: &str) {
        self.env
            .set(&JsValue::from_str(key), &JsValue::from_str(value));
    }

    /// Sets the image reference to start (`durable_object` scheduling
    /// policy only). Mutually exclusive with `set_container_snapshot`.
    pub fn set_image(&mut self, image: &str) {
        self.image = Some(image.to_string());
    }

    /// Sets a named instance size. See the `instance` field docs for the
    /// accepted names and the `basic`/`default`-policy caveat.
    pub fn set_instance(&mut self, instance: &str) {
        self.instance = Some(JsValue::from_str(instance));
    }

    /// Sets a custom instance size. `memory_mib` is in mebibytes and
    /// `disk_mb` is in megabytes; both must be positive integers within the
    /// platform's custom-instance limits.
    pub fn set_custom_instance(&mut self, vcpu: u32, memory_mib: u32, disk_mb: u32) {
        let obj = Object::new();
        Reflect::set(
            &obj,
            &JsValue::from_str("vcpu"),
            &JsValue::from_f64(vcpu as f64),
        )
        .unwrap();
        Reflect::set(
            &obj,
            &JsValue::from_str("memoryMib"),
            &JsValue::from_f64(memory_mib as f64),
        )
        .unwrap();
        Reflect::set(
            &obj,
            &JsValue::from_str("diskMb"),
            &JsValue::from_f64(disk_mb as f64),
        )
        .unwrap();
        self.instance = Some(obj.into());
    }

    /// Sets the snapshot to restore before startup, by the id returned from
    /// `snapshotContainer()`. Mutually exclusive with `set_image`.
    pub fn set_container_snapshot(&mut self, id: &str) {
        let obj = Object::new();
        Reflect::set(&obj, &JsValue::from_str("id"), &JsValue::from_str(id)).unwrap();
        self.container_snapshot = Some(obj.into());
    }

    pub fn add_label(&mut self, key: &str, value: &str) {
        self.labels
            .set(&JsValue::from_str(key), &JsValue::from_str(value));
    }
}

impl From<ContainerStartupOptions> for Object {
    fn from(options: ContainerStartupOptions) -> Self {
        let obj = options.clone().into();
        if !options.entrypoint.is_empty() {
            Reflect::delete_property(&obj, &JsValue::from_str("entrypoint")).unwrap();
        }
        if options.enable_internet.is_some() {
            Reflect::delete_property(&obj, &JsValue::from_str("enableInternet")).unwrap();
        }
        if options.env.size() != 0 {
            Reflect::delete_property(&obj, &JsValue::from_str("env")).unwrap();
        }
        if options.image.is_none() {
            Reflect::delete_property(&obj, &JsValue::from_str("image")).unwrap();
        }
        if options.instance.is_none() {
            Reflect::delete_property(&obj, &JsValue::from_str("instance")).unwrap();
        }
        if options.container_snapshot.is_none() {
            Reflect::delete_property(&obj, &JsValue::from_str("containerSnapshot")).unwrap();
        }
        if options.labels.size() == 0 {
            Reflect::delete_property(&obj, &JsValue::from_str("labels")).unwrap();
        }
        obj
    }
}

impl Default for ContainerStartupOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Options for [`Container::exec`]. Streaming `stdin` is not exposed by this
/// wrapper; omitting it closes standard input and sends EOF immediately,
/// matching the JS default.
#[wasm_bindgen(getter_with_clone)]
#[derive(Debug, Clone)]
pub struct ContainerExecOptions {
    pub cwd: Option<String>,
    pub env: Map,
    pub user: Option<String>,
    /// `"pipe"` (default) or `"ignore"`.
    pub stdout: Option<String>,
    /// `"pipe"` (default), `"ignore"`, or `"combined"`.
    pub stderr: Option<String>,
}

impl ContainerExecOptions {
    pub fn new() -> ContainerExecOptions {
        ContainerExecOptions {
            cwd: None,
            env: Map::new(),
            user: None,
            stdout: None,
            stderr: None,
        }
    }

    pub fn set_cwd(&mut self, cwd: &str) {
        self.cwd = Some(cwd.to_string());
    }

    pub fn add_env(&mut self, key: &str, value: &str) {
        self.env
            .set(&JsValue::from_str(key), &JsValue::from_str(value));
    }

    /// Sets the Linux user and group id the process runs as.
    pub fn set_user(&mut self, uid: u32, gid: u32) {
        self.user = Some(format!("{uid}:{gid}"));
    }

    /// `"pipe"` (default) or `"ignore"`.
    pub fn set_stdout(&mut self, mode: &str) {
        self.stdout = Some(mode.to_string());
    }

    /// `"pipe"` (default), `"ignore"`, or `"combined"`.
    pub fn set_stderr(&mut self, mode: &str) {
        self.stderr = Some(mode.to_string());
    }
}

impl From<ContainerExecOptions> for Object {
    fn from(options: ContainerExecOptions) -> Self {
        let obj = options.clone().into();
        if options.cwd.is_none() {
            Reflect::delete_property(&obj, &JsValue::from_str("cwd")).unwrap();
        }
        if options.env.size() == 0 {
            Reflect::delete_property(&obj, &JsValue::from_str("env")).unwrap();
        }
        if options.user.is_none() {
            Reflect::delete_property(&obj, &JsValue::from_str("user")).unwrap();
        }
        if options.stdout.is_none() {
            Reflect::delete_property(&obj, &JsValue::from_str("stdout")).unwrap();
        }
        if options.stderr.is_none() {
            Reflect::delete_property(&obj, &JsValue::from_str("stderr")).unwrap();
        }
        obj
    }
}

impl Default for ContainerExecOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// A process started inside a running container via [`Container::exec`].
#[derive(Debug)]
pub struct ExecProcess {
    inner: worker_sys::ExecProcess,
}

impl ExecProcess {
    pub fn pid(&self) -> u32 {
        self.inner.pid()
    }

    /// Resolves when the process exits. Nonzero exit codes resolve
    /// normally; they do not produce an `Err`.
    pub async fn exit_code(&self) -> Result<u32> {
        let promise = self.inner.exit_code();
        let value = JsFuture::from(promise).await?;
        Ok(value.as_f64().unwrap_or_default() as u32)
    }

    /// Reads buffered standard output/error and the exit code once. Throws
    /// when called more than once or after a readable stream has started
    /// being consumed.
    pub async fn output(&self) -> Result<ExecOutput> {
        let promise = self.inner.output()?;
        let value = JsFuture::from(promise).await?;
        Ok(ExecOutput {
            inner: value.unchecked_into(),
        })
    }

    /// Queues a signal for the process; `None` sends the default
    /// `SIGTERM` (15).
    pub fn kill(&self, signal: Option<i32>) -> Result<()> {
        self.inner.kill(signal).map_err(|e| e.into())
    }
}

/// The buffered result of [`ExecProcess::output`].
#[derive(Debug)]
pub struct ExecOutput {
    inner: worker_sys::ExecOutput,
}

impl ExecOutput {
    pub fn exit_code(&self) -> u32 {
        self.inner.exit_code()
    }

    pub fn stdout(&self) -> Vec<u8> {
        Uint8Array::new(&self.inner.stdout()).to_vec()
    }

    pub fn stderr(&self) -> Vec<u8> {
        Uint8Array::new(&self.inner.stderr()).to_vec()
    }
}
