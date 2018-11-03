"use strict";

if( typeof Rust === "undefined" ) {
    var Rust = {};
}

(function( root, factory ) {
    if( typeof define === "function" && define.amd ) {
        define( [], factory );
    } else if( typeof module === "object" && module.exports ) {
        module.exports = factory();
    } else {
        Rust.chart = factory();
    }
}( this, function() {
    return (function( module_factory ) {
        var instance = module_factory();

        if( typeof window === "undefined" && typeof process === "object" ) {
            var fs = require( "fs" );
            var path = require( "path" );
            var wasm_path = path.join( __dirname, "chart.wasm" );
            var buffer = fs.readFileSync( wasm_path );
            var mod = new WebAssembly.Module( buffer );
            var wasm_instance = new WebAssembly.Instance( mod, instance.imports );
            return instance.initialize( wasm_instance );
        } else {
            var file = fetch(chrome.runtime.getURL("chart.wasm"), {credentials: "same-origin"} );

            var wasm_instance = ( typeof WebAssembly.instantiateStreaming === "function"
                ? WebAssembly.instantiateStreaming( file, instance.imports )
                    .then( function( result ) { return result.instance; } )

                : file
                    .then( function( response ) { return response.arrayBuffer(); } )
                    .then( function( bytes ) { return WebAssembly.compile( bytes ); } )
                    .then( function( mod ) { return WebAssembly.instantiate( mod, instance.imports ) } ) );

            return wasm_instance
                .then( function( wasm_instance ) {
                    var exports = instance.initialize( wasm_instance );
                    console.log( "Finished loading Rust wasm module 'chart'" );
                    return exports;
                })
                .catch( function( error ) {
                    console.log( "Error loading Rust wasm module 'chart':", error );
                    throw error;
                });
        }
    }( function() {
    var Module = {};

    Module.STDWEB_PRIVATE = {};

// This is based on code from Emscripten's preamble.js.
Module.STDWEB_PRIVATE.to_utf8 = function to_utf8( str, addr ) {
    for( var i = 0; i < str.length; ++i ) {
        // Gotcha: charCodeAt returns a 16-bit word that is a UTF-16 encoded code unit, not a Unicode code point of the character! So decode UTF16->UTF32->UTF8.
        // See http://unicode.org/faq/utf_bom.html#utf16-3
        // For UTF8 byte structure, see http://en.wikipedia.org/wiki/UTF-8#Description and https://www.ietf.org/rfc/rfc2279.txt and https://tools.ietf.org/html/rfc3629
        var u = str.charCodeAt( i ); // possibly a lead surrogate
        if( u >= 0xD800 && u <= 0xDFFF ) {
            u = 0x10000 + ((u & 0x3FF) << 10) | (str.charCodeAt( ++i ) & 0x3FF);
        }

        if( u <= 0x7F ) {
            HEAPU8[ addr++ ] = u;
        } else if( u <= 0x7FF ) {
            HEAPU8[ addr++ ] = 0xC0 | (u >> 6);
            HEAPU8[ addr++ ] = 0x80 | (u & 63);
        } else if( u <= 0xFFFF ) {
            HEAPU8[ addr++ ] = 0xE0 | (u >> 12);
            HEAPU8[ addr++ ] = 0x80 | ((u >> 6) & 63);
            HEAPU8[ addr++ ] = 0x80 | (u & 63);
        } else if( u <= 0x1FFFFF ) {
            HEAPU8[ addr++ ] = 0xF0 | (u >> 18);
            HEAPU8[ addr++ ] = 0x80 | ((u >> 12) & 63);
            HEAPU8[ addr++ ] = 0x80 | ((u >> 6) & 63);
            HEAPU8[ addr++ ] = 0x80 | (u & 63);
        } else if( u <= 0x3FFFFFF ) {
            HEAPU8[ addr++ ] = 0xF8 | (u >> 24);
            HEAPU8[ addr++ ] = 0x80 | ((u >> 18) & 63);
            HEAPU8[ addr++ ] = 0x80 | ((u >> 12) & 63);
            HEAPU8[ addr++ ] = 0x80 | ((u >> 6) & 63);
            HEAPU8[ addr++ ] = 0x80 | (u & 63);
        } else {
            HEAPU8[ addr++ ] = 0xFC | (u >> 30);
            HEAPU8[ addr++ ] = 0x80 | ((u >> 24) & 63);
            HEAPU8[ addr++ ] = 0x80 | ((u >> 18) & 63);
            HEAPU8[ addr++ ] = 0x80 | ((u >> 12) & 63);
            HEAPU8[ addr++ ] = 0x80 | ((u >> 6) & 63);
            HEAPU8[ addr++ ] = 0x80 | (u & 63);
        }
    }
};

Module.STDWEB_PRIVATE.noop = function() {};
Module.STDWEB_PRIVATE.to_js = function to_js( address ) {
    var kind = HEAPU8[ address + 12 ];
    if( kind === 0 ) {
        return undefined;
    } else if( kind === 1 ) {
        return null;
    } else if( kind === 2 ) {
        return HEAP32[ address / 4 ];
    } else if( kind === 3 ) {
        return HEAPF64[ address / 8 ];
    } else if( kind === 4 ) {
        var pointer = HEAPU32[ address / 4 ];
        var length = HEAPU32[ (address + 4) / 4 ];
        return Module.STDWEB_PRIVATE.to_js_string( pointer, length );
    } else if( kind === 5 ) {
        return false;
    } else if( kind === 6 ) {
        return true;
    } else if( kind === 7 ) {
        var pointer = Module.STDWEB_PRIVATE.arena + HEAPU32[ address / 4 ];
        var length = HEAPU32[ (address + 4) / 4 ];
        var output = [];
        for( var i = 0; i < length; ++i ) {
            output.push( Module.STDWEB_PRIVATE.to_js( pointer + i * 16 ) );
        }
        return output;
    } else if( kind === 8 ) {
        var arena = Module.STDWEB_PRIVATE.arena;
        var value_array_pointer = arena + HEAPU32[ address / 4 ];
        var length = HEAPU32[ (address + 4) / 4 ];
        var key_array_pointer = arena + HEAPU32[ (address + 8) / 4 ];
        var output = {};
        for( var i = 0; i < length; ++i ) {
            var key_pointer = HEAPU32[ (key_array_pointer + i * 8) / 4 ];
            var key_length = HEAPU32[ (key_array_pointer + 4 + i * 8) / 4 ];
            var key = Module.STDWEB_PRIVATE.to_js_string( key_pointer, key_length );
            var value = Module.STDWEB_PRIVATE.to_js( value_array_pointer + i * 16 );
            output[ key ] = value;
        }
        return output;
    } else if( kind === 9 ) {
        return Module.STDWEB_PRIVATE.acquire_js_reference( HEAP32[ address / 4 ] );
    } else if( kind === 10 || kind === 12 || kind === 13 ) {
        var adapter_pointer = HEAPU32[ address / 4 ];
        var pointer = HEAPU32[ (address + 4) / 4 ];
        var deallocator_pointer = HEAPU32[ (address + 8) / 4 ];
        var num_ongoing_calls = 0;
        var drop_queued = false;
        var output = function() {
            if( pointer === 0 || drop_queued === true ) {
                if (kind === 10) {
                    throw new ReferenceError( "Already dropped Rust function called!" );
                } else if (kind === 12) {
                    throw new ReferenceError( "Already dropped FnMut function called!" );
                } else {
                    throw new ReferenceError( "Already called or dropped FnOnce function called!" );
                }
            }

            var function_pointer = pointer;
            if (kind === 13) {
                output.drop = Module.STDWEB_PRIVATE.noop;
                pointer = 0;
            }

            if (num_ongoing_calls !== 0) {
                if (kind === 12 || kind === 13) {
                    throw new ReferenceError( "FnMut function called multiple times concurrently!" );
                }
            }

            var args = Module.STDWEB_PRIVATE.alloc( 16 );
            Module.STDWEB_PRIVATE.serialize_array( args, arguments );

            try {
                num_ongoing_calls += 1;
                Module.STDWEB_PRIVATE.dyncall( "vii", adapter_pointer, [function_pointer, args] );
                var result = Module.STDWEB_PRIVATE.tmp;
                Module.STDWEB_PRIVATE.tmp = null;
            } finally {
                num_ongoing_calls -= 1;
            }

            if( drop_queued === true && num_ongoing_calls === 0 ) {
                output.drop();
            }

            return result;
        };

        output.drop = function() {
            if (num_ongoing_calls !== 0) {
                drop_queued = true;
                return;
            }

            output.drop = Module.STDWEB_PRIVATE.noop;
            var function_pointer = pointer;
            pointer = 0;

            if (function_pointer != 0) {
                Module.STDWEB_PRIVATE.dyncall( "vi", deallocator_pointer, [function_pointer] );
            }
        };

        return output;
    } else if( kind === 14 ) {
        var pointer = HEAPU32[ address / 4 ];
        var length = HEAPU32[ (address + 4) / 4 ];
        var array_kind = HEAPU32[ (address + 8) / 4 ];
        var pointer_end = pointer + length;

        switch( array_kind ) {
            case 0:
                return HEAPU8.subarray( pointer, pointer_end );
            case 1:
                return HEAP8.subarray( pointer, pointer_end );
            case 2:
                return HEAPU16.subarray( pointer, pointer_end );
            case 3:
                return HEAP16.subarray( pointer, pointer_end );
            case 4:
                return HEAPU32.subarray( pointer, pointer_end );
            case 5:
                return HEAP32.subarray( pointer, pointer_end );
            case 6:
                return HEAPF32.subarray( pointer, pointer_end );
            case 7:
                return HEAPF64.subarray( pointer, pointer_end );
        }
    } else if( kind === 15 ) {
        return Module.STDWEB_PRIVATE.get_raw_value( HEAPU32[ address / 4 ] );
    }
};

Module.STDWEB_PRIVATE.serialize_object = function serialize_object( address, value ) {
    var keys = Object.keys( value );
    var length = keys.length;
    var key_array_pointer = Module.STDWEB_PRIVATE.alloc( length * 8 );
    var value_array_pointer = Module.STDWEB_PRIVATE.alloc( length * 16 );
    HEAPU8[ address + 12 ] = 8;
    HEAPU32[ address / 4 ] = value_array_pointer;
    HEAPU32[ (address + 4) / 4 ] = length;
    HEAPU32[ (address + 8) / 4 ] = key_array_pointer;
    for( var i = 0; i < length; ++i ) {
        var key = keys[ i ];
        var key_length = Module.STDWEB_PRIVATE.utf8_len( key );
        var key_pointer = Module.STDWEB_PRIVATE.alloc( key_length );
        Module.STDWEB_PRIVATE.to_utf8( key, key_pointer );

        var key_address = key_array_pointer + i * 8;
        HEAPU32[ key_address / 4 ] = key_pointer;
        HEAPU32[ (key_address + 4) / 4 ] = key_length;

        Module.STDWEB_PRIVATE.from_js( value_array_pointer + i * 16, value[ key ] );
    }
};

Module.STDWEB_PRIVATE.serialize_array = function serialize_array( address, value ) {
    var length = value.length;
    var pointer = Module.STDWEB_PRIVATE.alloc( length * 16 );
    HEAPU8[ address + 12 ] = 7;
    HEAPU32[ address / 4 ] = pointer;
    HEAPU32[ (address + 4) / 4 ] = length;
    for( var i = 0; i < length; ++i ) {
        Module.STDWEB_PRIVATE.from_js( pointer + i * 16, value[ i ] );
    }
};

Module.STDWEB_PRIVATE.from_js = function from_js( address, value ) {
    var kind = Object.prototype.toString.call( value );
    if( kind === "[object String]" ) {
        var length = Module.STDWEB_PRIVATE.utf8_len( value );
        var pointer = 0;
        if( length > 0 ) {
            pointer = Module.STDWEB_PRIVATE.alloc( length );
            Module.STDWEB_PRIVATE.to_utf8( value, pointer );
        }
        HEAPU8[ address + 12 ] = 4;
        HEAPU32[ address / 4 ] = pointer;
        HEAPU32[ (address + 4) / 4 ] = length;
    } else if( kind === "[object Number]" ) {
        if( value === (value|0) ) {
            HEAPU8[ address + 12 ] = 2;
            HEAP32[ address / 4 ] = value;
        } else {
            HEAPU8[ address + 12 ] = 3;
            HEAPF64[ address / 8 ] = value;
        }
    } else if( value === null ) {
        HEAPU8[ address + 12 ] = 1;
    } else if( value === undefined ) {
        HEAPU8[ address + 12 ] = 0;
    } else if( value === false ) {
        HEAPU8[ address + 12 ] = 5;
    } else if( value === true ) {
        HEAPU8[ address + 12 ] = 6;
    } else if( kind === "[object Symbol]" ) {
        var id = Module.STDWEB_PRIVATE.register_raw_value( value );
        HEAPU8[ address + 12 ] = 15;
        HEAP32[ address / 4 ] = id;
    } else {
        var refid = Module.STDWEB_PRIVATE.acquire_rust_reference( value );
        HEAPU8[ address + 12 ] = 9;
        HEAP32[ address / 4 ] = refid;
    }
};

// This is ported from Rust's stdlib; it's faster than
// the string conversion from Emscripten.
Module.STDWEB_PRIVATE.to_js_string = function to_js_string( index, length ) {
    index = index|0;
    length = length|0;
    var end = (index|0) + (length|0);
    var output = "";
    while( index < end ) {
        var x = HEAPU8[ index++ ];
        if( x < 128 ) {
            output += String.fromCharCode( x );
            continue;
        }
        var init = (x & (0x7F >> 2));
        var y = 0;
        if( index < end ) {
            y = HEAPU8[ index++ ];
        }
        var ch = (init << 6) | (y & 63);
        if( x >= 0xE0 ) {
            var z = 0;
            if( index < end ) {
                z = HEAPU8[ index++ ];
            }
            var y_z = ((y & 63) << 6) | (z & 63);
            ch = init << 12 | y_z;
            if( x >= 0xF0 ) {
                var w = 0;
                if( index < end ) {
                    w = HEAPU8[ index++ ];
                }
                ch = (init & 7) << 18 | ((y_z << 6) | (w & 63));

                output += String.fromCharCode( 0xD7C0 + (ch >> 10) );
                ch = 0xDC00 + (ch & 0x3FF);
            }
        }
        output += String.fromCharCode( ch );
        continue;
    }
    return output;
};

Module.STDWEB_PRIVATE.id_to_ref_map = {};
Module.STDWEB_PRIVATE.id_to_refcount_map = {};
Module.STDWEB_PRIVATE.ref_to_id_map = new WeakMap();
// Not all types can be stored in a WeakMap
Module.STDWEB_PRIVATE.ref_to_id_map_fallback = new Map();
Module.STDWEB_PRIVATE.last_refid = 1;

Module.STDWEB_PRIVATE.id_to_raw_value_map = {};
Module.STDWEB_PRIVATE.last_raw_value_id = 1;

Module.STDWEB_PRIVATE.acquire_rust_reference = function( reference ) {
    if( reference === undefined || reference === null ) {
        return 0;
    }

    var id_to_refcount_map = Module.STDWEB_PRIVATE.id_to_refcount_map;
    var id_to_ref_map = Module.STDWEB_PRIVATE.id_to_ref_map;
    var ref_to_id_map = Module.STDWEB_PRIVATE.ref_to_id_map;
    var ref_to_id_map_fallback = Module.STDWEB_PRIVATE.ref_to_id_map_fallback;

    var refid = ref_to_id_map.get( reference );
    if( refid === undefined ) {
        refid = ref_to_id_map_fallback.get( reference );
    }
    if( refid === undefined ) {
        refid = Module.STDWEB_PRIVATE.last_refid++;
        try {
            ref_to_id_map.set( reference, refid );
        } catch (e) {
            ref_to_id_map_fallback.set( reference, refid );
        }
    }

    if( refid in id_to_ref_map ) {
        id_to_refcount_map[ refid ]++;
    } else {
        id_to_ref_map[ refid ] = reference;
        id_to_refcount_map[ refid ] = 1;
    }

    return refid;
};

Module.STDWEB_PRIVATE.acquire_js_reference = function( refid ) {
    return Module.STDWEB_PRIVATE.id_to_ref_map[ refid ];
};

Module.STDWEB_PRIVATE.increment_refcount = function( refid ) {
    Module.STDWEB_PRIVATE.id_to_refcount_map[ refid ]++;
};

Module.STDWEB_PRIVATE.decrement_refcount = function( refid ) {
    var id_to_refcount_map = Module.STDWEB_PRIVATE.id_to_refcount_map;
    if( 0 == --id_to_refcount_map[ refid ] ) {
        var id_to_ref_map = Module.STDWEB_PRIVATE.id_to_ref_map;
        var ref_to_id_map_fallback = Module.STDWEB_PRIVATE.ref_to_id_map_fallback;
        var reference = id_to_ref_map[ refid ];
        delete id_to_ref_map[ refid ];
        delete id_to_refcount_map[ refid ];
        ref_to_id_map_fallback.delete(reference);
    }
};

Module.STDWEB_PRIVATE.register_raw_value = function( value ) {
    var id = Module.STDWEB_PRIVATE.last_raw_value_id++;
    Module.STDWEB_PRIVATE.id_to_raw_value_map[ id ] = value;
    return id;
};

Module.STDWEB_PRIVATE.unregister_raw_value = function( id ) {
    delete Module.STDWEB_PRIVATE.id_to_raw_value_map[ id ];
};

Module.STDWEB_PRIVATE.get_raw_value = function( id ) {
    return Module.STDWEB_PRIVATE.id_to_raw_value_map[ id ];
};

Module.STDWEB_PRIVATE.alloc = function alloc( size ) {
    return Module.web_malloc( size );
};

Module.STDWEB_PRIVATE.dyncall = function( signature, ptr, args ) {
    return Module.web_table.get( ptr ).apply( null, args );
};

// This is based on code from Emscripten's preamble.js.
Module.STDWEB_PRIVATE.utf8_len = function utf8_len( str ) {
    var len = 0;
    for( var i = 0; i < str.length; ++i ) {
        // Gotcha: charCodeAt returns a 16-bit word that is a UTF-16 encoded code unit, not a Unicode code point of the character! So decode UTF16->UTF32->UTF8.
        // See http://unicode.org/faq/utf_bom.html#utf16-3
        var u = str.charCodeAt( i ); // possibly a lead surrogate
        if( u >= 0xD800 && u <= 0xDFFF ) {
            u = 0x10000 + ((u & 0x3FF) << 10) | (str.charCodeAt( ++i ) & 0x3FF);
        }

        if( u <= 0x7F ) {
            ++len;
        } else if( u <= 0x7FF ) {
            len += 2;
        } else if( u <= 0xFFFF ) {
            len += 3;
        } else if( u <= 0x1FFFFF ) {
            len += 4;
        } else if( u <= 0x3FFFFFF ) {
            len += 5;
        } else {
            len += 6;
        }
    }
    return len;
};

Module.STDWEB_PRIVATE.prepare_any_arg = function( value ) {
    var arg = Module.STDWEB_PRIVATE.alloc( 16 );
    Module.STDWEB_PRIVATE.from_js( arg, value );
    return arg;
};

Module.STDWEB_PRIVATE.acquire_tmp = function( dummy ) {
    var value = Module.STDWEB_PRIVATE.tmp;
    Module.STDWEB_PRIVATE.tmp = null;
    return value;
};



    var HEAP8 = null;
    var HEAP16 = null;
    var HEAP32 = null;
    var HEAPU8 = null;
    var HEAPU16 = null;
    var HEAPU32 = null;
    var HEAPF32 = null;
    var HEAPF64 = null;

    Object.defineProperty( Module, 'exports', { value: {} } );

    function __web_on_grow() {
        var buffer = Module.instance.exports.memory.buffer;
        HEAP8 = new Int8Array( buffer );
        HEAP16 = new Int16Array( buffer );
        HEAP32 = new Int32Array( buffer );
        HEAPU8 = new Uint8Array( buffer );
        HEAPU16 = new Uint16Array( buffer );
        HEAPU32 = new Uint32Array( buffer );
        HEAPF32 = new Float32Array( buffer );
        HEAPF64 = new Float64Array( buffer );
    }

    return {
        imports: {
            env: {
                "__extjs_dc2fd915bd92f9e9c6a3bd15174f1414eee3dbaf": function() {
                console.error( 'Encountered a panic!' );
            },
            "__extjs_74d5764ddc102a8d3b6252116087a68f2db0c9d4": function($0) {
                Module.STDWEB_PRIVATE.from_js($0, (function(){return window ;})());
            },
            "__extjs_68d21cddc428c8f40dc802238f92a69b9ba9f92b": function($0) {
                Module.STDWEB_PRIVATE.from_js($0, (function(){var e = document.createElement ("style"); e.type = "text/css" ; document.head.appendChild (e); return e.sheet ;})());
            },
            "__extjs_fc52a58ca59f907dd0ac5c3478b1248029ae9b71": function($0) {
                $0 = Module.STDWEB_PRIVATE.to_js($0);($0). drop ();
            },
            "__extjs_dbbc1d7e712682edbf5208680fd4b7890164f467": function($0, $1) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);Module.STDWEB_PRIVATE.from_js($0, (function(){return ($1). checked ;})());
            },
            "__extjs_7c5535365a3df6a4cc1f59c4a957bfce1dbfb8ee": function($0, $1, $2, $3) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);$2 = Module.STDWEB_PRIVATE.to_js($2);$3 = Module.STDWEB_PRIVATE.to_js($3);Module.STDWEB_PRIVATE.from_js($0, (function(){var listener = ($1); ($2). addEventListener (($3), listener); return listener ;})());
            },
            "__extjs_35b945cda841e63bb9b708539a50cd97b61ac87a": function($0) {
                Module.STDWEB_PRIVATE.from_js($0, (function(){return document.body ;})());
            },
            "__extjs_9230d6a62b16efd096288aa5d2a21e896e597baf": function($0) {
                $0 = Module.STDWEB_PRIVATE.to_js($0);chrome.runtime.sendMessage (null , {type : "records:get-all"}, null , function (value){($0)(value);});
            },
            "__extjs_7a8d264960219d56eb855d4805d38340117514c6": function($0, $1, $2) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);$2 = Module.STDWEB_PRIVATE.to_js($2);Module.STDWEB_PRIVATE.from_js($0, (function(){return ($1). style.getPropertyValue (($2));})());
            },
            "__extjs_c41297f1f679af47d6390b4b617d1a8375706933": function($0) {
                $0 = Module.STDWEB_PRIVATE.to_js($0);console.error (($0));
            },
            "__extjs_ff5103e6cc179d13b4c7a785bdce2708fd559fc0": function($0) {
                Module.STDWEB_PRIVATE.tmp = Module.STDWEB_PRIVATE.to_js( $0 );
            },
            "__extjs_1c8769c3b326d77ceb673ada3dc887cf1d509509": function($0) {
                Module.STDWEB_PRIVATE.from_js($0, (function(){return document ;})());
            },
            "__extjs_5ecfd7ee5cecc8be26c1e6e3c90ce666901b547c": function($0, $1) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);Module.STDWEB_PRIVATE.from_js($0, (function(){return ($1). error ;})());
            },
            "__extjs_9f22d4ca7bc938409787341b7db181f8dd41e6df": function($0) {
                Module.STDWEB_PRIVATE.increment_refcount( $0 );
            },
            "__extjs_4f184f99dbb48468f75bc10e9fc4b1707e193775": function($0, $1, $2) {
                $0 = Module.STDWEB_PRIVATE.to_js($0);$1 = Module.STDWEB_PRIVATE.to_js($1);$2 = Module.STDWEB_PRIVATE.to_js($2);($0). setAttribute (($1), ($2));
            },
            "__extjs_80d6d56760c65e49b7be8b6b01c1ea861b046bf0": function($0) {
                Module.STDWEB_PRIVATE.decrement_refcount( $0 );
            },
            "__extjs_236eb30d6dbed0ab848705c6e083f10fa5ed98c5": function($0) {
                var o = Module.STDWEB_PRIVATE.acquire_js_reference( $0 );return (o instanceof SVGElement) | 0;
            },
            "__extjs_1b47036256330bc1f1d6cf232021ddf04df850e2": function($0, $1) {
                $0 = Module.STDWEB_PRIVATE.to_js($0);$1 = Module.STDWEB_PRIVATE.to_js($1);($0). style.removeProperty (($1));
            },
            "__extjs_f6dede91bad1901e954e3cca1b91ea50e10ca596": function($0) {
                Module.STDWEB_PRIVATE.from_js($0, (function(){return Date.now ();})());
            },
            "__extjs_05a27a55a494fd5964d28e50fdae41f4515e715f": function($0) {
                var o = Module.STDWEB_PRIVATE.acquire_js_reference( $0 );return (o instanceof MouseEvent && o.type === "mouseleave") | 0;
            },
            "__extjs_bba593abe6a9f7704329a20a500a6340adf813c3": function($0, $1, $2) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);$2 = Module.STDWEB_PRIVATE.to_js($2);Module.STDWEB_PRIVATE.from_js($0, (function(){var date = new Date (($1)); date.setUTCDate (date.getUTCDate ()+ ($2)); return date.getTime ();})());
            },
            "__extjs_96019afc94417ba3716a0fc16a6f6bb0b478a037": function($0) {
                var o = Module.STDWEB_PRIVATE.acquire_js_reference( $0 );return (o instanceof HTMLSelectElement) | 0;
            },
            "__extjs_2ff57da66ea0e6d13328bc60a5a5dbfee840cbf2": function($0, $1, $2) {
                $0 = Module.STDWEB_PRIVATE.to_js($0);$1 = Module.STDWEB_PRIVATE.to_js($1);$2 = Module.STDWEB_PRIVATE.to_js($2);var listener = ($0); ($1). removeEventListener (($2), listener); listener.drop ();
            },
            "__extjs_0e00a15cc549aaefd3d6dd8046cb0f65b2fd527c": function($0) {
                var o = Module.STDWEB_PRIVATE.acquire_js_reference( $0 );return (o instanceof Text) | 0;
            },
            "__extjs_6c74cb77248e217d4f866a58aaaa8d8af16c5e5a": function($0, $1, $2) {
                $0 = Module.STDWEB_PRIVATE.to_js($0);$1 = Module.STDWEB_PRIVATE.to_js($1);$2 = Module.STDWEB_PRIVATE.to_js($2);($0)[($1)]= ($2);
            },
            "__extjs_87b4e4c3d34c9f944cf0b8e8c1d62a587e5798be": function($0, $1, $2) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);$2 = Module.STDWEB_PRIVATE.to_js($2);Module.STDWEB_PRIVATE.from_js($0, (function(){try {let array = new Uint8Array (($1)); self.crypto.getRandomValues (array); HEAPU8.set (array , ($2)); return {success : true};}catch (err){return {success : false , error : err};}})());
            },
            "__extjs_c44823ca4ffecb1fc63ff99c8d1bd31834669f31": function($0, $1) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);Module.STDWEB_PRIVATE.from_js($0, (function(){return ($1). success})());
            },
            "__extjs_dafc7a42f11dfd59d938a8bf392d728f25dc1191": function($0, $1) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);Module.STDWEB_PRIVATE.from_js($0, (function(){return ($1). clientX ;})());
            },
            "__extjs_351b27505bc97d861c3914c20421b6277babb53b": function($0) {
                var o = Module.STDWEB_PRIVATE.acquire_js_reference( $0 );return (o instanceof Node) | 0;
            },
            "__extjs_7f342438c940bfafc63d7b76e806c8cc569cd9d6": function($0, $1, $2) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);$2 = Module.STDWEB_PRIVATE.to_js($2);Module.STDWEB_PRIVATE.from_js($0, (function(){var stylesheet = ($1); var length = stylesheet.cssRules.length ; stylesheet.insertRule (($2)+ "{}" , length); return stylesheet.cssRules [length];})());
            },
            "__extjs_763909d7e9e8c824a93a9c9635bb2b1fecb919ee": function($0, $1, $2) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);$2 = Module.STDWEB_PRIVATE.to_js($2);Module.STDWEB_PRIVATE.from_js($0, (function(){var date = new Date (($1)); date.setUTCDate (date.getUTCDate ()- ($2)); return date.getTime ();})());
            },
            "__extjs_ec28a01a0e33da46726388bff28c564a3ae5afa3": function($0) {
                var o = Module.STDWEB_PRIVATE.acquire_js_reference( $0 );return (o instanceof Event && o.type === "change") | 0;
            },
            "__extjs_4125cc9e90bd4b15315b9dfd73bc6605725d7451": function($0, $1) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);Module.STDWEB_PRIVATE.from_js($0, (function(){return document.createTextNode (($1));})());
            },
            "__extjs_b3b1b707ff3ea1372d58c7b79fadbf6050b79b64": function($0) {
                var o = Module.STDWEB_PRIVATE.acquire_js_reference( $0 );return (o instanceof EventTarget) | 0;
            },
            "__extjs_a91f76b9ef152612487d901da5cb59638ee773d2": function($0, $1) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);Module.STDWEB_PRIVATE.from_js($0, (function(){return ($1). error})());
            },
            "__extjs_6d9cd7006b4bd6e5370dc5e43d8d41f9aca46e77": function($0, $1) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);Module.STDWEB_PRIVATE.from_js($0, (function(){return ($1). clientWidth ;})());
            },
            "__extjs_083355932727e223f4e97ad2821c57acddf89e7a": function($0) {
                var o = Module.STDWEB_PRIVATE.acquire_js_reference( $0 );return (o instanceof CSSStyleRule) | 0;
            },
            "__extjs_a342681e5c1e3fb0bdeac6e35d67bf944fcd4102": function($0, $1) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);Module.STDWEB_PRIVATE.from_js($0, (function(){return ($1). value ;})());
            },
            "__extjs_db0226ae1bbecd407e9880ee28ddc70fc3322d9c": function($0) {
                $0 = Module.STDWEB_PRIVATE.to_js($0);Module.STDWEB_PRIVATE.unregister_raw_value (($0));
            },
            "__extjs_cd167021a7bb085ca183bf0c9dde5277b9f77aca": function($0) {
                $0 = Module.STDWEB_PRIVATE.to_js($0);($0). innerHTML = "" ;
            },
            "__extjs_352943ae98b2eeb817e36305c3531d61c7e1a52b": function($0) {
                var o = Module.STDWEB_PRIVATE.acquire_js_reference( $0 );return (o instanceof Element) | 0;
            },
            "__extjs_1522b5ea59072f4360f8c07a28b1f8548cb70424": function($0, $1, $2) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);$2 = Module.STDWEB_PRIVATE.to_js($2);Module.STDWEB_PRIVATE.from_js($0, (function(){try {let bytes = require ("crypto"). randomBytes (($1)); HEAPU8.set (new Uint8Array (bytes), ($2)); return {success : true};}catch (err){return {success : false , error : err};}})());
            },
            "__extjs_a7ab5f74041fe5b800b5238f34837560623fe11e": function($0) {
                Module.STDWEB_PRIVATE.from_js($0, (function(){try {if (typeof self ==="object" && typeof self.crypto ==="object" && typeof self.crypto.getRandomValues ==="function"){return {success : true , ty : 1};}if (typeof require ("crypto"). randomBytes ==="function"){return {success : true , ty : 2};}return {success : false , error : new Error ("not supported")};}catch (err){return {success : false , error : err};}})());
            },
            "__extjs_964a9c3ea63f7374eeff4e3e5c6b7525934ec155": function($0, $1, $2, $3) {
                $0 = Module.STDWEB_PRIVATE.to_js($0);$1 = Module.STDWEB_PRIVATE.to_js($1);$2 = Module.STDWEB_PRIVATE.to_js($2);$3 = Module.STDWEB_PRIVATE.to_js($3);($0). style.setProperty (($1), ($2), (($3)? "important" : ""));
            },
            "__extjs_a0b7b9e5ff62e9d75889569ce62d2d2c2ed899e1": function($0) {
                var o = Module.STDWEB_PRIVATE.acquire_js_reference( $0 );return (o instanceof HTMLElement) | 0;
            },
            "__extjs_64c7eea3395470fabfb75ec1090861d1f66c6c16": function($0) {
                $0 = Module.STDWEB_PRIVATE.to_js($0);($0). style.display = "flex" ;
            },
            "__extjs_a3b76c5b7916fd257ee3f362dc672b974e56c476": function($0, $1) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);Module.STDWEB_PRIVATE.from_js($0, (function(){return ($1). success ;})());
            },
            "__extjs_97495987af1720d8a9a923fa4683a7b683e3acd6": function($0, $1) {
                console.error( 'Panic error message:', Module.STDWEB_PRIVATE.to_js_string( $0, $1 ) );
            },
            "__extjs_2bc54aa3905bf9d7edc7884681f00d9baff13f1e": function($0, $1) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);Module.STDWEB_PRIVATE.from_js($0, (function(){return ($1). value})());
            },
            "__extjs_4cc2b2ed53586a2bd32ca2206724307e82bb32ff": function($0, $1) {
                $0 = Module.STDWEB_PRIVATE.to_js($0);$1 = Module.STDWEB_PRIVATE.to_js($1);($0). appendChild (($1));
            },
            "__extjs_a619fcd124d0713b9c1330733bb52e7a93475a4b": function($0, $1) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);Module.STDWEB_PRIVATE.from_js($0, (function(){var self = ($1); if (self.selectedIndex < 0){return null ;}else {return self.selectedIndex ;}})());
            },
            "__extjs_0e54fd9c163fcf648ce0a395fde4500fd167a40b": function($0) {
                var r = Module.STDWEB_PRIVATE.acquire_js_reference( $0 );return (r instanceof DOMException) && (r.name === "InvalidCharacterError");
            },
            "__extjs_2034053b7b6771271a2dad3e5416b045a74488a1": function($0, $1, $2, $3) {
                Module.STDWEB_PRIVATE.acquire_js_reference( $0 ).setTimeout( function() {Module.STDWEB_PRIVATE.dyncall( 'vi', $1, [$2] );}, $3 );
            },
            "__extjs_1458d968803f0ead5043da77c58d02a82094eda9": function($0) {
                $0 = Module.STDWEB_PRIVATE.to_js($0);($0). style.display = "none" ;
            },
            "__extjs_da7526dacc33bb6de7714dde287806f568820e31": function($0) {
                $0 = Module.STDWEB_PRIVATE.to_js($0);console.log (($0));
            },
            "__extjs_8de42e19567dc9a6c1a2ff637265f4d0db6f718a": function($0, $1) {
                $0 = Module.STDWEB_PRIVATE.to_js($0);$1 = Module.STDWEB_PRIVATE.to_js($1);($0). classList.add (($1));
            },
            "__extjs_b99a06f7004f71b3f4e223fbd9f24cf2620b1047": function($0, $1) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);Module.STDWEB_PRIVATE.from_js($0, (function(){return ($1). message ;})());
            },
            "__extjs_216c7045bad0aa79bff1f8b10b0e7a61cd417ecb": function($0) {
                var o = Module.STDWEB_PRIVATE.acquire_js_reference( $0 );return (o instanceof MouseEvent && o.type === "mousemove") | 0;
            },
            "__extjs_9956e6129dd3eaf3b4d0afa5d110953f3e3e7285": function($0, $1) {
                $0 = Module.STDWEB_PRIVATE.to_js($0);$1 = Module.STDWEB_PRIVATE.to_js($1);($0). data = ($1);
            },
            "__extjs_08b01249624ddc3d96fbac0c69755a1223a49c67": function($0, $1, $2) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);$2 = Module.STDWEB_PRIVATE.to_js($2);Module.STDWEB_PRIVATE.from_js($0, (function(){return document.createElementNS (($1), ($2));})());
            },
            "__extjs_ac8ad4183ba55ff52a4271f8c65efeba62875383": function($0, $1) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);Module.STDWEB_PRIVATE.from_js($0, (function(){return ($1). toLocaleString ("en-US" , {style : "currency" , currency : "USD" , minimumFractionDigits : 0});})());
            },
            "__extjs_906f13b1e97c3e6e6996c62d7584c4917315426d": function($0) {
                var o = Module.STDWEB_PRIVATE.acquire_js_reference( $0 );return (o instanceof MouseEvent && o.type === "click") | 0;
            },
            "__extjs_10f5aa3985855124ab83b21d4e9f7297eb496508": function($0) {
                var o = Module.STDWEB_PRIVATE.acquire_js_reference( $0 );return (o instanceof Array) | 0;
            },
            "__extjs_6665489d4099b58ea29d83630ab4c8be80d7387e": function($0, $1) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);Module.STDWEB_PRIVATE.from_js($0, (function(){return ($1). body ;})());
            },
            "__extjs_573f78ddb74010fede405487d72f98e9ee4ed97f": function($0, $1) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);Module.STDWEB_PRIVATE.from_js($0, (function(){return ($1). ty})());
            },
            "__extjs_b00332decb7a1ee6b8ac41a46cf9fd095a78679e": function($0, $1) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);Module.STDWEB_PRIVATE.from_js($0, (function(){var callback = ($1); var dropped = false ; function wrapper (){if (! dropped){callback ();}}var nextTick ; if (typeof MutationObserver ==="function"){var node = document.createTextNode ("0"); var state = false ; new MutationObserver (wrapper). observe (node , {characterData : true}); nextTick = function (){state = ! state ; node.data = (state ? "1" : "0");};}else {var promise = Promise.resolve (null); nextTick = function (){promise.then (wrapper);};}nextTick.drop = function (){dropped = true ; callback.drop ();}; return nextTick ;})());
            },
            "__extjs_49338674d01c4f5280cdde19baf85b3068471164": function($0, $1) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);Module.STDWEB_PRIVATE.from_js($0, (function(){return ($1). target ;})());
            },
            "__extjs_a94847da281e0efb983dce68b7315dc4a7731c72": function($0) {
                $0 = Module.STDWEB_PRIVATE.to_js($0);($0)();
            },
            "__extjs_33c9ffa61a585b5eccd2ff272475014b5c29e82e": function($0) {
                var o = Module.STDWEB_PRIVATE.acquire_js_reference( $0 );return (o instanceof MouseEvent && o.type === "mouseenter") | 0;
            },
            "__extjs_1b314661d7f229e657cf1110597325b26a0c54f2": function($0) {
                Module.STDWEB_PRIVATE.from_js($0, (function(){return document.createTextNode ("");})());
            },
            "__extjs_74b1b0b6327c6d2bad2259c05a1887a099ec3014": function($0, $1) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);Module.STDWEB_PRIVATE.from_js($0, (function(){return ($1). toLocaleString ("en-US" , {style : "decimal" , maximumFractionDigits : 2});})());
            },
            "__extjs_2e6bf47ec7f31b4cb119a8d2793bacfcfe38b112": function($0) {
                var o = Module.STDWEB_PRIVATE.acquire_js_reference( $0 );return (o instanceof Error) | 0;
            },
            "__extjs_8c2840cb312e587ab1e9209521811fabbf4b9634": function($0) {
                $0 = Module.STDWEB_PRIVATE.to_js($0);var node = ($0); node.textContent = "LOADING" ; node.style.cursor = "default" ; node.style.position = "fixed" ; node.style.left = "0px" ; node.style.top = "0px" ; node.style.width = "100%" ; node.style.height = "100%" ; node.style.zIndex = "2147483647" ; node.style.backgroundColor = "hsla(0, 0%, 0%, 0.50)" ; node.style.color = "white" ; node.style.fontWeight = "bold" ; node.style.fontSize = "30px" ; node.style.letterSpacing = "15px" ; node.style.textShadow = "2px 2px 10px black, 0px 0px 5px black" ; node.style.display = "flex" ; node.style.flexDirection = "row" ; node.style.alignItems = "center" ; node.style.justifyContent = "center" ;
            },
            "__extjs_72fc447820458c720c68d0d8e078ede631edd723": function($0, $1, $2) {
                console.error( 'Panic location:', Module.STDWEB_PRIVATE.to_js_string( $0, $1 ) + ':' + $2 );
            },
            "__extjs_8c32019649bb581b1b742eeedfc410e2bedd56a6": function($0, $1) {
                var array = Module.STDWEB_PRIVATE.acquire_js_reference( $0 );Module.STDWEB_PRIVATE.serialize_array( $1, array );
            },
            "__extjs_f167788c39e80562a6972017cda9ecd6bb91dba7": function($0, $1, $2) {
                $1 = Module.STDWEB_PRIVATE.to_js($1);$2 = Module.STDWEB_PRIVATE.to_js($2);Module.STDWEB_PRIVATE.from_js($0, (function(){try {return {value : function (){return ($1). createElement (($2));}(), success : true};}catch (error){return {error : error , success : false};}})());
            },
                "__web_on_grow": __web_on_grow
            }
        },
        initialize: function( instance ) {
            Object.defineProperty( Module, 'instance', { value: instance } );
            Object.defineProperty( Module, 'web_malloc', { value: Module.instance.exports.__web_malloc } );
            Object.defineProperty( Module, 'web_free', { value: Module.instance.exports.__web_free } );
            Object.defineProperty( Module, 'web_table', { value: Module.instance.exports.__web_table } );

            
            __web_on_grow();
            Module.instance.exports.main();

            return Module.exports;
        }
    };
}
 ));
}));
