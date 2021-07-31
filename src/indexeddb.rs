use super::{closure, WINDOW, MultiSender, poll_receiver, spawn};
use std::pin::Pin;
use std::future::Future;
use std::task::{Poll, Context};
use futures_channel::oneshot;
use web_sys::{IdbKeyRange, IdbDatabase, IdbRequest, IdbVersionChangeEvent, DomException, IdbTransaction, IdbTransactionMode, IdbCursorWithValue, IdbObjectStore, IdbObjectStoreParameters};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;


#[derive(Debug)]
struct Request {
    _on_success: Closure<dyn FnMut(&JsValue)>,
    _on_error: Closure<dyn FnMut(&JsValue)>,
}

impl Request {
    fn new<A, B>(request: &IdbRequest, on_success: A, on_error: B) -> Self
        where A: FnOnce(JsValue) + 'static,
              B: FnOnce(DomException) + 'static {

        let on_success = {
            let request = request.clone();

            Closure::once(move |_event: &JsValue| {
                on_success(request.result().unwrap());
            })
        };

        let on_error = {
            let request = request.clone();

            Closure::once(move |_event: &JsValue| {
                on_error(request.error().unwrap().unwrap());
            })
        };

        // TODO use addEventListener ?
        request.set_onsuccess(Some(on_success.as_ref().unchecked_ref()));

        request.set_onerror(Some(on_error.as_ref().unchecked_ref()));

        Self {
            _on_success: on_success,
            _on_error: on_error,
        }
    }
}


#[derive(Debug)]
struct TransactionFuture {
    receiver: oneshot::Receiver<Result<(), JsValue>>,
    _on_complete: Closure<dyn FnMut(&JsValue)>,
    _on_error: Closure<dyn FnMut(&JsValue)>,
    _on_abort: Closure<dyn FnMut(&JsValue)>,
}

impl TransactionFuture {
    fn new(tx: &IdbTransaction) -> Self {
        let (sender, receiver) = oneshot::channel();

        let sender = MultiSender::new(sender);

        let on_complete = {
            let sender = sender.clone();

            Closure::once(move |_event: &JsValue| {
                sender.send(Ok(()));
            })
        };

        let on_error = {
            let tx = tx.clone();
            let sender = sender.clone();

            Closure::once(move |_event: &JsValue| {
                let error = tx.error().unwrap();

                sender.send(Err(error.into()));
            })
        };

        let on_abort = Closure::once(move |_event: &JsValue| {
            // TODO better error handling
            sender.send(Err(js_sys::Error::new("Transaction aborted").into()));
        });

        // TODO use addEventListener ?
        tx.set_oncomplete(Some(on_complete.as_ref().unchecked_ref()));

        tx.set_onerror(Some(on_error.as_ref().unchecked_ref()));

        tx.set_onabort(Some(on_abort.as_ref().unchecked_ref()));

        Self {
            receiver,
            _on_complete: on_complete,
            _on_error: on_error,
            _on_abort: on_abort,
        }
    }
}

impl Future for TransactionFuture {
    type Output = Result<(), JsValue>;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        poll_receiver(&mut self.receiver, cx)
    }
}


#[derive(Debug)]
struct RequestFuture<A> {
    receiver: oneshot::Receiver<Result<A, JsValue>>,
    _request: Request,
}

impl<A> RequestFuture<A> where A: 'static {
    fn new_raw<F>(
        request: &IdbRequest,
        sender: MultiSender<Result<A, JsValue>>,
        receiver: oneshot::Receiver<Result<A, JsValue>>,
        map: F,
    ) -> Self
        where F: FnOnce(JsValue) -> A + 'static {

        let onsuccess = {
            let sender = sender.clone();

            move |result| {
                sender.send(Ok(map(result)));
            }
        };

        let onerror = move |error: DomException| {
            sender.send(Err(error.into()));
        };

        Self {
            receiver,
            _request: Request::new(&request, onsuccess, onerror),
        }
    }

    fn new<F>(request: &IdbRequest, map: F) -> Self
        where F: FnOnce(JsValue) -> A + 'static {

        let (sender, receiver) = oneshot::channel();

        let sender = MultiSender::new(sender);

        Self::new_raw(request, sender, receiver, map)
    }
}

