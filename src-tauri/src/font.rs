// Native OpenType compiler. No system-font outlines or external executables.
use crate::model::*;
use base64::{Engine,engine::general_purpose::STANDARD};
use serde::{Deserialize,Serialize};
use std::collections::{BTreeMap,HashSet};
#[derive(Default,Deserialize)]
pub struct ExportOptions {pub mode:Option<String>,pub locale:Option<String>,pub group:Option<String>,pub ids:Option<Vec<String>>,pub split:Option<bool>}
#[derive(Serialize)]
pub struct ExportResult {pub filename:String,pub data:String,pub reports:Vec<Report>}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Report {pub family:String,pub glyph_count:usize,pub authored_count:usize,pub tables:Vec<String>,pub mode:String,pub warnings:Vec<String>,pub attribution:String}
pub fn export(_p:&Project,_options:&ExportOptions)->Result<ExportResult,String>{Err("Font compiler initializing".into())}
