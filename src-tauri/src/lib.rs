use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::Path;

// 文件信息结构体
#[derive(Serialize, Deserialize, Debug)]
pub struct FileInfo {
    name: String,
    content: Vec<u8>,
    is_text: bool,
}

// 解压结果
#[derive(Serialize, Deserialize, Debug)]
pub struct ExtractResult {
    success: bool,
    files: Vec<FileInfo>,
    error: Option<String>,
}

// 问候命令
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// 获取临时文件路径
#[tauri::command]
fn get_temp_path(file_name: String) -> Result<String, String> {
    let temp_dir = std::env::temp_dir();
    let full_path = temp_dir.join(file_name);
    full_path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "无效的路径".to_string())
}

// 解压 ZIP 文件
#[tauri::command]
async fn extract_zip(file_path: String, password: Option<String>) -> Result<ExtractResult, String> {
    let path = Path::new(&file_path);
    
    if !path.exists() {
        return Ok(ExtractResult {
            success: false,
            files: vec![],
            error: Some("文件不存在".to_string()),
        });
    }

    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) => return Ok(ExtractResult {
            success: false,
            files: vec![],
            error: Some(format!("无法打开文件: {}", e)),
        }),
    };

    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(e) => return Ok(ExtractResult {
            success: false,
            files: vec![],
            error: Some(format!("无法读取 ZIP 文件: {}", e)),
        }),
    };

    let mut files = Vec::new();

    for i in 0..archive.len() {
        let mut file = match archive.by_index_decrypt(i, password.as_deref().unwrap_or("").as_bytes()) {
            Ok(f) => f,
            Err(e) => {
                return Ok(ExtractResult {
                    success: false,
                    files: vec![],
                    error: Some(format!("解压失败，可能是密码错误: {}", e)),
                });
            }
        };

        // 跳过目录
        if file.is_dir() {
            continue;
        }

        let name = file.name().to_string();
        let mut content = Vec::new();
        
        if let Err(e) = file.read_to_end(&mut content) {
            return Ok(ExtractResult {
                success: false,
                files: vec![],
                error: Some(format!("读取文件内容失败: {}", e)),
            });
        }

        // 判断是否为文本文件
        let is_text = is_text_file(&name, &content);

        files.push(FileInfo {
            name,
            content,
            is_text,
        });
    }

    Ok(ExtractResult {
        success: true,
        files,
        error: None,
    })
}

// 解压 7z 文件
#[tauri::command]
async fn extract_7z(file_path: String, password: Option<String>) -> Result<ExtractResult, String> {
    let path = Path::new(&file_path);
    
    if !path.exists() {
        return Ok(ExtractResult {
            success: false,
            files: vec![],
            error: Some("文件不存在".to_string()),
        });
    }

    // 创建临时目录
    let temp_dir = std::env::temp_dir().join(format!("tauri_extract_{}", std::process::id()));
    if let Err(e) = fs::create_dir_all(&temp_dir) {
        return Ok(ExtractResult {
            success: false,
            files: vec![],
            error: Some(format!("创建临时目录失败: {}", e)),
        });
    }

    // 使用 sevenz-rust2 解压
    let result = if let Some(pwd) = password {
        // 带密码解压
        sevenz_rust2::decompress_file_with_password(
            path,
            &temp_dir,
            pwd.as_str().into()
        )
    } else {
        // 无密码解压
        sevenz_rust2::decompress_file(path, &temp_dir)
    };

    if let Err(e) = result {
        // 清理临时目录
        let _ = fs::remove_dir_all(&temp_dir);
        return Ok(ExtractResult {
            success: false,
            files: vec![],
            error: Some(format!("解压 7z 失败: {:?}", e)),
        });
    }

    // 读取解压后的文件
    let mut files = Vec::new();
    
    fn read_dir_recursive(dir: &Path, base_path: &Path, files: &mut Vec<FileInfo>) -> Result<(), String> {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            
            if path.is_dir() {
                read_dir_recursive(&path, base_path, files)?;
            } else {
                let content = fs::read(&path).map_err(|e| e.to_string())?;
                let name = path.strip_prefix(base_path)
                    .map_err(|e| e.to_string())?
                    .to_string_lossy()
                    .to_string();
                let is_text = is_text_file(&name, &content);
                
                files.push(FileInfo {
                    name,
                    content,
                    is_text,
                });
            }
        }
        Ok(())
    }

    if let Err(e) = read_dir_recursive(&temp_dir, &temp_dir, &mut files) {
        let _ = fs::remove_dir_all(&temp_dir);
        return Ok(ExtractResult {
            success: false,
            files: vec![],
            error: Some(format!("读取解压文件失败: {}", e)),
        });
    }

    // 清理临时目录
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(ExtractResult {
        success: true,
        files,
        error: None,
    })
}

// 判断是否为文本文件
fn is_text_file(name: &str, content: &[u8]) -> bool {
    // 根据扩展名判断
    let text_extensions = [
        ".txt", ".md", ".json", ".js", ".ts", ".vue", ".html", ".css",
        ".xml", ".yaml", ".yml", ".toml", ".ini", ".conf", ".config",
        ".rs", ".py", ".java", ".c", ".cpp", ".h", ".hpp", ".go",
        ".rb", ".php", ".swift", ".kt", ".scala", ".r", ".m",
        ".sh", ".bash", ".zsh", ".ps1", ".bat", ".cmd",
        ".log", ".csv", ".tsv"
    ];
    
    let lower_name = name.to_lowercase();
    if text_extensions.iter().any(|ext| lower_name.ends_with(ext)) {
        return true;
    }

    // 检查内容是否为文本（简单的启发式检测）
    if content.is_empty() {
        return true;
    }

    // 检查前 1024 字节是否包含空字节（二进制文件通常包含空字节）
    let check_len = content.len().min(1024);
    for i in 0..check_len {
        if content[i] == 0 {
            return false;
        }
    }

    // 检查是否为有效的 UTF-8
    String::from_utf8(content[..check_len].to_vec()).is_ok()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![greet, get_temp_path, extract_zip, extract_7z])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
