use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::Mutex;
use tauri::Manager;

// 共享的登录状态存储
#[derive(Default)]
pub struct SharedAuthState {
    // 存储 cookies，key 是域名，value 是该域名的 cookies
    cookies: Mutex<std::collections::HashMap<String, Vec<CookieData>>>,
    // 存储 localStorage 数据，key 是域名
    local_storage: Mutex<std::collections::HashMap<String, std::collections::HashMap<String, String>>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CookieData {
    name: String,
    value: String,
    domain: String,
    path: String,
}

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

// 打开外部 URL
#[tauri::command]
async fn open_external_url(url: String) -> Result<(), String> {
    // 使用系统命令打开 URL
    std::process::Command::new("cmd")
        .args(["/c", "start", "", &url])
        .spawn()
        .map_err(|e| format!("打开链接失败: {}", e))?;
    Ok(())
}

// 获取 webview 的当前 URL
#[tauri::command]
fn get_webview_url(window: tauri::Window, label: String) -> Result<String, String> {
    // 获取指定 label 的 webview
    let webview = window
        .get_webview(&label)
        .ok_or_else(|| format!("找不到 webview: {}", label))?;
    
    // 获取当前 URL
    let url = webview.url()
        .map_err(|e| format!("获取 URL 失败: {}", e))?;
    Ok(url.to_string())
}

// 保存 cookies 到共享状态
#[tauri::command]
fn save_shared_cookies(
    state: tauri::State<'_, SharedAuthState>,
    domain: String,
    cookies: Vec<CookieData>,
) -> Result<(), String> {
    let mut cookies_map = state.cookies.lock().map_err(|e| e.to_string())?;
    cookies_map.insert(domain, cookies);
    Ok(())
}

// 获取共享的 cookies
#[tauri::command]
fn get_shared_cookies(
    state: tauri::State<'_, SharedAuthState>,
    domain: String,
) -> Result<Vec<CookieData>, String> {
    let cookies_map = state.cookies.lock().map_err(|e| e.to_string())?;
    Ok(cookies_map.get(&domain).cloned().unwrap_or_default())
}

// 保存 localStorage 到共享状态
#[tauri::command]
fn save_shared_local_storage(
    state: tauri::State<'_, SharedAuthState>,
    domain: String,
    data: std::collections::HashMap<String, String>,
) -> Result<(), String> {
    let mut storage_map = state.local_storage.lock().map_err(|e| e.to_string())?;
    storage_map.insert(domain, data);
    Ok(())
}

// 获取共享的 localStorage
#[tauri::command]
fn get_shared_local_storage(
    state: tauri::State<'_, SharedAuthState>,
    domain: String,
) -> Result<std::collections::HashMap<String, String>, String> {
    let storage_map = state.local_storage.lock().map_err(|e| e.to_string())?;
    Ok(storage_map.get(&domain).cloned().unwrap_or_default())
}

// 刷新指定域名的所有 webviews（除了发起请求的 webview），使用初始 URL
#[tauri::command]
async fn refresh_webviews_by_domain(
    window: tauri::Window,
    domain: String,
    exclude_label: String,
    initial_url: String,
) -> Result<(), String> {
    // 获取所有 webviews
    let webviews: Vec<String> = window.webviews()
        .iter()
        .map(|w| w.label().to_string())
        .filter(|label| label != &exclude_label)
        .collect();
    
    for label in webviews {
        if let Some(webview) = window.get_webview(&label) {
            // 获取 webview 的当前 URL
            if let Ok(url) = webview.url() {
                let url_str = url.to_string();
                // 检查 URL 是否包含目标域名
                if url_str.contains(&domain) {
                    // 使用初始 URL 导航，而不是刷新当前页面
                    let js_code = format!("window.location.href = '{}';", initial_url);
                    let _ = webview.eval(&js_code);
                    println!("已刷新 webview: {} -> {}", label, initial_url);
                }
            }
        }
    }
    
    Ok(())
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

// 创建带有新窗口处理器的内嵌 webview（仅桌面端支持）
#[tauri::command]
async fn create_webview_with_handler(
    #[allow(unused_variables)] window: tauri::Window,
    #[allow(unused_variables)] label: String,
    #[allow(unused_variables)] url: String,
    #[allow(unused_variables)] initial_url: String,
    #[allow(unused_variables)] x: f64,
    #[allow(unused_variables)] y: f64,
    #[allow(unused_variables)] width: f64,
    #[allow(unused_variables)] height: f64,
) -> Result<(), String> {
    // 桌面端实现
    #[cfg(desktop)]
    {
        use tauri::WebviewUrl;
        use tauri::webview::WebviewBuilder;

        // 使用 WebviewBuilder 创建内嵌 webview
        let webview_builder = WebviewBuilder::new(
            label.clone(),
            WebviewUrl::External(url.parse().map_err(|e| format!("无效的URL: {}", e))?)
        );

        // 添加 webview 到窗口
        let webview = window
            .add_child(
                webview_builder,
                tauri::LogicalPosition { x, y },
                tauri::LogicalSize { width, height }
            )
            .map_err(|e| format!("创建webview失败: {}", e))?;

        // 注入 JavaScript 来拦截 target="_blank" 链接点击，并同步登录状态
        // 使用 window.__TAURI__.core.invoke 直接调用 Rust 命令
        let js_code = format!(r#"
            (function() {{
                // 保存初始 URL，用于刷新
                var initialUrl = '{}';
                
                // 安装链接拦截器
                function installLinkInterceptor() {{
                    // 移除旧的事件监听器（如果存在）
                    if (window.__linkClickHandler) {{
                        document.removeEventListener('click', window.__linkClickHandler, true);
                    }}
                    
                    // 定义新的事件处理器
                    window.__linkClickHandler = function(e) {{
                        var target = e.target;
                        // 向上查找最近的 A 标签
                        while (target && target.tagName !== 'A') {{
                            target = target.parentElement;
                        }}
                        if (target && target.tagName === 'A') {{
                            var href = target.getAttribute('href');
                            var targetAttr = target.getAttribute('target');
                            // 如果是 target="_blank" 链接
                            if (targetAttr === '_blank' && href) {{
                                e.preventDefault();
                                e.stopPropagation();
                                // 使用 Tauri invoke 直接调用 Rust 命令
                                if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {{
                                    window.__TAURI__.core.invoke('open_external_url', {{ url: href }});
                                }} else {{
                                    // 备用方案：在当前窗口打开
                                    window.location.href = href;
                                }}
                                return false;
                            }}
                        }}
                    }};
                    
                    // 添加事件监听器（捕获阶段）
                    document.addEventListener('click', window.__linkClickHandler, true);
                    console.log('链接拦截器已安装/重新安装');
                }}
                
                // 立即安装
                installLinkInterceptor();
                
                // 监听 DOM 变化，为新添加的链接安装拦截器
                if (!window.__mutationObserver) {{
                    window.__mutationObserver = new MutationObserver(function(mutations) {{
                        // DOM 变化时重新安装，确保新链接也被拦截
                        installLinkInterceptor();
                    }});
                    window.__mutationObserver.observe(document.body, {{
                        childList: true,
                        subtree: true
                    }});
                }}
                
                // 登录状态检测与同步功能
                function setupAuthSync() {{
                    var domain = window.location.hostname;
                    var webviewLabel = window.__TAURI_WEBVIEW_LABEL__ || '';
                    
                    // 存储上一次的登录状态哈希，用于检测变化
                    window.__lastAuthHash = '';
                    
                    // 计算当前登录状态的哈希（简单拼接所有 auth 相关的 key-value）
                    function calcAuthHash() {{
                        var authData = [];
                        for (var i = 0; i < localStorage.length; i++) {{
                            var key = localStorage.key(i);
                            if (key) {{
                                var lowerKey = key.toLowerCase();
                                if (lowerKey.includes('token') || 
                                    lowerKey.includes('auth') || 
                                    lowerKey.includes('session') || 
                                    lowerKey.includes('user') || 
                                    lowerKey.includes('login') ||
                                    lowerKey.includes('credential')) {{
                                    authData.push(key + '=' + localStorage.getItem(key));
                                }}
                            }}
                        }}
                        return authData.sort().join('|');
                    }}
                    
                    // 检测登录状态变化并刷新其他页面
                    function checkAuthChange() {{
                        var currentHash = calcAuthHash();
                        
                        // 如果状态发生变化且不是首次检测
                        if (window.__lastAuthHash && currentHash !== window.__lastAuthHash) {{
                            console.log('检测到登录状态变化，通知 Rust 刷新其他页面');
                            
                            if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {{
                                window.__TAURI__.core.invoke('refresh_webviews_by_domain', {{
                                    domain: domain,
                                    excludeLabel: webviewLabel,
                                    initialUrl: initialUrl
                                }}).catch(function(e) {{
                                    console.log('刷新其他页面失败:', e);
                                }});
                            }}
                        }}
                        
                        window.__lastAuthHash = currentHash;
                    }}
                    
                    // 初始化哈希值
                    window.__lastAuthHash = calcAuthHash();
                    
                    // 定期检测登录状态变化（每 3 秒）
                    setInterval(checkAuthChange, 3000);
                    
                    // 监听 localStorage 变化事件（同一页面内的变化）
                    window.addEventListener('storage', function(e) {{
                        var lowerKey = e.key.toLowerCase();
                        if (lowerKey.includes('token') || 
                            lowerKey.includes('auth') || 
                            lowerKey.includes('session') || 
                            lowerKey.includes('user') || 
                            lowerKey.includes('login') ||
                            lowerKey.includes('credential')) {{
                            console.log('localStorage 变化 detected:', e.key);
                            // 立即检测变化
                            setTimeout(checkAuthChange, 100);
                        }}
                    }});
                }}
                
                // 延迟启动登录状态同步，等待页面加载完成
                setTimeout(setupAuthSync, 2000);
            }})();
        "#, initial_url);

        // 在页面加载完成后注入脚本
        let webview_for_script = webview.clone();
        let js_code_for_periodic = js_code.clone();
        tauri::async_runtime::spawn(async move {
            // 延迟 1 秒后注入脚本，确保页面已加载
            std::thread::sleep(std::time::Duration::from_secs(1));
            let _ = webview_for_script.eval(js_code);
            
            // 每 3 秒重新注入一次，确保动态内容也能被拦截
            let webview_for_periodic = webview_for_script.clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    let _ = webview_for_periodic.eval(js_code_for_periodic.clone());
                }
            });
        });

        Ok(())
    }
    
    // 移动端不支持内嵌 webview，返回错误提示
    #[cfg(not(desktop))]
    {
        Err("移动端暂不支持内嵌 webview 功能".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(SharedAuthState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            greet, get_temp_path, extract_zip, extract_7z, 
            open_external_url, create_webview_with_handler, get_webview_url,
            save_shared_cookies, get_shared_cookies,
            save_shared_local_storage, get_shared_local_storage,
            refresh_webviews_by_domain
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