impl<A> Future for RequestFuture<A> {
    type Output = Result<A, JsValue>;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        poll_receiver(&mut self.receiver, cx)
    }
}


#[derive(Debug)]
struct DbOpen {
    future: RequestFuture<Db>,
    _onupgradeneeded: Closure<dyn FnMut(&IdbVersionChangeEvent)>,
    _onblocked: Closure<dyn FnMut(&JsValue)>,
}

impl Future for DbOpen {
    type Output = Result<Db, JsValue>;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        Pin::new(&mut self.future).poll(cx)
    }
}


#[derive(Debug)]
struct Fold<A> {
    _on_success: Closure<dyn FnMut(&JsValue)>,
    _on_error: Closure<dyn FnMut(&JsValue)>,
    receiver: oneshot::Receiver<Result<A, JsValue>>,
}

impl<A> Future for Fold<A> {
    type Output = Result<A, JsValue>;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        poll_receiver(&mut self.receiver, cx)
    }
}


trait Cursor {
    fn next(&self);
}


#[derive(Debug)]
pub struct ReadCursor {
    cursor: IdbCursorWithValue,
}

impl ReadCursor {
    pub fn key(&self) -> JsValue {
        self.cursor.key().unwrap()
    }

    pub fn value(&self) -> JsValue {
        self.cursor.value().unwrap()
    }
}

impl Cursor for ReadCursor {
    #[inline]
    fn next(&self) {
        self.cursor.continue_().unwrap();
    }
}


#[derive(Debug)]
pub struct WriteCursor {
    cursor: ReadCursor,
}

impl WriteCursor {
    pub fn delete(&self) {
        self.cursor.cursor.delete().unwrap();
    }

    pub fn update(&self, value: &JsValue) {
        self.cursor.cursor.update(value).unwrap();
    }
}

impl std::ops::Deref for WriteCursor {
    type Target = ReadCursor;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.cursor
    }
}

impl Cursor for WriteCursor {
    #[inline]
    fn next(&self) {
        self.cursor.next();
    }
}


#[wasm_bindgen]
extern "C" {
    pub type Record;

    // TODO return Option<JsValue> ?
    #[wasm_bindgen(method, getter)]
    pub fn key(this: &Record) -> JsValue;

    #[wasm_bindgen(method, getter)]
    pub fn value(this: &Record) -> JsValue;
}

impl Record {
    // TODO make this more efficient
    pub fn new(key: Option<&JsValue>, value: &JsValue) -> Self {
        let x = js_sys::Object::new();

        if let Some(key) = key {
            js_sys::Reflect::set(&x, &JsValue::from(wasm_bindgen::intern("key")), key).unwrap();
        }

        js_sys::Reflect::set(&x, &JsValue::from(wasm_bindgen::intern("value")), value).unwrap();

        x.unchecked_into()
    }
}


#[derive(Debug)]
pub struct Read {
    tx: IdbTransaction,
}

impl Read {
    fn store(&self, name: &str) -> IdbObjectStore {
        self.tx.object_store(wasm_bindgen::intern(name)).unwrap()
    }

    fn get_range(&self, name: &str, last_key: Option<&JsValue>, limit: Option<u32>) -> impl Future<Output = Result<Vec<Record>, JsValue>> {
        let start = match last_key {
            // Gets all of the keys that are greater than start
            Some(start) => IdbKeyRange::lower_bound_with_open(start, true).unwrap().into(),
            None => JsValue::UNDEFINED,
        };

        let store = self.store(name);

        let req = match limit {
            Some(limit) => store.get_all_with_key_and_limit(&start, limit).unwrap(),
            None => store.get_all_with_key(&start).unwrap(),
        };

        RequestFuture::new(&req, move |values| {
            let values: js_sys::Array = values.unchecked_into();
            values.iter().map(|value| value.unchecked_into()).collect()
        })
    }

