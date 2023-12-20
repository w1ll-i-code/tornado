use crate::icinga2::model::check_result::Command;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt::{Display, Formatter, Write};

pub trait ConfigProvider<'a> {
    type Context;

    fn live_creation_function(&self, ctx: &'a Self::Context) -> String {
        format!(
            r#"existing = {}
if (existing) {{
    "ObjectAlreadyExistingError"
}} else {{
    result = Internal.run_with_activation_context(function () {{
        {}
    }})
    
    if (result) {{
        "Success"
    }} else {{
        "ObjectCreationFailureError"
    }}
}}"#,
            self.getter_function(ctx),
            self.configuration()
        )
    }
    fn configuration(&self) -> String;
    fn getter_function(&self, ctx: &'a Self::Context) -> String;
}

#[derive(Serialize, Deserialize, Default, PartialEq)]
pub struct Host {
    pub name: String,
    pub import: Option<String>,
    pub check_command: Option<Command>,
    pub zone: Option<String>,
    pub groups: Option<Vec<String>>,
    pub address: Option<String>,
    pub address6: Option<String>,

    pub max_check_attempts: Option<String>,
    pub check_interval: Option<String>,
    pub retry_interval: Option<String>,
    pub enable_active_checks: Option<bool>,

    pub vars: Option<Map<String, Value>>,
}

impl Display for Host {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.render_head(f)?;
        self.render_import(f)?;
        self.render_check_command(f)?;
        self.render_zone(f)?;
        self.render_groups(f)?;
        self.render_address(f)?;
        self.render_address6(f)?;
        self.render_max_check_attempts(f)?;
        self.render_check_interval(f)?;
        self.render_retry_interval(f)?;
        self.render_enable_active_checks(f)?;
        self.render_vars(f)?;
        self.render_close(f)
    }
}

impl Host {
    fn render_head(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("object Host ")?;
        self.render_name(f)?;
        f.write_str(" {\n")
    }

    fn render_name(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        render_escaped_string(&self.name, true, f)
    }

    fn render_import(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(import) = &self.import {
            render_import(import, f)?;
        }
        Ok(())
    }

    fn render_check_command(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        const CONFIG_KEY: &str = "check_command";

        if let Some(check_command) = &self.check_command {
            match check_command {
                Command::String(string) => render_kv_property(CONFIG_KEY, string, f)?,
                Command::Vec(array) => render_string_array(CONFIG_KEY, array, f)?,
            };
        }
        Ok(())
    }

    fn render_zone(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(zone) = &self.zone {
            render_kv_property("zone", zone, f)?;
        }
        Ok(())
    }

    fn render_groups(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(groups) = &self.groups {
            render_string_array("groups", groups, f)?;
        }
        Ok(())
    }

    fn render_address(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(address) = &self.address {
            render_kv_property("address", address, f)?;
        }
        Ok(())
    }

    fn render_address6(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(address6) = &self.address6 {
            render_kv_property("address6", address6, f)?;
        }
        Ok(())
    }

    fn render_max_check_attempts(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(max_check_attempts) = &self.max_check_attempts {
            render_kv_property("max_check_attempts", max_check_attempts, f)?;
        }
        Ok(())
    }

    fn render_check_interval(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(check_interval) = &self.check_interval {
            render_kv_property("check_interval", check_interval, f)?;
        }
        Ok(())
    }

    fn render_retry_interval(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(retry_interval) = &self.retry_interval {
            render_kv_property("retry_interval", retry_interval, f)?;
        }
        Ok(())
    }

    fn render_enable_active_checks(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(enable_active_checks) = &self.enable_active_checks {
            render_kv_raw("enable_active_checks", enable_active_checks, f)?;
        }
        Ok(())
    }

    fn render_vars(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(vars) = &self.vars {
            render_indent(1, f)?;
            f.write_str("vars = ")?;
            render_vars(vars, 1, f)?;
            f.write_char('\n')?;
        }
        Ok(())
    }

    fn render_close(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_char('}')
    }
}

impl ConfigProvider<'_> for Host {
    type Context = ();

    fn configuration(&self) -> String {
        format!("{}", self)
    }

    fn getter_function(&self, _ctx: &()) -> String {
        format!(r#"get_host("{}")"#, self.name.escape_default().collect::<String>())
    }
}

#[derive(Serialize, Deserialize, Default, PartialEq)]
pub struct Service {
    pub name: String,
    pub import: Option<String>,
    pub host: Option<String>,
    pub check_command: Option<Command>,
    pub command_endpoint: Option<String>,
    pub enable_notifications: Option<bool>,
    pub enable_active_checks: Option<bool>,
    pub enable_passive_checks: Option<bool>,
    pub enable_event_handler: Option<bool>,
    pub enable_flapping: Option<bool>,
    pub enable_perfdata: Option<bool>,
    pub volatile: Option<bool>,
    pub vars: Option<Map<String, Value>>,
}

