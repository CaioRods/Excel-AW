//! Lossless editing checkpoint used only after the receiving editor acknowledges import.
use crate::{DocState, HistoryEntry, Selection, Session, Trust, model::Workbook};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[derive(Serialize, Deserialize)]
pub struct Checkpoint {
    wb: WorkbookState,
    saved: WorkbookState,
    dirty: bool,
    selection: Selection,
    undo: Vec<HistoryState>,
    redo: Vec<HistoryState>,
    path: Option<String>,
    revision: u64,
    uid: u64,
    title: String,
    sheet_selections: Vec<Selection>,
    unlocked_ranges: Vec<(String, String)>,
    content_kind: Option<crate::io::FileKind>,
    scripts_trusted: bool,
    trust: Trust,
    go_to_recent: Vec<String>,
}

// Runtime file preservation is deliberately omitted by the ordinary JSON format. A move must
// retain it, including every undo snapshot, instead of silently stripping unknown file parts.
#[derive(Serialize, Deserialize)]
struct WorkbookState {
    wb: Arc<Workbook>,
    sheets: Vec<SheetState>,
    preserved: Option<Arc<crate::model::preserved::Preserved>>,
    macros_unread: Arc<Vec<serde_json::Value>>,
    source_dir: Option<std::path::PathBuf>,
    source_path: Option<std::path::PathBuf>,
    region: String,
}
#[derive(Serialize, Deserialize)]
struct SheetState {
    id: u64,
    file_id: u32,
    preserved: Option<Arc<crate::model::preserved::Preserved>>,
}
impl WorkbookState {
    fn capture(wb: &Arc<Workbook>) -> Self {
        Self {
            wb: wb.clone(),
            sheets: wb.sheets.iter().map(|s| SheetState { id: s.uid.get(), file_id: s.file_id, preserved: s.preserved.clone() }).collect(),
            preserved: wb.preserved.clone(),
            macros_unread: wb.macros_unread.clone(),
            source_dir: wb.source_dir.clone(),
            source_path: wb.source_path.clone(),
            region: wb.region.tag.into(),
        }
    }
    fn restore(mut self, ids: &mut std::collections::HashMap<u64, crate::model::sheet::SheetUid>) -> Arc<Workbook> {
        let wb = Arc::make_mut(&mut self.wb);
        wb.preserved = self.preserved;
        wb.macros_unread = self.macros_unread;
        wb.source_dir = self.source_dir;
        wb.source_path = self.source_path;
        wb.region = crate::core::region::lookup(&self.region);
        for (sheet, extra) in wb.sheets.iter_mut().zip(self.sheets) {
            let sheet = Arc::make_mut(sheet);
            sheet.uid = *ids.entry(extra.id).or_insert_with(crate::model::sheet::SheetUid::fresh);
            sheet.file_id = extra.file_id;
            sheet.preserved = extra.preserved;
        }
        wb.refresh_equations();
        self.wb
    }
}
#[derive(Serialize, Deserialize)]
struct HistoryState {
    label: String,
    wb: WorkbookState,
    selection: Selection,
    retained_bytes: usize,
}
impl HistoryState {
    fn capture(h: &HistoryEntry) -> Self {
        Self { label: h.label.clone(), wb: WorkbookState::capture(&h.wb), selection: h.selection.clone(), retained_bytes: h.retained_bytes }
    }
    fn restore(self, ids: &mut std::collections::HashMap<u64, crate::model::sheet::SheetUid>) -> HistoryEntry {
        HistoryEntry { label: self.label, wb: self.wb.restore(ids), selection: self.selection, retained_bytes: self.retained_bytes }
    }
}
impl Session {
    pub fn workspace_checkpoint(&self) -> Result<Checkpoint, String> {
        let d = self.active().ok_or("No document")?;
        if self.groups.iter().any(|g| g.doc == d.uid) || self.macro_debug.is_some() || self.recorder.is_some() {
            return Err("Finish the current operation before moving the workbook".into());
        }
        Ok(Checkpoint {
            wb: WorkbookState::capture(&d.wb),
            saved: WorkbookState::capture(&d.saved),
            dirty: d.is_dirty(),
            selection: d.selection.clone(),
            undo: d.undo.iter().map(HistoryState::capture).collect(),
            redo: d.redo.iter().map(HistoryState::capture).collect(),
            path: d.path.clone(),
            revision: d.revision,
            uid: d.uid,
            title: d.title.clone(),
            sheet_selections: d.sheet_selections.clone(),
            unlocked_ranges: d.unlocked_ranges.clone(),
            content_kind: d.content_kind,
            scripts_trusted: d.scripts_trusted,
            trust: d.trust,
            go_to_recent: d.go_to_recent.clone(),
        })
    }
    pub fn workspace_import(&mut self, c: Checkpoint) -> Result<(), String> {
        if self.docs.iter().any(|d| d.is_dirty() || d.path.is_some() || !d.undo.is_empty())
            || self.docs.len() > 1
            || c.uid == 0
            || c.uid >= u64::MAX - 1024
        {
            return Err("Destination is not an empty editor".into());
        }
        while !self.docs.is_empty() {
            self.close_document(0);
        }
        let mut identities = std::collections::HashMap::new();
        let wb = c.wb.restore(&mut identities);
        let saved = c.saved.restore(&mut identities);
        let mut d = DocState::new((*wb).clone(), c.path, c.title, c.trust);
        d.wb = wb;
        d.saved = if c.dirty { saved } else { d.wb.clone() };
        d.selection = c.selection;
        d.undo = c.undo.into_iter().map(|h| h.restore(&mut identities)).collect();
        d.redo = c.redo.into_iter().map(|h| h.restore(&mut identities)).collect();
        d.revision = c.revision;
        d.uid = c.uid;
        d.sheet_selections = c.sheet_selections;
        d.unlocked_ranges = c.unlocked_ranges;
        d.content_kind = c.content_kind;
        d.scripts_trusted = c.scripts_trusted;
        d.go_to_recent = c.go_to_recent;
        for entry in d.undo.iter_mut().chain(d.redo.iter_mut()) {
            if *entry.wb == *d.saved {
                entry.wb = d.saved.clone();
            }
        }
        crate::NEXT_UID.fetch_max(d.uid + 1, std::sync::atomic::Ordering::Relaxed);
        self.add_document(d);
        Ok(())
    }
    pub fn workspace_commit(&mut self, id: u64, revision: u64) -> Result<(), String> {
        let index = self.docs.iter().position(|d| d.uid == id && d.revision == revision).ok_or("The source changed; the original was kept")?;
        if self.docs.len() < 2 {
            return Err("Use live-window docking for the last document".into());
        }
        self.close_document(index);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn aw_selected_checkpoint_keeps_file_parts_and_sheet_identity_through_undo() {
        let mut s = crate::Session::new();
        s.new_workbook();
        let mut preserved = crate::model::preserved::Preserved::default();
        preserved.parts.push(crate::model::preserved::PreservedPart {
            name: "custom/part.xml".into(),
            data: std::sync::Arc::from(&b"synthetic original"[..]),
            ..Default::default()
        });
        {
            let wb = std::sync::Arc::make_mut(&mut s.active_mut().unwrap().wb);
            wb.preserved = Some(std::sync::Arc::new(preserved));
            wb.sheet_mut(0).unwrap().file_id = 7;
        }
        s.execute("cell.set", serde_json::json!({"cell":"A1","input":"edit"})).unwrap();
        let bytes = serde_json::to_vec(&s.workspace_checkpoint().unwrap()).unwrap();
        let mut d = crate::Session::new();
        d.workspace_import(serde_json::from_slice(&bytes).unwrap()).unwrap();
        let uid = d.doc().unwrap().wb.sheet(0).unwrap().uid.get();
        d.execute("edit.undo", serde_json::json!({})).unwrap();
        let wb = &d.doc().unwrap().wb;
        assert_eq!(wb.sheet(0).unwrap().uid.get(), uid);
        assert_eq!(wb.sheet(0).unwrap().file_id, 7);
        assert_eq!(&*wb.preserved.as_ref().unwrap().parts[0].data, b"synthetic original");
    }
    #[test]
    fn aw_selected_checkpoint_rejects_a_changed_source() {
        let mut s = crate::Session::new();
        s.new_workbook();
        s.new_workbook();
        let id = s.doc().unwrap().uid;
        let revision = s.doc().unwrap().revision;
        s.execute("cell.set", serde_json::json!({"cell":"A1","input":"continua na origem"})).unwrap();
        assert!(s.workspace_commit(id, revision).is_err());
        assert_eq!(s.documents().len(), 2);
    }
    #[test]
    fn aw_selected_checkpoint_keeps_other_workbook_and_undo() {
        let mut source = crate::Session::new();
        source.new_workbook();
        source.execute("cell.set", serde_json::json!({"cell":"B2","input":"10"})).unwrap();
        let first = source.doc().unwrap().uid;
        source.new_workbook();
        source.execute("cell.set", serde_json::json!({"cell":"C3","input":"20"})).unwrap();
        let id = source.doc().unwrap().uid;
        let rev = source.doc().unwrap().revision;
        let bytes = serde_json::to_vec(&source.workspace_checkpoint().unwrap()).unwrap();
        let mut destination = crate::Session::new();
        destination.workspace_import(serde_json::from_slice(&bytes).unwrap()).unwrap();
        source.workspace_commit(id, rev).unwrap();
        assert_eq!(source.documents().len(), 1);
        assert_eq!(source.doc().unwrap().uid, first);
        assert_eq!(destination.doc().unwrap().uid, id);
        assert!(destination.doc().unwrap().is_dirty());
        assert_eq!(destination.doc().unwrap().selection.active, crate::core::CellRef::default());
        destination.execute("edit.undo", serde_json::json!({})).unwrap();
        assert!(destination.doc().unwrap().wb.active().unwrap().cells.is_empty());
        assert!(!destination.doc().unwrap().is_dirty());
    }
}
