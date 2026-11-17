use worker::{Container, ContainerExecOptions, ContainerStartupOptions};

fn startup_options() -> Result<ContainerStartupOptions, wasm_bindgen::JsValue> {
    let mut options = ContainerStartupOptions::default();
    options.set_image("example/image:latest");
    options.set_instance("standard-1");
    options.set_custom_instance(1, 1024, 10240)?;
    options.set_container_snapshot("snapshot-id")?;
    options.add_label("service", "test");
    Ok(options)
}

async fn exec(container: &Container) -> worker::Result<()> {
    let mut options = ContainerExecOptions::default();
    options.set_cwd("/tmp");
    options.add_env("MODE", "test");
    options.set_user(1000, 1000);
    options.set_stdout("pipe");
    options.set_stderr("combined");

    let process = container.exec(&["echo", "hello"], Some(options)).await?;
    let _pid = process.pid();
    let _exit_code = process.exit_code().await?;
    process.kill(None)?;

    let output = process.output().await?;
    let _exit_code = output.exit_code();
    let _stdout = output.stdout();
    let _stderr = output.stderr();
    Ok(())
}

fn main() {
    let _ = startup_options;
    let _ = exec;
}