impl Display for Service {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.render_head(f)?;
        self.render_import(f)?;
        self.render_host(f)?;
        self.render_check_command(f)?;
        self.render_command_endpoint(f)?;
        self.render_enable_notifications(f)?;
        self.render_enable_active_checks(f)?;
        self.render_enable_passive_checks(f)?;
        self.render_enable_event_handler(f)?;
        self.render_enable_flapping(f)?;
        self.render_enable_perfdata(f)?;
        self.render_volatile(f)?;
        self.render_vars(f)?;
        self.render_close(f)
    }
}

impl Service {
    fn render_head(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("object Service ")?;
        self.render_name(f)?;
        f.write_str(" {\n")
    }

    fn render_name(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        render_escaped_string(&self.name, true, f)
    }

    fn render_import(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(import) = &self.import {
            render_import(import, f)?;
        }
        Ok(())
    }

    fn render_check_command(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(check_command) = &self.check_command {
            const CONFIG_KEY: &str = "check_command";

            match check_command {
                Command::String(string) => render_kv_property(CONFIG_KEY, string, f)?,
                Command::Vec(array) => render_string_array(CONFIG_KEY, array, f)?,
            };
        }
        Ok(())
    }

    fn render_host(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(host) = &self.host {
            render_kv_property("host", host, f)?;
        }
        Ok(())
    }

    fn render_command_endpoint(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(command_endpoint) = &self.command_endpoint {
            render_kv_property("command_endpoint", command_endpoint, f)?;
        }
        Ok(())
    }

    fn render_enable_notifications(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(enable_notifications) = &self.enable_notifications {
            render_kv_raw("enable_notifications", enable_notifications, f)?;
        }
        Ok(())
    }

    fn render_enable_active_checks(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(enable_active_checks) = &self.enable_active_checks {
            render_kv_raw("enable_active_checks", enable_active_checks, f)?;
        }
        Ok(())
    }

    fn render_enable_passive_checks(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(enable_passive_checks) = &self.enable_passive_checks {
            render_kv_raw("enable_passive_checks", enable_passive_checks, f)?;
        }
        Ok(())
    }

    fn render_enable_event_handler(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(enable_event_handler) = &self.enable_event_handler {
            render_kv_raw("enable_event_handler", enable_event_handler, f)?;
        }
        Ok(())
    }

    fn render_enable_flapping(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(enable_flapping) = &self.enable_flapping {
            render_kv_raw("enable_flapping", enable_flapping, f)?;
        }
        Ok(())
    }

    fn render_enable_perfdata(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(enable_perfdata) = &self.enable_perfdata {
            render_kv_raw("enable_perfdata", enable_perfdata, f)?;
        }
        Ok(())
    }

    fn render_volatile(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(volatile) = &self.volatile {
            render_kv_raw("volatile", volatile, f)?;
        }
        Ok(())
    }

    fn render_vars(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(vars) = &self.vars {
            render_indent(1, f)?;
            f.write_str("vars = ")?;
            render_vars(vars, 1, f)?;
            f.write_char('\n')?;
        }
        Ok(())
    }

    fn render_close(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_char('}')
    }
}

impl<'a> ConfigProvider<'a> for Service {
    type Context = &'a str;

    fn configuration(&self) -> String {
        format!("{}", self)
    }

    fn getter_function(&self, ctx: &'a Self::Context) -> String {
        format!(
            r#"get_service("{}", "{}")"#,
            self.name.escape_default().collect::<String>(),
            ctx.escape_default().collect::<String>()
        )
    }
}

fn render_indent(indent: usize, f: &mut Formatter<'_>) -> std::fmt::Result {
    for _ in 0..indent {
        f.write_str("    ")?;
    }
    Ok(())
}

fn render_kv_property(key: &str, value: &str, f: &mut Formatter<'_>) -> std::fmt::Result {
    render_indent(1, f)?;
    render_kv(key, value, f)?;
    f.write_char('\n')
}

fn render_string_array<Str: AsRef<str>>(
    key: &str,
    array: &[Str],
    f: &mut Formatter<'_>,
) -> std::fmt::Result {
    render_indent(1, f)?;
    render_escaped_string(key, false, f)?;
    f.write_str(" = [")?;
    for (i, string) in array.iter().enumerate() {
        if i != 0 {
            f.write_str(", ")?;
        }
        render_escaped_string(string.as_ref(), true, f)?;
    }
    f.write_str("]\n")
}

