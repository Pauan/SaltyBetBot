function add_event(elem, name, f) {
        elem.addEventListener(name, f, {
            capture: false,
            once: false,
            passive: true
        });
    }

    function remove_event(elem, name, f) {
        elem.removeEventListener(name, f, false);
    }

function send_message_raw(message) {
        return new Promise(function (resolve, reject) {
            chrome.runtime.sendMessage(null, message, null, function (x) {
                var error = chrome.runtime.lastError;

                if (error != null) {
                    reject(new Error(error.message));

                } else {
                    resolve(x);
                }
            });
        });
    }

    function chrome_port_connect(name) {
        return chrome.runtime.connect(null, { name: name });
    }

    function get_extension_url(url) {
        return chrome.runtime.getURL(url);
    }

    // TODO add to js_sys
    function decimal(f) {
        return f.toLocaleString("en-US", {
            style: "decimal",
            maximumFractionDigits: 2
        });
    }

let wasm;

const heap = new Array(32).fill(undefined);

heap.push(undefined, null, true, false);

function getObject(idx) { return heap[idx]; }

let heap_next = heap.length;

function dropObject(idx) {
    if (idx < 36) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });

cachedTextDecoder.decode();

let cachegetUint8Memory0 = null;
function getUint8Memory0() {
    if (cachegetUint8Memory0 === null || cachegetUint8Memory0.buffer !== wasm.memory.buffer) {
        cachegetUint8Memory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachegetUint8Memory0;
}

function getStringFromWasm0(ptr, len) {
    return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));
}

let WASM_VECTOR_LEN = 0;

let cachedTextEncoder = new TextEncoder('utf-8');

const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
    ? function (arg, view) {
    return cachedTextEncoder.encodeInto(arg, view);
}
    : function (arg, view) {
    const buf = cachedTextEncoder.encode(arg);
    view.set(buf);
    return {
        read: arg.length,
        written: buf.length
    };
});

function passStringToWasm0(arg, malloc, realloc) {

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length);
        getUint8Memory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len);

    const mem = getUint8Memory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }

    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3);
        const view = getUint8Memory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);

        offset += ret.written;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

let cachegetInt32Memory0 = null;
function getInt32Memory0() {
    if (cachegetInt32Memory0 === null || cachegetInt32Memory0.buffer !== wasm.memory.buffer) {
        cachegetInt32Memory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachegetInt32Memory0;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {
        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            if (--state.cnt === 0) wasm.__wbindgen_export_2.get(dtor)(a, state.b);
            else state.a = a;
        }
    };
    real.original = state;
    return real;
}
function __wbg_adapter_24(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures__invoke1_mut__h16b4e8e694f4fff3(arg0, arg1, addHeapObject(arg2));
}

let stack_pointer = 32;

function addBorrowedObject(obj) {
    if (stack_pointer == 1) throw new Error('out of js stack');
    heap[--stack_pointer] = obj;
    return stack_pointer;
}
function __wbg_adapter_27(arg0, arg1, arg2) {
    try {
        wasm.wasm_bindgen__convert__closures__invoke1_mut_ref__h852ae2abafa10d21(arg0, arg1, addBorrowedObject(arg2));
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

function __wbg_adapter_30(arg0, arg1) {
    wasm.wasm_bindgen__convert__closures__invoke0_mut__h15d2b3e903a521ad(arg0, arg1);
}

function __wbg_adapter_33(arg0, arg1, arg2, arg3) {
    var ptr0 = passStringToWasm0(arg2, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    wasm.wasm_bindgen__convert__closures__invoke2_mut__h97a6b6d0e35ca68b(arg0, arg1, ptr0, len0, addHeapObject(arg3));
}

function getCachedStringFromWasm0(ptr, len) {
    if (ptr === 0) {
        return getObject(len);
    } else {
        return getStringFromWasm0(ptr, len);
    }
}

function handleError(f) {
    return function () {
        try {
            return f.apply(this, arguments);

        } catch (e) {
            wasm.__wbindgen_exn_store(addHeapObject(e));
        }
    };
}

function getArrayU8FromWasm0(ptr, len) {
    return getUint8Memory0().subarray(ptr / 1, ptr / 1 + len);
}

async function load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                if (module.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);

    } else {

        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };

        } else {
            return instance;
        }
    }
}