    // TODO return a Stream instead ?
    pub async fn get_all(&self, name: &str) -> Result<Vec<Record>, JsValue> {
        // TODO make this customizable
        const CHUNK_LIMIT: u32 = 50_000;

        let mut output = vec![];

        let mut last_key = None;

        loop {
            let mut values = self.get_range(name, last_key.as_ref(), Some(CHUNK_LIMIT)).await?;

            // TODO is this robust ?
            if values.len() < (CHUNK_LIMIT as usize) {
                output.append(&mut values);
                break;

            } else {
                assert_eq!(values.len(), CHUNK_LIMIT as usize);

                let last = values.last().unwrap();
                last_key = Some(last.key());

                output.append(&mut values);
            }
        }

        Ok(output)
    }

    // TODO improve the monomorphism ?
    fn fold_raw<A, C, F, M>(&self, name: &str, initial: A, mut f: F, mut map: M) -> impl Future<Output = Result<A, JsValue>>
        where A: 'static,
              C: Cursor,
              F: FnMut(A, &C) -> A + 'static,
              M: FnMut(IdbCursorWithValue) -> C + 'static {

        let (sender, receiver) = oneshot::channel();

        let sender = MultiSender::new(sender);

        let request = self.store(name).open_cursor().unwrap();

        let on_success = {
            let sender = sender.clone();
            let request = request.clone();

            let mut initial = Some(initial);

            closure!(move |_event: &JsValue| {
                let cursor = request.result().unwrap();

                let current = initial.take().unwrap();

                if cursor.is_null() {
                    sender.send(Ok(current));

                } else {
                    let cursor = map(cursor.dyn_into().unwrap());
                    initial = Some(f(current, &cursor));
                    cursor.next();
                }
            })
        };

        let on_error = {
            let request = request.clone();

            Closure::once(move |_event: &JsValue| {
                let error = request.error().unwrap().unwrap();
                sender.send(Err(error.into()));
            })
        };

        // TODO use addEventListener ?
        request.set_onsuccess(Some(on_success.as_ref().unchecked_ref()));

        request.set_onerror(Some(on_error.as_ref().unchecked_ref()));

        Fold {
            _on_success: on_success,
            _on_error: on_error,
            receiver,
        }
    }

    // TODO remove this
    pub fn fold<A, F>(&self, name: &str, initial: A, f: F) -> impl Future<Output = Result<A, JsValue>>
        where A: 'static,
              F: FnMut(A, &ReadCursor) -> A + 'static {
        self.fold_raw(name, initial, f, move |cursor| ReadCursor { cursor })
    }
}


#[derive(Debug)]
pub struct Write {
    read: Read,
}

impl Write {
    pub fn insert(&self, name: &str, record: &Record) {
        self.store(name).add(record).unwrap();
    }

    pub fn insert_many<I>(&self, name: &str, values: I) where I: IntoIterator<Item = Record> {
        let store = self.store(name);

        for value in values {
            store.add(&value).unwrap();
        }
    }

    pub fn remove(&self, name: &str, key: &JsValue) {
        self.store(name).delete(key).unwrap();
    }

    pub fn remove_many<I>(&self, name: &str, keys: I) where I: IntoIterator<Item = JsValue> {
        let store = self.store(name);

        for key in keys {
            store.delete(&key).unwrap();
        }
    }

    pub fn clear(&self, name: &str) {
        self.store(name).clear().unwrap();
    }

    // TODO remove this
    pub fn fold<A, F>(&self, name: &str, initial: A, f: F) -> impl Future<Output = Result<A, JsValue>>
        where A: 'static,
              F: FnMut(A, &WriteCursor) -> A + 'static {
        self.fold_raw(name, initial, f, move |cursor| WriteCursor { cursor: ReadCursor { cursor } })
    }
}

impl std::ops::Deref for Write {
    type Target = Read;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.read
    }
}


#[derive(Debug, Clone)]
pub struct TableOptions {
    pub auto_increment: bool,
}


#[derive(Debug)]
pub struct Upgrade {
    db: IdbDatabase,
    write: Write,
}

impl Upgrade {
    pub fn create_table(&self, name: &str, options: &TableOptions) {
        self.db.create_object_store_with_optional_parameters(
            wasm_bindgen::intern(name),
            IdbObjectStoreParameters::new()
                .auto_increment(options.auto_increment)
                .key_path(Some(&JsValue::from(wasm_bindgen::intern("key")))),
        ).unwrap();
    }

