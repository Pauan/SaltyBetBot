use super::{closure, WINDOW};
use std::rc::Rc;
use std::cell::RefCell;
use std::pin::Pin;
use std::future::Future;
use std::task::{Poll, Context};
use futures_channel::oneshot;
use web_sys::{IdbDatabase, IdbRequest, IdbVersionChangeEvent, DomException, IdbTransaction, IdbTransactionMode, IdbCursorWithValue, IdbObjectStore, IdbObjectStoreParameters};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;


#[derive(Debug)]
struct MultiSender<A> {
    sender: Rc<RefCell<Option<oneshot::Sender<A>>>>,
}

impl<A> MultiSender<A> {
    fn new(sender: oneshot::Sender<A>) -> Self {
        Self {
            sender: Rc::new(RefCell::new(Some(sender))),
        }
    }

    fn send(&self, value: A) {
        let _ = self.sender.borrow_mut()
            .take()
            .unwrap_throw()
            .send(value);
    }
}

impl<A> Clone for MultiSender<A> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}


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
                on_success(request.result().unwrap_throw());
            })
        };

        let on_error = {
            let request = request.clone();

            Closure::once(move |_event: &JsValue| {
                on_error(request.error().unwrap_throw().unwrap_throw());
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
                let error = tx.error().unwrap_throw();

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
        Pin::new(&mut self.receiver).poll(cx).map(|x| {
            // TODO better error handling
            match x {
                Ok(x) => x,
                Err(_) => unreachable!(),
            }
        })
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
        Pin::new(&mut self.receiver).poll(cx).map(|x| {
            // TODO better error handling
            match x {
                Ok(x) => x,
                Err(_) => unreachable!(),
            }
        })
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
struct ForEach {
    _on_success: Closure<dyn FnMut(&JsValue)>,
    _on_error: Closure<dyn FnMut(&JsValue)>,
    receiver: oneshot::Receiver<Result<(), JsValue>>,
}

impl Future for ForEach {
    type Output = Result<(), JsValue>;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        Pin::new(&mut self.receiver).poll(cx).map(|x| {
            // TODO better error handling
            match x {
                Ok(x) => x,
                Err(_) => unreachable!(),
            }
        })
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
        self.cursor.key().unwrap_throw()
    }

    pub fn value(&self) -> JsValue {
        self.cursor.value().unwrap_throw()
    }
}

impl Cursor for ReadCursor {
    #[inline]
    fn next(&self) {
        self.cursor.continue_().unwrap_throw();
    }
}


#[derive(Debug)]
pub struct WriteCursor {
    cursor: ReadCursor,
}

impl WriteCursor {
    pub fn delete(&self) {
        self.cursor.cursor.delete().unwrap_throw();
    }

    pub fn update(&self, value: &JsValue) {
        self.cursor.cursor.update(value).unwrap_throw();
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


#[derive(Debug)]
pub struct Read {
    tx: IdbTransaction,
}

impl Read {
    fn store(&self, name: &str) -> IdbObjectStore {
        self.tx.object_store(wasm_bindgen::intern(name)).unwrap_throw()
    }

    pub fn get_all(&self, name: &str) -> impl Future<Output = Result<js_sys::Array, JsValue>> {
        RequestFuture::new(&self.store(name).get_all().unwrap_throw(), move |values| values.dyn_into().unwrap_throw())
    }

    fn for_each_raw<A, F, M>(&self, name: &str, mut f: F, mut map: M) -> impl Future<Output = Result<(), JsValue>>
        where A: Cursor,
              F: FnMut(&A) + 'static,
              M: FnMut(IdbCursorWithValue) -> A + 'static {

        let (sender, receiver) = oneshot::channel();

        let sender = MultiSender::new(sender);

        let request = self.store(name).open_cursor().unwrap_throw();

        let on_success = {
            let sender = sender.clone();
            let request = request.clone();

            closure!(move |_event: &JsValue| {
                let cursor = request.result().unwrap_throw();

                if cursor.is_undefined() {
                    sender.send(Ok(()));

                } else {
                    let cursor = map(cursor.dyn_into().unwrap_throw());
                    f(&cursor);
                    cursor.next();
                }
            })
        };

        let on_error = {
            let request = request.clone();

            Closure::once(move |_event: &JsValue| {
                let error = request.error().unwrap_throw().unwrap_throw();
                sender.send(Err(error.into()));
            })
        };

        // TODO use addEventListener ?
        request.set_onsuccess(Some(on_success.as_ref().unchecked_ref()));

        request.set_onerror(Some(on_error.as_ref().unchecked_ref()));

        ForEach {
            _on_success: on_success,
            _on_error: on_error,
            receiver,
        }
    }

    pub fn for_each<F>(&self, name: &str, f: F) -> impl Future<Output = Result<(), JsValue>> where F: FnMut(&ReadCursor) + 'static {
        self.for_each_raw(name, f, move |cursor| ReadCursor { cursor })
    }
}


#[derive(Debug)]
pub struct Write {
    read: Read,
}

impl Write {
    pub fn insert(&self, name: &str, value: &JsValue) {
        self.store(name).add(value).unwrap_throw();
    }

    pub fn insert_many<I>(&self, name: &str, values: I) where I: IntoIterator<Item = JsValue> {
        let store = self.store(name);

        for value in values {
            store.add(&value).unwrap_throw();
        }
    }

    pub fn clear(&self, name: &str) {
        self.store(name).clear().unwrap_throw();
    }

    pub fn for_each<F>(&self, name: &str, f: F) -> impl Future<Output = Result<(), JsValue>> where F: FnMut(&WriteCursor) + 'static {
        self.for_each_raw(name, f, move |cursor| WriteCursor { cursor: ReadCursor { cursor } })
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
pub struct TableOptions<'a> {
    pub key_path: Option<&'a str>,
    pub auto_increment: bool,
}


#[derive(Debug)]
pub struct DbUpgrade {
    db: Db,
}

impl DbUpgrade {
    pub fn create_table(&self, name: &str, options: &TableOptions) {
        self.db.db.create_object_store_with_optional_parameters(
            wasm_bindgen::intern(name),
            IdbObjectStoreParameters::new()
                .auto_increment(options.auto_increment)
                // TODO intern this ?
                .key_path(options.key_path.map(JsValue::from).as_ref()),
        ).unwrap_throw();
    }

    pub fn delete_table(&self, name: &str) {
        // TODO intern this ?
        self.db.db.delete_object_store(name).unwrap_throw();
    }
}

impl std::ops::Deref for DbUpgrade {
    type Target = Db;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.db
    }
}


#[derive(Clone, Debug)]
pub struct Db {
    db: IdbDatabase,
}

impl Db {
    // TODO this should actually be u64
    pub fn open<F>(name: &str, version: u32, on_upgrade: F) -> impl Future<Output = Result<Self, JsValue>>
        where F: FnOnce(&DbUpgrade, u32, Option<u32>) + 'static {

        let (sender, receiver) = oneshot::channel();

        let sender = MultiSender::new(sender);

        let request = WINDOW.with(|x| x.indexed_db()
            .unwrap_throw()
            .unwrap_throw()
            // TODO should this intern the name ?
            .open_with_u32(wasm_bindgen::intern(name), version)
            .unwrap_throw());

        let onupgradeneeded = {
            let request = request.clone();

            Closure::once(move |event: &IdbVersionChangeEvent| {
                let db = request.result().unwrap_throw().dyn_into().unwrap_throw();

                // TODO are these u32 conversions correct ?
                // TODO test this with oldVersion and newVersion
                on_upgrade(&DbUpgrade { db: Db { db } }, event.old_version() as u32, event.new_version().map(|x| x as u32));
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
                    db: result.dyn_into().unwrap_throw(),
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

        self.db.transaction_with_str_sequence_and_mode(&names, mode).unwrap_throw()
    }

    pub fn read<A, B, F>(&self, names: &[&str], f: F) -> impl Future<Output = Result<A, JsValue>>
        where B: Future<Output = Result<A, JsValue>>,
              F: FnOnce(Read) -> B {

        let tx = self.transaction(names, IdbTransactionMode::Readonly);

        // TODO test that this always works correctly
        let complete = TransactionFuture::new(&tx);

        async move {
            let value = f(Read { tx }).await?;
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

        async move {
            let value = f(Write { read: Read { tx } }).await?;
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