async function init(input) {
    if (typeof input === 'undefined') {
        input = import.meta.url.replace(/\.js$/, '_bg.wasm');
    }
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbg_sendmessageraw_06db037e50adb455 = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        var ret = send_message_raw(v0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_cb_forget = function(arg0) {
        takeObject(arg0);
    };
    imports.wbg.__wbg_parentNode_3b2b0c389a4d6045 = function(arg0) {
        var ret = getObject(arg0).parentNode;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_instanceof_HtmlElement_8306a9fea71295d9 = function(arg0) {
        var ret = getObject(arg0) instanceof HTMLElement;
        return ret;
    };
    imports.wbg.__wbg_style_a1939f108963de8c = function(arg0) {
        var ret = getObject(arg0).style;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_object_clone_ref = function(arg0) {
        var ret = getObject(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
        takeObject(arg0);
    };
    imports.wbg.__wbg_addevent_e95b1b8bc347987d = function(arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        add_event(getObject(arg0), v0, getObject(arg3));
    };
    imports.wbg.__wbg_new0_210d1cef1e5ccdd4 = function() {
        var ret = new Date();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_toUTCString_26e6acb7f2c1fdfe = function(arg0) {
        var ret = getObject(arg0).toUTCString();
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        var ret = getStringFromWasm0(arg0, arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_log_eb1108411ecc4a7f = function(arg0) {
        console.log(getObject(arg0));
    };
    imports.wbg.__wbg_onMessage_39105b92abbad6eb = function(arg0) {
        var ret = getObject(arg0).onMessage;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_addListener_2ac5cbd510ccd7c6 = function(arg0, arg1) {
        getObject(arg0).addListener(getObject(arg1));
    };
    imports.wbg.__wbg_onDisconnect_1678887fe804d850 = function(arg0) {
        var ret = getObject(arg0).onDisconnect;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_removeListener_f1deaca333139c3d = function(arg0, arg1) {
        getObject(arg0).removeListener(getObject(arg1));
    };
    imports.wbg.__wbindgen_cb_drop = function(arg0) {
        const obj = takeObject(arg0).original;
        if (obj.cnt-- == 1) {
            obj.a = 0;
            return true;
        }
        var ret = false;
        return ret;
    };
    imports.wbg.__wbg_appendChild_3d4ec7dbf3472d31 = handleError(function(arg0, arg1) {
        var ret = getObject(arg0).appendChild(getObject(arg1));
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_getPropertyValue_2549bbdba4668962 = handleError(function(arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg2, arg3);
        var ret = getObject(arg1).getPropertyValue(v0);
        var ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    });
    imports.wbg.__wbg_new_59cb74e423758ede = function() {
        var ret = new Error();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_stack_558ba5917b466edd = function(arg0, arg1) {
        var ret = getObject(arg1).stack;
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    };
    imports.wbg.__wbg_error_4bb6c2a97407129a = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
    if (arg0 !== 0) { wasm.__wbindgen_free(arg0, arg1); }
    console.error(v0);
};
imports.wbg.__wbg_document_c26d0f423c143e0c = function(arg0) {
    var ret = getObject(arg0).document;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};
imports.wbg.__wbg_createElement_44ab59c4ad367831 = handleError(function(arg0, arg1, arg2) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    var ret = getObject(arg0).createElement(v0);
    return addHeapObject(ret);
});
imports.wbg.__wbg_removeProperty_f4c93418b3e220cc = handleError(function(arg0, arg1, arg2, arg3) {
    var v0 = getCachedStringFromWasm0(arg2, arg3);
    var ret = getObject(arg1).removeProperty(v0);
    var ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len1 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len1;
    getInt32Memory0()[arg0 / 4 + 0] = ptr1;
});
imports.wbg.__wbg_cssRules_c038197f43832459 = handleError(function(arg0) {
    var ret = getObject(arg0).cssRules;
    return addHeapObject(ret);
});
imports.wbg.__wbg_length_7bca3575e2dfc21c = function(arg0) {
    var ret = getObject(arg0).length;
    return ret;
};
imports.wbg.__wbg_insertRule_c0373a211f90c126 = handleError(function(arg0, arg1, arg2, arg3) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    var ret = getObject(arg0).insertRule(v0, arg3 >>> 0);
    return ret;
});
imports.wbg.__wbg_get_a860b9763db92fcd = function(arg0, arg1) {
    var ret = getObject(arg0)[arg1 >>> 0];
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};
imports.wbg.__wbg_style_8404af1e6ba2159c = function(arg0) {
    var ret = getObject(arg0).style;
    return addHeapObject(ret);
};
imports.wbg.__wbg_settype_28c925707f3cdada = function(arg0, arg1, arg2) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    getObject(arg0).type = v0;
};
imports.wbg.__wbg_head_2b6d3d5b03819f20 = function(arg0) {
    var ret = getObject(arg0).head;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};
imports.wbg.__wbg_sheet_d24ec2bd414e40ea = function(arg0) {
    var ret = getObject(arg0).sheet;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};
imports.wbg.__wbg_Window_f826a1dec163bacb = function(arg0) {
    var ret = getObject(arg0).Window;
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_is_undefined = function(arg0) {
    var ret = getObject(arg0) === undefined;
    return ret;
};
imports.wbg.__wbg_WorkerGlobalScope_967d186155183d38 = function(arg0) {
    var ret = getObject(arg0).WorkerGlobalScope;
    return addHeapObject(ret);
};
imports.wbg.__wbg_self_c0d3a5923e013647 = handleError(function() {
    var ret = self.self;
    return addHeapObject(ret);
});
imports.wbg.__wbg_window_7ee6c8be3432927d = handleError(function() {
    var ret = window.window;
    return addHeapObject(ret);
});
imports.wbg.__wbg_globalThis_c6de1d938e089cf0 = handleError(function() {
    var ret = globalThis.globalThis;
    return addHeapObject(ret);
});
imports.wbg.__wbg_global_c9a01ce4680907f8 = handleError(function() {
    var ret = global.global;
    return addHeapObject(ret);
});
imports.wbg.__wbg_newnoargs_8aad4a6554f38345 = function(arg0, arg1) {
    var v0 = getCachedStringFromWasm0(arg0, arg1);
    var ret = new Function(v0);
    return addHeapObject(ret);
};
imports.wbg.__wbg_call_1f85aaa5836dfb23 = handleError(function(arg0, arg1) {
    var ret = getObject(arg0).call(getObject(arg1));
    return addHeapObject(ret);
});
imports.wbg.__wbg_new_3a746f2619705add = function(arg0, arg1) {
    var v0 = getCachedStringFromWasm0(arg0, arg1);
    var ret = new Function(v0);
    return addHeapObject(ret);
};
imports.wbg.__wbg_call_f54d3a6dadb199ca = function(arg0, arg1) {
    var ret = getObject(arg0).call(getObject(arg1));
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_jsval_eq = function(arg0, arg1) {
    var ret = getObject(arg0) === getObject(arg1);
    return ret;
};
imports.wbg.__wbg_self_ac379e780a0d8b94 = function(arg0) {
    var ret = getObject(arg0).self;
    return addHeapObject(ret);
};
imports.wbg.__wbg_crypto_1e4302b85d4f64a2 = function(arg0) {
    var ret = getObject(arg0).crypto;
    return addHeapObject(ret);
};
imports.wbg.__wbg_getRandomValues_1b4ba144162a5c9e = function(arg0) {
    var ret = getObject(arg0).getRandomValues;
    return addHeapObject(ret);
};
imports.wbg.__wbg_require_6461b1e9a0d7c34a = function(arg0, arg1) {
    var v0 = getCachedStringFromWasm0(arg0, arg1);
    var ret = require(v0);
    return addHeapObject(ret);
};
imports.wbg.__wbg_getRandomValues_1ef11e888e5228e9 = function(arg0, arg1, arg2) {
    getObject(arg0).getRandomValues(getArrayU8FromWasm0(arg1, arg2));
};
imports.wbg.__wbg_randomFillSync_1b52c8482374c55b = function(arg0, arg1, arg2) {
    getObject(arg0).randomFillSync(getArrayU8FromWasm0(arg1, arg2));
};
imports.wbg.__wbg_new_802fc2e5124bd594 = function(arg0, arg1, arg2, arg3) {
    var v0 = getCachedStringFromWasm0(arg0, arg1);
    var v1 = getCachedStringFromWasm0(arg2, arg3);
    var ret = new RegExp(v0, v1);
    return addHeapObject(ret);
};
imports.wbg.__wbg_textContent_11e88e9f262e569b = function(arg0, arg1) {
    var ret = getObject(arg1).textContent;
    var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};
imports.wbg.__wbg_value_c2fd875fedc14f57 = function(arg0, arg1) {
    var ret = getObject(arg1).value;
    var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};
imports.wbg.__wbg_querySelector_9cf023db23245913 = handleError(function(arg0, arg1, arg2) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    var ret = getObject(arg0).querySelector(v0);
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
});
imports.wbg.__wbg_querySelectorAll_35903dfc85a958c7 = handleError(function(arg0, arg1, arg2) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    var ret = getObject(arg0).querySelectorAll(v0);
    return addHeapObject(ret);
});
imports.wbg.__wbg_clearTimeout_ef4ff11df3d8e727 = function(arg0, arg1) {
    getObject(arg0).clearTimeout(arg1);
};
imports.wbg.__wbg_setTimeout_78ef5ef9cc48797e = handleError(function(arg0, arg1, arg2) {
    var ret = getObject(arg0).setTimeout(getObject(arg1), arg2);
    return ret;
});
imports.wbg.__wbg_location_55774a0e1fed1144 = function(arg0) {
    var ret = getObject(arg0).location;
    return addHeapObject(ret);
};
imports.wbg.__wbg_reload_67452dc14603b156 = handleError(function(arg0) {
    getObject(arg0).reload();
});
imports.wbg.__wbg_getextensionurl_d188f8a1261b87c0 = function(arg0, arg1, arg2) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    var ret = get_extension_url(v0);
    var ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len1 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len1;
    getInt32Memory0()[arg0 / 4 + 0] = ptr1;
};
imports.wbg.__wbg_decimal_38874f68d559be18 = function(arg0, arg1) {
    var ret = decimal(arg1);
    var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};
imports.wbg.__wbg_exec_b4b4b86775b63a1f = function(arg0, arg1, arg2) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    var ret = getObject(arg0).exec(v0);
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};
imports.wbg.__wbg_length_0f0e68fde7e14c19 = function(arg0) {
    var ret = getObject(arg0).length;
    return ret;
};
imports.wbg.__wbg_get_5fd9dd78e47d6ed2 = function(arg0, arg1) {
    var ret = getObject(arg0)[arg1 >>> 0];
    return addHeapObject(ret);
};
imports.wbg.__wbg_setlastindex_75d5a26757dc1ad0 = function(arg0, arg1) {
    getObject(arg0).lastIndex = arg1 >>> 0;
};
imports.wbg.__wbg_replace_35c25a2f8397ee89 = function(arg0, arg1, arg2, arg3) {
    var v0 = getCachedStringFromWasm0(arg2, arg3);
    var ret = getObject(arg0).replace(getObject(arg1), v0);
    return addHeapObject(ret);
};
imports.wbg.__wbg_instanceof_HtmlInputElement_9e9349535b986dc4 = function(arg0) {
    var ret = getObject(arg0) instanceof HTMLInputElement;
    return ret;
};
imports.wbg.__wbg_setvalue_eb5415236467cd34 = function(arg0, arg1, arg2) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    getObject(arg0).value = v0;
};
imports.wbg.__wbg_click_3b9a7794e3ac6e30 = function(arg0) {
    getObject(arg0).click();
};
imports.wbg.__wbg_length_190b91e6ee9bf4c0 = function(arg0) {
    var ret = getObject(arg0).length;
    return ret;
};
imports.wbg.__wbg_get_1bd63197ca654c31 = function(arg0, arg1) {
    var ret = getObject(arg0)[arg1 >>> 0];
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};
imports.wbg.__wbg_querySelector_a57bcececb9a0823 = handleError(function(arg0, arg1, arg2) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    var ret = getObject(arg0).querySelector(v0);
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
});
imports.wbg.__wbg_classList_26cad35d60a907de = function(arg0) {
    var ret = getObject(arg0).classList;
    return addHeapObject(ret);
};
imports.wbg.__wbg_contains_6f90cb529cd138a3 = function(arg0, arg1, arg2) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    var ret = getObject(arg0).contains(v0);
    return ret;
};
imports.wbg.__wbg_add_e43d8149de92004a = handleError(function(arg0, arg1, arg2) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    getObject(arg0).add(v0);
});
imports.wbg.__wbg_removeChild_d8035999cf171601 = handleError(function(arg0, arg1) {
    var ret = getObject(arg0).removeChild(getObject(arg1));
    return addHeapObject(ret);
});
imports.wbg.__wbg_removeevent_d85987e3524619e6 = function(arg0, arg1, arg2, arg3) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    remove_event(getObject(arg0), v0, getObject(arg3));
};
imports.wbg.__wbg_setInterval_b5b6859ec7e84914 = handleError(function(arg0, arg1, arg2) {
    var ret = getObject(arg0).setInterval(getObject(arg1), arg2);
    return ret;
});
imports.wbg.__wbg_setInterval_037ab21c1449ece3 = handleError(function(arg0, arg1, arg2) {
    var ret = getObject(arg0).setInterval(getObject(arg1), arg2);
    return ret;
});
imports.wbg.__wbg_setTimeout_306a4abb69f2ceea = handleError(function(arg0, arg1, arg2) {
    var ret = getObject(arg0).setTimeout(getObject(arg1), arg2);
    return ret;
});
imports.wbg.__wbg_chromeportconnect_5dc6204b52808a38 = function(arg0, arg1) {
    var v0 = getCachedStringFromWasm0(arg0, arg1);
    var ret = chrome_port_connect(v0);
    return addHeapObject(ret);
};
imports.wbg.__wbg_new_1d56e97b8de3067f = function(arg0, arg1) {
    var v0 = getCachedStringFromWasm0(arg0, arg1);
    var ret = new Error(v0);
    return addHeapObject(ret);
};
imports.wbg.__wbg_settextContent_917f10f51a06bd14 = function(arg0, arg1, arg2) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    getObject(arg0).textContent = v0;
};
imports.wbg.__wbg_setAttribute_1e9980589f904db6 = handleError(function(arg0, arg1, arg2, arg3, arg4) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    var v1 = getCachedStringFromWasm0(arg3, arg4);
    getObject(arg0).setAttribute(v0, v1);
});
imports.wbg.__wbindgen_string_get = function(arg0, arg1) {
    const obj = getObject(arg1);
    var ret = typeof(obj) === 'string' ? obj : undefined;
    var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};
imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
    var ret = debugString(getObject(arg1));
    var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};
imports.wbg.__wbindgen_throw = function(arg0, arg1) {
    throw new Error(getStringFromWasm0(arg0, arg1));
};
imports.wbg.__wbindgen_rethrow = function(arg0) {
    throw takeObject(arg0);
};
imports.wbg.__wbg_then_8c23dce80c84c8fb = function(arg0, arg1) {
    var ret = getObject(arg0).then(getObject(arg1));
    return addHeapObject(ret);
};
imports.wbg.__wbg_then_300153bb889a5b4b = function(arg0, arg1, arg2) {
    var ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
    return addHeapObject(ret);
};
imports.wbg.__wbg_resolve_708df7651c8929b8 = function(arg0) {
    var ret = Promise.resolve(getObject(arg0));
    return addHeapObject(ret);
};
imports.wbg.__wbg_instanceof_Window_17fdb5cd280d476d = function(arg0) {
    var ret = getObject(arg0) instanceof Window;
    return ret;
};
imports.wbg.__wbg_setProperty_74426e9bb01e1cae = handleError(function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    var v1 = getCachedStringFromWasm0(arg3, arg4);
    var v2 = getCachedStringFromWasm0(arg5, arg6);
    getObject(arg0).setProperty(v0, v1, v2);
});
imports.wbg.__wbindgen_closure_wrapper1139 = function(arg0, arg1, arg2) {
    var ret = makeMutClosure(arg0, arg1, 45, __wbg_adapter_24);
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_closure_wrapper125 = function(arg0, arg1, arg2) {
    var ret = makeMutClosure(arg0, arg1, 45, __wbg_adapter_27);
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_closure_wrapper109 = function(arg0, arg1, arg2) {
    var ret = makeMutClosure(arg0, arg1, 45, __wbg_adapter_30);
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_closure_wrapper148 = function(arg0, arg1, arg2) {
    var ret = makeMutClosure(arg0, arg1, 45, __wbg_adapter_33);
    return addHeapObject(ret);
};

if (typeof input === 'string' || (typeof Request === 'function' && input instanceof Request) || (typeof URL === 'function' && input instanceof URL)) {
    input = fetch(input);
}

const { instance, module } = await load(await input, imports);

wasm = instance.exports;
init.__wbindgen_wasm_module = module;
wasm.__wbindgen_start();
return wasm;
}

init(chrome.runtime.getURL("js/saltybet.wasm")).catch(console.error);
//# sourceMappingURL=saltybet.js.map