    pub fn delete_table(&self, name: &str) {
        // TODO intern this ?
        self.db.delete_object_store(name).unwrap();
    }
}

impl std::ops::Deref for Upgrade {
    type Target = Write;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.write
    }
}


#[derive(Debug)]
pub struct Db {
    db: IdbDatabase,
}

impl Db {
    // TODO this should actually be u64
    // TODO handle versionchange event
    pub fn open<A, F>(name: &str, version: u32, on_upgrade: F) -> impl Future<Output = Result<Self, JsValue>>
        // TODO remove the 'static from A ?
        where A: Future<Output = Result<(), JsValue>> + 'static,
              F: FnOnce(Upgrade, Option<u32>, u32) -> A + 'static {

        let (sender, receiver) = oneshot::channel();

        let sender = MultiSender::new(sender);

        let request = WINDOW.with(|x| x.indexed_db()
            .unwrap()
            .unwrap()
            // TODO should this intern the name ?
            .open_with_u32(wasm_bindgen::intern(name), version)
            .unwrap());

        let onupgradeneeded = {
            let request = request.clone();

            Closure::once(move |event: &IdbVersionChangeEvent| {
                // TODO are these u32 conversions correct ?
                let old_version = event.old_version() as u32;
                let new_version = event.new_version().unwrap() as u32;

                let db = request.result().unwrap().dyn_into().unwrap();

                let tx = request.transaction().unwrap();

                // TODO test that this always works correctly
                let complete = TransactionFuture::new(&tx);

                // TODO test this with oldVersion and newVersion
                let fut = on_upgrade(
                    Upgrade { db, write: Write { read: Read { tx } } },

                    if old_version == 0 {
                        None
                    } else {
                        Some(old_version)
                    },

                    new_version,
                );

                spawn(async move {
                    fut.await?;
                    complete.await?;
                    Ok(())
                });
            })
        };

        let onblocked = {
            let sender = sender.clone();

            Closure::once(move |_event: &JsValue| {
                // TODO better error handling
                sender.send(Err(js_sys::Error::new("Database is blocked").into()));
            })
        };

        request.set_onupgradeneeded(Some(onupgradeneeded.as_ref().unchecked_ref()));

        request.set_onblocked(Some(onblocked.as_ref().unchecked_ref()));

        DbOpen {
            future: RequestFuture::new_raw(&request, sender, receiver, move |result| {
                Self {
                    db: result.dyn_into().unwrap(),
                }
            }),
            _onupgradeneeded: onupgradeneeded,
            _onblocked: onblocked,
        }
    }

    fn transaction(&self, names: &[&str], mode: IdbTransactionMode) -> IdbTransaction {
        // TODO can the names be converted more efficiently ?
        // TODO verify that the names are interned properly when calling JsValue::from
        let names = names.into_iter().map(|x| JsValue::from(wasm_bindgen::intern(*x))).collect::<js_sys::Array>();

        self.db.transaction_with_str_sequence_and_mode(&names, mode).unwrap()
    }

    pub fn read<A, B, F>(&self, names: &[&str], f: F) -> impl Future<Output = Result<A, JsValue>>
        where B: Future<Output = Result<A, JsValue>>,
              F: FnOnce(Read) -> B {

        let tx = self.transaction(names, IdbTransactionMode::Readonly);

        // TODO test that this always works correctly
        let complete = TransactionFuture::new(&tx);

        // TODO should this be inside the async ?
        let fut = f(Read { tx });

        async move {
            let value = fut.await?;
            complete.await?;
            Ok(value)
        }
    }

    pub fn write<A, B, F>(&self, names: &[&str], f: F) -> impl Future<Output = Result<A, JsValue>>
        where B: Future<Output = Result<A, JsValue>>,
              F: FnOnce(Write) -> B {

        let tx = self.transaction(names, IdbTransactionMode::Readwrite);

        // TODO test that this always works correctly
        let complete = TransactionFuture::new(&tx);

        // TODO should this be inside the async ?
        let fut = f(Write { read: Read { tx } });

        async move {
            let value = fut.await?;
            complete.await?;
            Ok(value)
        }
    }
}

impl Drop for Db {
    #[inline]
    fn drop(&mut self) {
        self.db.close();
    }
}