fn render_kv_raw<T: Display>(key: &str, value: &T, f: &mut Formatter<'_>) -> std::fmt::Result {
    render_indent(1, f)?;
    render_escaped_string(key, false, f)?;
    f.write_str(" = ")?;
    f.write_fmt(format_args!("{}\n", value))
}

fn render_kv(key: &str, value: &str, f: &mut Formatter<'_>) -> std::fmt::Result {
    render_escaped_string(key, false, f)?;
    f.write_str(" = ")?;
    render_escaped_string(value, true, f)
}

fn render_escaped_string(content: &str, force: bool, f: &mut Formatter<'_>) -> std::fmt::Result {
    if !force && content.bytes().all(|c| (b'\x20'..=b'\x7e').contains(&c)) {
        return f.write_str(content);
    }

    f.write_char('"')?;
    for c in content.escape_default() {
        f.write_char(c)?
    }
    f.write_char('"')
}

fn render_import(import: &str, f: &mut Formatter) -> std::fmt::Result {
    render_indent(1, f)?;
    f.write_str("import ")?;
    render_escaped_string(import, true, f)?;
    f.write_char('\n')
}

fn render_vars(
    vars: &Map<String, Value>,
    indent: usize,
    f: &mut Formatter<'_>,
) -> std::fmt::Result {
    f.write_str("{\n")?;
    for (key, value) in vars {
        render_indent(indent + 1, f)?;
        render_escaped_string(key, false, f)?;
        f.write_str(" = ")?;
        render_value(value, indent + 1, f)?;
        f.write_char('\n')?;
    }
    render_indent(indent, f)?;
    f.write_str("}")
}

fn render_value(val: &Value, indent: usize, f: &mut Formatter<'_>) -> std::fmt::Result {
    match val {
        Value::Null => f.write_str("null"),
        Value::Bool(b) => f.write_fmt(format_args!("{}", b)),
        Value::Number(number) => number.fmt(f),
        Value::String(string) => render_escaped_string(string, true, f),
        Value::Array(array) => {
            f.write_str("[\n")?;
            for (i, value) in array.iter().enumerate() {
                if i != 0 {
                    f.write_str(",\n")?;
                }
                render_indent(indent + 1, f)?;
                render_value(value, indent + 1, f)?;
            }
            f.write_char('\n')?;
            render_indent(indent, f)?;
            f.write_char(']')
        }
        Value::Object(object) => render_vars(object, indent, f),
    }
}

#[cfg(test)]
mod test {
    use super::{ConfigProvider, Host, Service};
    use crate::icinga2::model::check_result::Command;
    use serde_json::{json, Map};

    #[test]
    fn should_render_host_config_with_host_name() {
        let host = Host { name: "my-host".to_string(), ..Host::default() };

        let expected = r#"object Host "my-host" {
}"#;

        let result = format!("{}", host);

