use crate::model::Project;
use std::{path::Path,io::Write};
pub fn read(path:&Path)->Result<Project,String>{
    let len=std::fs::metadata(path).map_err(|e|e.to_string())?.len();if len>64*1024*1024{return Err("64MB 이하 프로젝트를 열어 주세요.".into())}
    let p:Project=serde_json::from_slice(&std::fs::read(path).map_err(|e|e.to_string())?).map_err(|e|format!("프로젝트 파일을 읽을 수 없어요: {e}"))?;p.validate()?;Ok(p)
}
pub fn save(path:&Path,p:&Project)->Result<(),String>{
    p.validate()?;if let Some(parent)=path.parent(){std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
    let temp=path.with_extension("tmp");let mut f=std::fs::File::create(&temp).map_err(|e|e.to_string())?;
    serde_json::to_writer(&mut f,p).map_err(|e|e.to_string())?;f.flush().map_err(|e|e.to_string())?;f.sync_all().map_err(|e|e.to_string())?;drop(f);
    if std::fs::metadata(&temp).map_err(|e|e.to_string())?.len()>64*1024*1024{return Err("프로젝트가 64MB를 넘어요. 이미지를 줄이거나 프로젝트를 나눠 주세요.".into())}
    if path.exists(){std::fs::copy(path,path.with_extension("backup.json")).map_err(|e|e.to_string())?;}
    std::fs::rename(temp,path).map_err(|e|e.to_string())
}
