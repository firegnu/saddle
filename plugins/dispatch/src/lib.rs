//! Optional routing business and resources. Only the binary composition root imports this crate.
mod json;
mod route;
#[cfg(test)]
mod route_tests;
mod rules;
mod transport;

use saddle_core_plugin::{
    Call, Command, Completion, CorePlugin, Manifest, Operation, Resource, ResourceFile,
    ResourceKind, SetupFile,
};

pub struct Dispatch;
pub static PLUGIN: Dispatch = Dispatch;

static MANIFEST: Manifest = Manifest {
    id: "dispatch",
    name: "Dispatch",
    version: env!("CARGO_PKG_VERSION"),
    commands: &[Command {
        name: "route",
        capture: Some(Operation::Route),
    }],
    resources: &[Resource {
        kind: ResourceKind::AgentSkill,
        name: "corral-dispatch",
        revision: 2,
        files: &[
            ResourceFile {
                path: "SKILL.md",
                bytes: include_bytes!("../resources/corral-dispatch/SKILL.md"),
            },
            ResourceFile {
                path: "项目AGENTS模板.md",
                bytes: include_bytes!("../resources/corral-dispatch/项目AGENTS模板.md"),
            },
            ResourceFile {
                path: "遥测操作.md",
                bytes: include_bytes!("../resources/corral-dispatch/遥测操作.md"),
            },
            ResourceFile {
                path: "README.md",
                bytes: include_bytes!("../resources/corral-dispatch/README.md"),
            },
        ],
    }],
    setup_note: "Dispatch 提供路由命令 `saddle plugin run dispatch route`，并随插件安装 corral-dispatch 技能（状态见上）。\n技能装好不会让所有项目都改成主控分派。要让某个项目默认这样做：打开下面的模板，把第1节复制进该项目的 AGENTS.md，替换尖括号里的项目参数，并处理和已有规则的冲突。不采用的项目不用改；某次对话里明确要求走分派流程也可以。\n路由需要环境变量 TYPESAFE_API_KEY，没有时主控自己判断，不影响委派。遥测初始关闭，由 Settings → General 单独控制；路由采集还需显式传入记录上下文。",
    setup_files: &[SetupFile {
        label: "模板",
        resource: "corral-dispatch",
        path: "项目AGENTS模板.md",
    }],
};

impl CorePlugin for Dispatch {
    fn manifest(&self) -> &'static Manifest {
        &MANIFEST
    }
    fn run(&self, call: Call<'_>) -> Completion {
        route::run(
            call,
            std::env::var("TYPESAFE_API_KEY").ok().as_deref(),
            transport::post,
            std::thread::sleep,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct NoBegin;
    impl saddle_core_plugin::Recorder for NoBegin {
        fn begin(&mut self, _: saddle_core_plugin::Begin) {
            panic!("no key must not begin");
        }
    }
    #[test]
    fn absent_key_is_a_business_failure_without_begin() {
        let result = route::run(
            Call {
                command: "route",
                stdin: &mut &b"synthetic"[..],
                recorder: &mut NoBegin,
            },
            None,
            |_, _, _| panic!("no request without key"),
            |_| panic!("no sleep"),
        );
        assert_eq!(result.exit_code, 1);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap(),
            serde_json::json!({"ok":false,"error":"TYPESAFE_API_KEY is not set"})
        );
        assert!(result.end.is_none());
    }
}

#[cfg(test)]
mod resource_tests {
    use super::*;
    use sha2::{Digest, Sha256};
    #[test]
    fn revision_and_original_template_are_pinned_to_bytes() {
        let resource = &PLUGIN.manifest().resources[0];
        let mut list = resource
            .files
            .iter()
            .map(|file| (file.path, format!("{:x}", Sha256::digest(file.bytes))))
            .collect::<Vec<_>>();
        list.sort();
        let fingerprint = format!(
            "sha256:{:x}",
            Sha256::digest(serde_json::to_vec(&list).unwrap())
        );
        assert_eq!(
            (resource.revision, fingerprint.as_str()),
            (
                2,
                "sha256:0696e3c3fdf411be50b9751f2f430e0cff016611d13cb4d631e6dd3241850f9f"
            )
        );
        assert_eq!(
            list.iter()
                .find(|(name, _)| *name == "项目AGENTS模板.md")
                .unwrap()
                .1,
            "22339f4674469d1b8a5041086f4aa3e5a4b5e9d53f356f946dd1a06cc2f73384"
        );
    }
}