        assert_eq!(result, expected)
    }

    #[test]
    fn should_render_host_import() {
        let host = Host {
            name: "my-host".to_string(),
            import: Some("neteye-local-template".to_string()),
            ..Host::default()
        };

        let expected = r#"object Host "my-host" {
    import "neteye-local-template"
}"#;

        let result = format!("{}", host);

        assert_eq!(result, expected)
    }

    #[test]
    fn should_render_host_check_command() {
        let host = Host {
            name: "my-host".to_string(),
            import: Some("neteye-local-template".to_string()),
            check_command: Some(Command::String("hostalive".to_string())),
            ..Host::default()
        };

        let expected = r#"object Host "my-host" {
    import "neteye-local-template"
    check_command = "hostalive"
}"#;

        let result = format!("{}", host);

        assert_eq!(result, expected)
    }

    #[test]
    fn should_render_host_array_command() {
        let host = Host {
            name: "my-host".to_string(),
            import: Some("neteye-local-template".to_string()),
            check_command: Some(Command::Vec(vec![
                "ping".to_string(),
                "127.0.0.1".to_string(),
                "-c".to_string(),
                "10".to_string(),
            ])),
            ..Host::default()
        };

        let expected = r#"object Host "my-host" {
    import "neteye-local-template"
    check_command = ["ping", "127.0.0.1", "-c", "10"]
}"#;

        let result = format!("{}", host);

        assert_eq!(result, expected)
    }

    #[test]
    fn should_render_host_vars() {
        let mut vars = Map::new();
        vars.insert("os".to_string(), json!("linux"));

        let host = Host {
            name: "my-host".to_string(),
            import: Some("neteye-local-template".to_string()),
            check_command: Some(Command::String("hostalive".to_string())),
            vars: Some(vars),
            ..Host::default()
        };

        let expected = r#"object Host "my-host" {
    import "neteye-local-template"
    check_command = "hostalive"
    vars = {
        os = "linux"
    }
}"#;

        let result = format!("{}", host);

        assert_eq!(result, expected)
    }

    #[test]
    fn should_render_host_complex_vars() {
        let mut vars = Map::new();
        vars.insert("os".to_string(), json!("linux"));
        vars.insert(
            "other".to_string(),
            json!({"test_data": ["linux", true, {"name": "anton"}], }),
        );

        let host = Host {
            name: "my-host".to_string(),
            import: Some("neteye-local-template".to_string()),
            check_command: Some(Command::String("hostalive".to_string())),
            vars: Some(vars),
            ..Host::default()
        };

        let expected = r#"object Host "my-host" {
    import "neteye-local-template"
    check_command = "hostalive"
    vars = {
        os = "linux"
        other = {
            test_data = [
                "linux",
                true,
                {
                    name = "anton"
                }
            ]
        }
    }
}"#;

        let result = format!("{}", host);

        assert_eq!(result, expected)
    }

    #[test]
    fn should_render_host_group() {
        let host = Host {
            name: "my-host".to_string(),
            groups: Some(vec!["neteye".to_string(), "uibk".to_string()]),
            ..Host::default()
        };

        let expected = r#"object Host "my-host" {
    groups = ["neteye", "uibk"]
}"#;

        let result = format!("{}", host);

        assert_eq!(result, expected)
    }

    #[test]
    fn should_render_host_whole_config() {
        let mut vars = Map::new();
        vars.insert("os".to_string(), json!({"test_data": ["linux", true, {"name": "anton"}], }));

        let host = Host {
            name: "my-host".to_string(),
            check_command: Some(Command::String("hostalive".to_string())),
            zone: Some("master".to_string()),
            import: Some("neteye-local-template".to_string()),
            address: Some("uibk.ac.at".to_string()),
            address6: Some("::1".to_string()),
            max_check_attempts: Some("3".to_string()),
            check_interval: Some("1s".to_string()),
            retry_interval: Some("30s".to_string()),
            groups: Some(vec!["neteye".to_string(), "uibk".to_string()]),
            vars: Some(vars),
            enable_active_checks: Some(true),
        };

        let expected = r#"object Host "my-host" {
    import "neteye-local-template"
    check_command = "hostalive"
    zone = "master"
    groups = ["neteye", "uibk"]
    address = "uibk.ac.at"
    address6 = "::1"
    max_check_attempts = "3"
    check_interval = "1s"
    retry_interval = "30s"
    enable_active_checks = true
    vars = {
        os = {
            test_data = [
                "linux",
                true,
                {
                    name = "anton"
                }
            ]
        }
    }
}"#;

        let result = format!("{}", host);

        assert_eq!(expected, result)
    }

    #[test]
    fn should_render_host_creation_function() {
        let host = Host {
            name: "my-host".to_string(),
            check_command: Some(Command::String("hostalive".to_string())),
            zone: Some("master".to_string()),
            ..Host::default()
        };

        let result = host.configuration();

        println!("{}", result)
    }

    #[test]
    fn should_render_service() {
        let service = Service { name: "my-service".to_string(), ..Service::default() };

        let expected = r#"object Service "my-service" {
}"#;

        let result = format!("{}", service);

        assert_eq!(result, expected)
    }

    #[test]
    fn should_render_service_whole_config() {
        let mut vars = Map::new();
        vars.insert("os".to_string(), json!("linux"));
        vars.insert(
            "other".to_string(),
            json!({"test_data": ["linux", true, {"name": "anton"}], }),
        );

        let service = Service {
            name: "my-service".to_string(),
            import: Some("neteye-service-template".to_string()),
            host: Some("uibk.ac.at".to_string()),
            check_command: Some(Command::String("hostalive".to_string())),
            command_endpoint: Some("uibk.ac.at".to_string()),
            enable_notifications: Some(true),
            enable_active_checks: Some(false),
            enable_passive_checks: Some(true),
            enable_event_handler: Some(true),
            enable_flapping: Some(false),
            enable_perfdata: Some(true),
            volatile: Some(true),
            vars: Some(vars),
        };

        let expected = r#"object Service "my-service" {
    import "neteye-service-template"
    host = "uibk.ac.at"
    check_command = "hostalive"
    command_endpoint = "uibk.ac.at"
    enable_notifications = true
    enable_active_checks = false
    enable_passive_checks = true
    enable_event_handler = true
    enable_flapping = false
    enable_perfdata = true
    volatile = true
    vars = {
        os = "linux"
        other = {
            test_data = [
                "linux",
                true,
                {
                    name = "anton"
                }
            ]
        }
    }
}"#;

        let result = format!("{}", service);

        assert_eq!(result, expected)
    }
}
