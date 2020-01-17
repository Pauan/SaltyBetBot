use crate::{send_message_result, send_message, spawn, find_first_index, find_last_index};
use serde_derive::{Serialize, Deserialize};
use algorithm::record::{Tier, Mode, Winner, Record};
use wasm_bindgen::JsValue;


// 50 minutes
// TODO is this high enough ?
pub const MAX_MATCH_TIME_LIMIT: f64 = 1000.0 * 60.0 * 50.0;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaifuBetsOpen {
    pub left: String,
    pub right: String,
    pub tier: Tier,
    pub mode: Mode,
    pub date: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaifuBetsClosedInfo {
    pub name: String,
    pub win_streak: f64,
    pub bet_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaifuBetsClosed {
    pub left: WaifuBetsClosedInfo,
    pub right: WaifuBetsClosedInfo,
    pub date: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaifuWinner {
    pub name: String,
    pub side: Winner,
    pub date: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WaifuMessage {
    BetsOpen(WaifuBetsOpen),
    BetsClosed(WaifuBetsClosed),
    Winner(WaifuWinner),
    ModeSwitch { date: f64, is_exhibition: bool },
    ReloadPage,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    RecordsNew,
    RecordsSlice(u32, u32, u32),
    RecordsDrop(u32),

    InsertRecords(Vec<Record>),
    DeleteAllRecords,
    ServerLog(String),
}

const CHUNK_SIZE: u32 = 50_000;

pub async fn records_get_all() -> Result<Vec<Record>, JsValue> {
    let mut records = vec![];

    let mut index = 0;

    let id: u32 = send_message_result(&Message::RecordsNew).await?;

    loop {
        let chunk: Option<Vec<Record>> = send_message(&Message::RecordsSlice(id, index, index + CHUNK_SIZE)).await?;

        if let Some(mut chunk) = chunk {
            records.append(&mut chunk);
            index += CHUNK_SIZE;

        } else {
            break;
        }
    }

    send_message(&Message::RecordsDrop(id)).await?;

    Ok(records)
}

pub async fn records_insert(records: Vec<Record>) -> Result<(), JsValue> {
    // TODO more idiomatic check
    if records.len() > 0 {
        for chunk in records.chunks(CHUNK_SIZE as usize) {
            // TODO can this be made more efficient ?
            send_message_result(&Message::InsertRecords(chunk.into_iter().cloned().collect())).await?;
        }
    }

    Ok(())
}

pub async fn records_delete_all() -> Result<(), JsValue> {
    send_message_result(&Message::DeleteAllRecords).await
}

pub fn server_log(message: String) {
    spawn(send_message(&Message::ServerLog(message)))
}


pub fn sorted_record_index(old_records: &[Record], new_record: &Record) -> Result<(), usize> {
    let start_date = new_record.date - MAX_MATCH_TIME_LIMIT;
    let end_date = new_record.date + MAX_MATCH_TIME_LIMIT;

    let index = find_first_index(&old_records, |x| x.date.partial_cmp(&start_date).unwrap());

    let mut found = false;

    for old_record in &old_records[index..] {
        assert!(old_record.date >= start_date);

        if old_record.date <= end_date {
            if old_record.is_duplicate(&new_record) {
                found = true;
                break;
            }

        } else {
            break;
        }
    }

    if found {
        // TODO return the index of the duplicate ?
        Ok(())

    } else {
        let new_index = find_last_index(&old_records, |x| Record::sort_date(x, &new_record));
        Err(new_index)
    }
}


pub fn get_added_records(mut old_records: Vec<Record>, new_records: Vec<Record>) -> Vec<Record> {
    assert!(old_records.is_sorted_by(|x, y| Some(Record::sort_date(x, y))));

    let mut added_records = vec![];

    // TODO this can be implemented more efficiently (linear rather than quadratic)
    for new_record in new_records {
        if let Err(index) = sorted_record_index(&old_records, &new_record) {
            old_records.insert(index, new_record.clone());
            added_records.push(new_record);
        }
    }

    added_records
}


/// Returns a Vec of the non-duplicate Records, and a Vec of the duplicate Record keys
pub fn partition_records(old_records: Vec<(u32, Record)>) -> (Vec<Record>, Vec<u32>) {
    let mut records = vec![];
    let mut deleted = vec![];

    for (id, record) in old_records {
        match sorted_record_index(&records, &record) {
            Ok(_) => {
                deleted.push(id);
            },
            Err(index) => {
                // TODO figure out a way to avoid this clone
                records.insert(index, record);
            },
        }
    }

    (records, deleted)
}
