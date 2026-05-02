// 自动生成，请勿手动编辑
use std::collections::HashMap;

pub struct StaticFile {
    pub content: &'static [u8],
    pub content_type: &'static str,
}

pub fn load_static_files() -> HashMap<String, StaticFile> {
    let mut files = HashMap::new();

    files.insert(
        "assets/Admin-BuoXVEuq.js".to_string(),
        StaticFile {
            content: include_bytes!("../dist/assets/Admin-BuoXVEuq.js"),
            content_type: "application/javascript",
        },
    );

    files.insert(
        "assets/Admin-CpufGAsY.css".to_string(),
        StaticFile {
            content: include_bytes!("../dist/assets/Admin-CpufGAsY.css"),
            content_type: "text/css",
        },
    );

    files.insert(
        "assets/Home-8TISyg_t.js".to_string(),
        StaticFile {
            content: include_bytes!("../dist/assets/Home-8TISyg_t.js"),
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
        "assets/Login-BAtnm-8X.js".to_string(),
        StaticFile {
            content: include_bytes!("../dist/assets/Login-BAtnm-8X.js"),
            content_type: "application/javascript",
        },
    );

    files.insert(
        "assets/Login-D5pDrefa.css".to_string(),
        StaticFile {
            content: include_bytes!("../dist/assets/Login-D5pDrefa.css"),
            content_type: "text/css",
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
        "assets/Query-Cd7MlGz2.js".to_string(),
        StaticFile {
            content: include_bytes!("../dist/assets/Query-Cd7MlGz2.js"),
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
        "assets/index-j3N0eAQS.js".to_string(),
        StaticFile {
            content: include_bytes!("../dist/assets/index-j3N0eAQS.js"),
            content_type: "application/javascript",
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
