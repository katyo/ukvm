fn main() {
    #[cfg(feature = "web")]
    {
        use std::{
            env::var,
            fs::{read_dir, File},
            io::Write,
            path::Path,
            process::Command,
        };

        let web_dir = Path::new("web");
        let target_dir = web_dir.join("dist");
        let node_dir = web_dir.join("node_modules");

        let index_file = Path::new("index.html");

        println!(
            "cargo:rerun-if-changed={}",
            web_dir.join("package.json").display()
        );

        println!(
            "cargo:rerun-if-changed={}",
            web_dir.join("package-lock.json").display()
        );

        println!("cargo:rerun-if-changed={}", node_dir.display());

        println!("cargo:rerun-if-changed={}", target_dir.display());

        if !node_dir.is_dir() {
            Command::new("npm")
                .arg("update")
                .current_dir(web_dir)
                .output()
                .expect("Error while npm update");

            Command::new("npm")
                .arg("install")
                .current_dir(web_dir)
                .output()
                .expect("Error while npm install");
        }

        Command::new("npm")
            .arg("run")
            .arg("build")
            .current_dir(web_dir)
            .output()
            .expect("Error while npm run build");

        let src_path = Path::new(&var("OUT_DIR").expect("OUT_DIR is not set")).join("web.rs");
        let mut src = File::create(src_path).expect("Unable to create src file");

        let mut count = 0;

        for ent in read_dir(&target_dir).expect("Unable to read web target directory") {
            let ent = ent.expect("Unable to read directory entry");
            let path = ent.path();

            if path.is_file() {
                let rel_path = path
                    .strip_prefix(&target_dir)
                    .expect("Unable to get relative path");

                let inc_path = path.canonicalize().expect("Unable to get absolute path");

                println!("cargo:rerun-if-changed={}", path.display());

                let ext = rel_path
                    .extension()
                    .expect("Unable to get web file extension")
                    .to_str()
                    .unwrap();

                if count > 0 {
                    write!(src, "    .or(").unwrap();
                } else {
                    write!(src, "    ").unwrap();
                }

                if rel_path == index_file {
                    write!(src, "warp::path::end()").unwrap();
                } else {
                    write!(
                        src,
                        "warp::path({:?}).and(warp::path::end())",
                        rel_path.display()
                    )
                    .unwrap();
                }

                write!(
                    src,
                    ".map(|| warp::http::Response::builder().header(\"content-type\", {:?})",
                    match ext {
                        "html" => "text/html; charset=utf-8",
                        "css" => "text/css",
                        "js" => "text/javascript",
                        "json" | "map" => "application/json",
                        _ => "application/octet-stream",
                    }
                )
                .unwrap();
                write!(src, ".body(include_str!({:?})))", inc_path.display()).unwrap();

                if count > 0 {
                    writeln!(src, ")").unwrap();
                } else {
                    writeln!(src, "").unwrap();
                }

                count += 1;
            }
        }
    }
}
