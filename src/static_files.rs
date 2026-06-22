// 自动生成，请勿手动编辑
use std::collections::HashMap;

pub struct StaticFile {
    pub content: &'static [u8],
    pub content_type: &'static str,
}

pub fn load_static_files() -> HashMap<String, StaticFile> {
    let mut files = HashMap::new();

    files.insert(
        "assets/Admin-DzhYJw7X.css".to_string(),
        StaticFile {
            content: include_bytes!("../dist/assets/Admin-DzhYJw7X.css"),
            content_type: "text/css",
        },
    );

    files.insert(
        "assets/Admin-fJmKOJWC.js".to_string(),
        StaticFile {
            content: include_bytes!("../dist/assets/Admin-fJmKOJWC.js"),
            content_type: "application/javascript",
        },
    );

    files.insert(
        "assets/Home-C9OMJCz1.css".to_string(),
        StaticFile {
            content: include_bytes!("../dist/assets/Home-C9OMJCz1.css"),
            content_type: "text/css",
        },
    );

    files.insert(
        "assets/Home-DAEGUPt4.js".to_string(),
        StaticFile {
            content: include_bytes!("../dist/assets/Home-DAEGUPt4.js"),
            content_type: "application/javascript",
        },
    );

    files.insert(
        "assets/Login-Crm__5gx.css".to_string(),
        StaticFile {
            content: include_bytes!("../dist/assets/Login-Crm__5gx.css"),
            content_type: "text/css",
        },
    );

    files.insert(
        "assets/Login-gUwaKPNG.js".to_string(),
        StaticFile {
            content: include_bytes!("../dist/assets/Login-gUwaKPNG.js"),
            content_type: "application/javascript",
        },
    );

    files.insert(
        "assets/Query-C3tPn5lr.css".to_string(),
        StaticFile {
            content: include_bytes!("../dist/assets/Query-C3tPn5lr.css"),
            content_type: "text/css",
        },
    );

    files.insert(
        "assets/Query-cmhHCF0P.js".to_string(),
        StaticFile {
            content: include_bytes!("../dist/assets/Query-cmhHCF0P.js"),
            content_type: "application/javascript",
        },
    );

    files.insert(
        "assets/_plugin-vue_export-helper-DlAUqK2U.js".to_string(),
        StaticFile {
            content: include_bytes!("../dist/assets/_plugin-vue_export-helper-DlAUqK2U.js"),
            content_type: "application/javascript",
        },
    );

    files.insert(
        "assets/index-CGE6agbe.js".to_string(),
        StaticFile {
            content: include_bytes!("../dist/assets/index-CGE6agbe.js"),
            content_type: "application/javascript",
        },
    );

    files.insert(
        "assets/index-DZ_Kha3d.js".to_string(),
        StaticFile {
            content: include_bytes!("../dist/assets/index-DZ_Kha3d.js"),
            content_type: "application/javascript",
        },
    );

    files.insert(
        "assets/index-Shk2lMNe.css".to_string(),
        StaticFile {
            content: include_bytes!("../dist/assets/index-Shk2lMNe.css"),
            content_type: "text/css",
        },
    );

    files.insert(
        "index.html".to_string(),
        StaticFile {
            content: include_bytes!("../dist/index.html"),
            content_type: "text/html; charset=utf-8",
        },
    );

    files
}
