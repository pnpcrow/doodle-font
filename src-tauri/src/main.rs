#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use doodle_core::{model::{Project,Template},font::{ExportOptions,ExportResult}};
use tauri::{Manager,State};
use std::{path::PathBuf,sync::Mutex};
struct Storage { root:PathBuf, lock:Mutex<()> }
#[tauri::command]
fn load_project(storage:State<Storage>)->Result<Option<Project>,String>{
    let _guard=storage.lock.lock().map_err(|e|e.to_string())?;
    let path=storage.root.join("project.json");
    if !path.exists(){return Ok(None)}
    let p:Project=serde_json::from_slice(&std::fs::read(path).map_err(|e|e.to_string())?).map_err(|_|"자동 저장 파일을 읽을 수 없어요. 원본을 보존했습니다. 백업 프로젝트를 열어 주세요.")?;
    p.validate()?; Ok(Some(p))
}
#[tauri::command]
fn save_project(project:Project,storage:State<Storage>)->Result<(),String>{
    project.validate()?;
    let _guard=storage.lock.lock().map_err(|e|e.to_string())?;
    std::fs::create_dir_all(&storage.root).map_err(|e|e.to_string())?;
    let path=storage.root.join("project.json"); let temp=storage.root.join("project.tmp");
    let data=serde_json::to_vec(&project).map_err(|e|e.to_string())?;
    if data.len()>64*1024*1024{return Err("프로젝트가 64MB를 넘어요. 프로젝트를 분리해 주세요.".into())}
    use std::io::Write;
    let mut f=std::fs::File::create(&temp).map_err(|e|e.to_string())?;f.write_all(&data).map_err(|e|e.to_string())?;f.sync_all().map_err(|e|e.to_string())?;drop(f);
    if path.exists(){std::fs::copy(&path,storage.root.join("project.backup.json")).map_err(|e|e.to_string())?;}
    std::fs::rename(temp,path).map_err(|e|e.to_string())
}
#[tauri::command]
fn validate_project(project:Project)->Result<Project,String>{ project.validate()?; Ok(project) }
#[tauri::command]
fn validate_template(template:Template)->Result<Template,String>{ template.validate()?; Ok(template) }
#[tauri::command]
async fn open_file()->Result<Option<String>,String>{
    tauri::async_runtime::spawn_blocking(||{
        let Some(file)=rfd::FileDialog::new().add_filter("Doodle Font JSON",&["json"]).pick_file() else{return Ok(None)};
        if std::fs::metadata(&file).map_err(|e|e.to_string())?.len()>64*1024*1024{return Err("64MB 이하 파일을 선택해 주세요.".into())}
        std::fs::read_to_string(file).map(Some).map_err(|e|e.to_string())
    }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn save_file(name:String,data:Vec<u8>)->Result<bool,String>{
    if name.contains(['/', '\\'])||!([".json",".ttf",".zip",".svg"].iter().any(|e|name.ends_with(e)))||data.len()>64*1024*1024{return Err("파일 형식을 확인해 주세요.".into())}
    tauri::async_runtime::spawn_blocking(move||{
        let Some(file)=rfd::FileDialog::new().set_file_name(name).save_file() else{return Ok(false)};
        std::fs::write(file,data).map_err(|e|e.to_string())?;Ok(true)
    }).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn export_font(project:Project,options:ExportOptions)->Result<ExportResult,String>{
    tauri::async_runtime::spawn_blocking(move||doodle_core::font::export(&project,&options)).await.map_err(|e|e.to_string())?
}
#[tauri::command]
async fn generate_candidate(payload:serde_json::Value)->Result<doodle_core::model::Artwork,String>{doodle_core::ai::generate(payload).await}
#[tauri::command]
fn extract_text(text:String)->Result<Vec<String>,String>{
    use unicode_segmentation::UnicodeSegmentation;
    if text.len()>1_000_000{return Err("100만 바이트 이하 문장을 사용해 주세요.".into())}
    let mut seen=std::collections::HashSet::new();Ok(text.graphemes(true).filter(|g|seen.insert(*g)).map(str::to_owned).collect())
}
fn main(){
    tauri::Builder::default().setup(|app|{
        let root=if cfg!(debug_assertions){std::env::var_os("DOODLE_TEST_DIR").map(PathBuf::from).unwrap_or(app.path().app_data_dir()?)}else{app.path().app_data_dir()?};
        app.manage(Storage{root,lock:Mutex::new(())});Ok(())
    }).invoke_handler(tauri::generate_handler![load_project,save_project,validate_project,validate_template,open_file,save_file,export_font,generate_candidate,extract_text]).run(tauri::generate_context!()).expect("Doodle Font failed to start");
}
