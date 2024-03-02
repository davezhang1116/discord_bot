use pyo3::prelude::*;
use pyo3::types::PyTuple;
use crate::xml::reader::get_data;

// NOTE
// python code adapted from https://github.com/INCT-DD/dogecoin-OP_RETURN
// this adaption raises the standard OP_RETURN limit from 80 bytes to 256 bytes using OP_PUSHDATA2 opcodes 
// details about OP_PUSHDATA2: https://wiki.bitcoinsv.io/index.php/Pushdata_Opcodes
// it also enables users to attach multiple OP_RETURNs on a single transaction to maximize user input data
// idea inspired from https://opreturn.net/, a website that allows users to store their thoughts on blockchain


pub fn send(
    message: String,
    address: Option<String>,
    amount: Option<String>,
    testnet: Option<String>
) -> Result<String, ()> {
    let arg1 = &address.unwrap_or(String::from("nq98cmMzxSAxjGH1wLMCYMjTkmEkaV4gP1"));
    let arg2 = &amount.unwrap_or(String::from("0.001"));
    let arg3 = &message;
    let arg4 = &testnet.unwrap_or(String::from("1"));

    let result: Result<Py<PyAny>, PyErr> = Python::with_gil(|py: Python<'_>| {
        let data = get_data();
        let config = format!(
"OP_RETURN_BITCOIN_IP = '{0}' 
OP_RETURN_BITCOIN_PORT = '{1}' 
OP_RETURN_BITCOIN_USER = '{2}'  
OP_RETURN_BITCOIN_PASSWORD = '{3}'  
OP_RETURN_BTC_FEE = 1 
OP_RETURN_BTC_DUST = 0.00001 
OP_RETURN_MAX_BYTES = 30000
OP_RETURN_MAX_BLOCKS = 10  
OP_RETURN_NET_TIMEOUT = 10  
SAT = 100000000", data.url, data.port, data.username, data.password);
        let code = r#"from string import hexdigits
import codecs
import binascii
import hashlib
import json
import os.path
import random
import re
import struct
import time

basestring = str




#def OP_RETURN_send(send_address, send_amount, message, testnet=False):
def OP_RETURN_send(*args):
    
    send_address, send_amount, message, testnet = args[0], float(args[1]), args[2], bool(int(args[3]))

    if not OP_RETURN_bitcoin_check(testnet):
        return {'error': 'Please check Bitcoin Core is running and OP_RETURN_BITCOIN_* constants are set correctly'}

    result = OP_RETURN_bitcoin_cmd('validateaddress', testnet, send_address)
    if not ('isvalid' in result and result['isvalid']):
        return {'error': 'Send address could not be validated: ' + send_address}

    output_amount = send_amount + OP_RETURN_BTC_FEE

    inputs_spend = OP_RETURN_select_inputs(output_amount, testnet)

    if len(message) > OP_RETURN_MAX_BYTES:
        return {'error': 'input too large'}

    if 'error' in inputs_spend:
        return {'error': inputs_spend['error']}

    change_amount = inputs_spend['total'] - output_amount


    change_address = OP_RETURN_bitcoin_cmd('getrawchangeaddress', testnet)

    outputs = {send_address: send_amount}

    if change_amount >= OP_RETURN_BTC_DUST:
        outputs[change_address] = change_amount

    raw_txn = OP_RETURN_create_txn(inputs_spend['inputs'], outputs, message, len(outputs), testnet)

    return OP_RETURN_sign_send_txn(raw_txn, testnet)


def OP_RETURN_check(transaction_hash, testnet=False):
    if not OP_RETURN_bitcoin_check(testnet):
        return {'error': 'Please check Bitcoin Core is running and OP_RETURN_BITCOIN_* constants are set correctly'}

    try:
        raw_transaction = OP_RETURN_bitcoin_cmd('getrawtransaction', testnet, transaction_hash)
    except Exception as e:
        return {'error': f"Couldn't find the specified transaction. Cause: {e}"}
    try:
        transaction_data = OP_RETURN_bitcoin_cmd('decoderawtransaction', testnet, raw_transaction)
    except Exception as e:
        return {'error': f"Couldn't decode the specified transaction. Cause: {e}"}

    results = dict()

    for output in transaction_data['vout']:
        if 'OP_RETURN' in output['scriptPubKey']['asm']:
            op_return_data = output['scriptPubKey']['asm'].split(' ')[1]
            results[op_return_data] = codecs.decode(op_return_data, 'hex').decode('utf-8')

    if not bool(results):
        return {'error': 'No OP_RETURN data found'}
    else:
        return results


def OP_RETURN_select_inputs(total_amount, testnet):
    # List and sort unspent inputs by priority

    unspent_inputs = OP_RETURN_bitcoin_cmd('listunspent', testnet, 0)
    if not isinstance(unspent_inputs, list):
        return {'error': 'Could not retrieve list of unspent inputs'}

    unspent_inputs.sort(key=lambda unspent_input: unspent_input['amount'] * unspent_input['confirmations'],
                        reverse=True)

    # Identify which inputs should be spent

    inputs_spend = []
    input_amount = 0

    for unspent_input in unspent_inputs:
        inputs_spend.append(unspent_input)

        input_amount += unspent_input['amount']
        if input_amount >= total_amount:
            break  # stop when we have enough

    if input_amount < total_amount:
        return {'error': 'Not enough funds are available to cover the amount and fee'}

    # Return the successful result

    return {
        'inputs': inputs_spend,
        'total': input_amount,
    }


def OP_RETURN_create_txn(inputs, outputs, message, metadata_pos, testnet):

    raw_txn = OP_RETURN_bitcoin_cmd('createrawtransaction', testnet, inputs, outputs)

    txn_unpacked = OP_RETURN_unpack_txn(OP_RETURN_hex_to_bin(raw_txn))

    x=100
    res=[message[y-x:y] for y in range(x, len(message)+x,x)]
    pos = 2

    for metadata in res:

        metadata = metadata.encode("utf-8")

        metadata_len = len(metadata)

        if metadata_len <= 75:
            payload = bytearray((metadata_len,)) + metadata  # length byte + data (https://en.bitcoin.it/wiki/Script)
        elif metadata_len <= 256:
            payload = b"\x4c" + bytearray((metadata_len,)) + metadata  # OP_PUSHDATA1 format
        else:
            payload = b"\x4d" + bytearray((metadata_len % 256,)) + bytearray(
                (int(metadata_len / 256),)) + metadata  # OP_PUSHDATA2 format

        metadata_pos = min(max(0, metadata_pos), len(txn_unpacked['vout']))  # constrain to valid values
        txn_unpacked['vout'][pos:pos] = [{
            'value': 0,
            'scriptPubKey': '6a' + OP_RETURN_bin_to_hex(payload)  # here's the OP_RETURN
        }]
        pos += 1
    return OP_RETURN_bin_to_hex(OP_RETURN_pack_txn(txn_unpacked))

def OP_RETURN_sign_send_txn(raw_txn, testnet):
    signed_txn = OP_RETURN_bitcoin_cmd('signrawtransaction', testnet, raw_txn)
    if not ('complete' in signed_txn and signed_txn['complete']):
        return {'error': 'Could not sign the transaction'}
    send_txid = OP_RETURN_bitcoin_cmd('sendrawtransaction', testnet, signed_txn['hex'])
    if not (isinstance(send_txid, basestring) and len(send_txid) == 64):
        return {'error': 'Could not send the transaction'}
    return str(send_txid)


def OP_RETURN_list_mempool_txns(testnet):
    return OP_RETURN_bitcoin_cmd('getrawmempool', testnet)


def OP_RETURN_get_mempool_txn(txid, testnet):
    raw_txn = OP_RETURN_bitcoin_cmd('getrawtransaction', testnet, txid)
    return OP_RETURN_unpack_txn(OP_RETURN_hex_to_bin(raw_txn))


def OP_RETURN_get_mempool_txns(testnet):
    txids = OP_RETURN_list_mempool_txns(testnet)

    txns = {}
    for txid in txids:
        txns[txid] = OP_RETURN_get_mempool_txn(txid, testnet)

    return txns


def OP_RETURN_get_raw_block(height, testnet):
    block_hash = OP_RETURN_bitcoin_cmd('getblockhash', testnet, height)

    if not (isinstance(block_hash, basestring) and len(block_hash) == 64):
        return {'error': 'Block at height ' + str(height) + ' not found'}

    block_data = OP_RETURN_hex_to_bin(OP_RETURN_bitcoin_cmd('getblock', testnet, block_hash, 'false'))

    return {
        'block': block_data
    }


def OP_RETURN_get_block_txns(height, testnet):
    raw_block = OP_RETURN_get_raw_block(height, testnet)
    if 'error' in raw_block:
        return {'error': raw_block['error']}

    block = OP_RETURN_unpack_block(raw_block['block'])

    return block['txs']


# Talking to bitcoin-cli

def OP_RETURN_bitcoin_check(testnet):
    info = OP_RETURN_bitcoin_cmd('getinfo', testnet)

    return isinstance(info, dict) and 'balance' in info


def OP_RETURN_bitcoin_cmd(command, testnet, *args):  # more params are read from here
        request = {
            'id': str(time.time()) + '-' + str(random.randint(100000, 999999)),
            'method': command,
            'params': args,
        }

        port = OP_RETURN_BITCOIN_PORT
        user = OP_RETURN_BITCOIN_USER
        password = OP_RETURN_BITCOIN_PASSWORD

        if not (len(port) and len(user) and len(password)):
            conf_lines = open(os.path.expanduser('~') + '/.dogecoin/dogecoin.conf').readlines()

            for conf_line in conf_lines:
                parts = conf_line.strip().split('=', 1)  # up to 2 parts

                if (parts[0] == 'rpcport') and not len(port):
                    port = int(parts[1])
                if (parts[0] == 'rpcuser') and not len(user):
                    user = parts[1]
                if (parts[0] == 'rpcpassword') and not len(password):
                    password = parts[1]

        if not len(port):
            port = 18332 if testnet else 8332

        if not (len(user) and len(password)):
            return None  # no point trying in this case

        url = 'http://' + OP_RETURN_BITCOIN_IP + ':' + str(port) + '/'

        try:
            from urllib2 import HTTPPasswordMgrWithDefaultRealm, HTTPBasicAuthHandler, build_opener, install_opener, \
                urlopen
        except ImportError:
            from urllib.request import HTTPPasswordMgrWithDefaultRealm, HTTPBasicAuthHandler, build_opener, \
                install_opener, urlopen

        passman = HTTPPasswordMgrWithDefaultRealm()
        passman.add_password(None, url, user, password)
        auth_handler = HTTPBasicAuthHandler(passman)
        opener = build_opener(auth_handler)
        install_opener(opener)
        raw_result = urlopen(url, json.dumps(request).encode('utf-8'), OP_RETURN_NET_TIMEOUT).read()

        result_array = json.loads(raw_result.decode('utf-8'))
        result = result_array['result']

        return result


# Working with data references

# The format of a data reference is: [estimated block height]-[partial txid] - where:

# [estimated block height] is the block where the first transaction might appear and following
# which all subsequent transactions are expected to appear. In the event of a weird blockchain
# reorg, it is possible the first transaction might appear in a slightly earlier block. When
# embedding data, we set [estimated block height] to 1+(the current block height).

# [partial txid] contains 2 adjacent bytes from the txid, at a specific position in the txid:
# 2*([partial txid] div 65536) gives the offset of the 2 adjacent bytes, between 0 and 28.
# ([partial txid] mod 256) is the byte of the txid at that offset.
# (([partial txid] mod 65536) div 256) is the byte of the txid at that offset plus one.
# Note that the txid is ordered according to user presentation, not raw data in the block.


def OP_RETURN_calc_ref(next_height, txid, avoid_txids):
    txid_binary = OP_RETURN_hex_to_bin(txid)

    for txid_offset in range(15):
        sub_txid = txid_binary[2 * txid_offset:2 * txid_offset + 2]
        clashed = False

        for avoid_txid in avoid_txids:
            avoid_txid_binary = OP_RETURN_hex_to_bin(avoid_txid)

            if (
                    (avoid_txid_binary[2 * txid_offset:2 * txid_offset + 2] == sub_txid) and
                    (txid_binary != avoid_txid_binary)
            ):
                clashed = True
                break

        if not clashed:
            break

    if clashed:  # could not find a good reference
        return None

    tx_ref = ord(txid_binary[2 * txid_offset:1 + 2 * txid_offset]) + 256 * ord(
        txid_binary[1 + 2 * txid_offset:2 + 2 * txid_offset]) + 65536 * txid_offset

    return '%06d-%06d' % (next_height, tx_ref)


def OP_RETURN_get_ref_parts(ref):
    if not re.search('^[0-9]+\-[0-9A-Fa-f]+$', ref):  # also support partial txid for second half
        return None

    parts = ref.split('-')

    if re.search('[A-Fa-f]', parts[1]):
        if len(parts[1]) >= 4:
            txid_binary = OP_RETURN_hex_to_bin(parts[1][0:4])
            parts[1] = ord(txid_binary[0:1]) + 256 * ord(txid_binary[1:2]) + 65536 * 0
        else:
            return None

    parts = list(map(int, parts))

    if parts[1] > 983039:  # 14*65536+65535
        return None

    return parts


def OP_RETURN_get_ref_heights(ref, max_height):
    parts = OP_RETURN_get_ref_parts(ref)
    if not parts:
        return None

    return OP_RETURN_get_try_heights(parts[0], max_height, True)


def OP_RETURN_get_try_heights(est_height, max_height, also_back):
    forward_height = est_height
    back_height = min(forward_height - 1, max_height)

    heights = []
    mempool = False
    try_height = 0

    while True:
        if also_back and ((try_height % 3) == 2):  # step back every 3 tries
            heights.append(back_height)
            back_height -= 1

        else:
            if forward_height > max_height:
                if not mempool:
                    heights.append(0)  # indicates to try mempool
                    mempool = True

                elif not also_back:
                    break  # nothing more to do here

            else:
                heights.append(forward_height)

            forward_height += 1

        if len(heights) >= OP_RETURN_MAX_BLOCKS:
            break

        try_height += 1

    return heights


def OP_RETURN_match_ref_txid(ref, txid):
    parts = OP_RETURN_get_ref_parts(ref)
    if not parts:
        return None

    txid_offset = int(parts[1] / 65536)
    txid_binary = OP_RETURN_hex_to_bin(txid)

    txid_part = txid_binary[2 * txid_offset:2 * txid_offset + 2]
    txid_match = bytearray([parts[1] % 256, int((parts[1] % 65536) / 256)])

    return txid_part == txid_match  # exact binary comparison


# Unpacking and packing bitcoin blocks and transactions	

def OP_RETURN_unpack_block(binary):
    buffer = OP_RETURN_buffer(binary)
    block = {}

    block['version'] = buffer.shift_unpack(4, '<L')
    block['hashPrevBlock'] = OP_RETURN_bin_to_hex(buffer.shift(32)[::-1])
    block['hashMerkleRoot'] = OP_RETURN_bin_to_hex(buffer.shift(32)[::-1])
    block['time'] = buffer.shift_unpack(4, '<L')
    block['bits'] = buffer.shift_unpack(4, '<L')
    block['nonce'] = buffer.shift_unpack(4, '<L')
    block['tx_count'] = buffer.shift_varint()

    block['txs'] = {}

    old_ptr = buffer.used()

    while buffer.remaining():
        transaction = OP_RETURN_unpack_txn_buffer(buffer)
        new_ptr = buffer.used()
        size = new_ptr - old_ptr

        raw_txn_binary = binary[old_ptr:old_ptr + size]
        txid = OP_RETURN_bin_to_hex(hashlib.sha256(hashlib.sha256(raw_txn_binary).digest()).digest()[::-1])

        old_ptr = new_ptr

        transaction['size'] = size
        block['txs'][txid] = transaction

    return block


def OP_RETURN_unpack_txn(binary):
    return OP_RETURN_unpack_txn_buffer(OP_RETURN_buffer(binary))


def OP_RETURN_unpack_txn_buffer(buffer):
    # see: https://en.bitcoin.it/wiki/Transactions

    txn = {
        'vin': [],
        'vout': [],
    }

    txn['version'] = buffer.shift_unpack(4, '<L')  # small-endian 32-bits

    inputs = buffer.shift_varint()
    if inputs > 100000:  # sanity check
        return None

    for _ in range(inputs):
        input = {}

        input['txid'] = OP_RETURN_bin_to_hex(buffer.shift(32)[::-1])
        input['vout'] = buffer.shift_unpack(4, '<L')
        length = buffer.shift_varint()
        input['scriptSig'] = OP_RETURN_bin_to_hex(buffer.shift(length))
        input['sequence'] = buffer.shift_unpack(4, '<L')

        txn['vin'].append(input)

    outputs = buffer.shift_varint()
    if outputs > 100000:  # sanity check
        return None

    for _ in range(outputs):
        output = {}

        output['value'] = float(buffer.shift_uint64()) / 100000000
        length = buffer.shift_varint()
        output['scriptPubKey'] = OP_RETURN_bin_to_hex(buffer.shift(length))

        txn['vout'].append(output)

    txn['locktime'] = buffer.shift_unpack(4, '<L')

    return txn


def OP_RETURN_find_spent_txid(txns, spent_txid, spent_vout):
    for txid, txn_unpacked in txns.items():
        for input in txn_unpacked['vin']:
            if (input['txid'] == spent_txid) and (input['vout'] == spent_vout):
                return txid

    return None


def OP_RETURN_find_txn_data(txn_unpacked):
    for index, output in enumerate(txn_unpacked['vout']):
        op_return = OP_RETURN_get_script_data(OP_RETURN_hex_to_bin(output['scriptPubKey']))

        if op_return:
            return {
                'index': index,
                'op_return': op_return,
            }

    return None


def OP_RETURN_get_script_data(scriptPubKeyBinary):
    op_return = None

    if scriptPubKeyBinary[0:1] == b'\x6a':
        first_ord = ord(scriptPubKeyBinary[1:2])

        if first_ord <= 75:
            op_return = scriptPubKeyBinary[2:2 + first_ord]
        elif first_ord == 0x4c:
            op_return = scriptPubKeyBinary[3:3 + ord(scriptPubKeyBinary[2:3])]
        elif first_ord == 0x4d:
            op_return = scriptPubKeyBinary[4:4 + ord(scriptPubKeyBinary[2:3]) + 256 * ord(scriptPubKeyBinary[3:4])]

    return op_return


def OP_RETURN_pack_txn(txn):
    binary = b''

    binary += struct.pack('<L', txn['version'])

    binary += OP_RETURN_pack_varint(len(txn['vin']))

    for input in txn['vin']:
        binary += OP_RETURN_hex_to_bin(input['txid'])[::-1]
        binary += struct.pack('<L', input['vout'])
        binary += OP_RETURN_pack_varint(int(len(input['scriptSig']) / 2))  # divide by 2 because it is currently in hex
        binary += OP_RETURN_hex_to_bin(input['scriptSig'])
        binary += struct.pack('<L', input['sequence'])

    binary += OP_RETURN_pack_varint(len(txn['vout']))

    for output in txn['vout']:
        binary += OP_RETURN_pack_uint64(int(round(output['value'] * 100000000)))
        binary += OP_RETURN_pack_varint(
            int(len(output['scriptPubKey']) / 2))  # divide by 2 because it is currently in hex
        binary += OP_RETURN_hex_to_bin(output['scriptPubKey'])

    binary += struct.pack('<L', txn['locktime'])

    return binary


def OP_RETURN_pack_varint(integer):
    if integer > 0xFFFFFFFF:
        packed = "\xFF" + OP_RETURN_pack_uint64(integer)
    elif integer > 0xFFFF:
        packed = "\xFE" + struct.pack('<L', integer)
    elif integer > 0xFC:
        packed = "\xFD".struct.pack('<H', integer)
    else:
        packed = struct.pack('B', integer)

    return packed


def OP_RETURN_pack_uint64(integer):
    upper = int(integer / 4294967296)
    lower = integer - upper * 4294967296

    return struct.pack('<L', lower) + struct.pack('<L', upper)


# Helper class for unpacking bitcoin binary data

class OP_RETURN_buffer():

    def __init__(self, data, ptr=0):
        self.data = data
        self.len = len(data)
        self.ptr = ptr

    def shift(self, chars):
        prefix = self.data[self.ptr:self.ptr + chars]
        self.ptr += chars

        return prefix

    def shift_unpack(self, chars, format):
        unpack = struct.unpack(format, self.shift(chars))

        return unpack[0]

    def shift_varint(self):
        value = self.shift_unpack(1, 'B')

        if value == 0xFF:
            value = self.shift_uint64()
        elif value == 0xFE:
            value = self.shift_unpack(4, '<L')
        elif value == 0xFD:
            value = self.shift_unpack(2, '<H')

        return value

    def shift_uint64(self):
        return self.shift_unpack(4, '<L') + 4294967296 * self.shift_unpack(4, '<L')

    def used(self):
        return min(self.ptr, self.len)

    def remaining(self):
        return max(self.len - self.ptr, 0)


# Converting binary <-> hexadecimal

def OP_RETURN_hex_to_bin(hex):
    try:
        raw = binascii.a2b_hex(hex)
    except Exception as e:
        print(e)
        return None

    return raw


def OP_RETURN_bin_to_hex(string):
    return binascii.b2a_hex(string).decode('utf-8')
"#;


        let fun: Py<PyAny> = PyModule::from_code(
            py,
            format!("{}\n{}", config, code).as_str(),
            "",
            "",
        )?
        .getattr("OP_RETURN_send")?
        .into();

        let args = PyTuple::new(py, &[arg1, arg2, arg3, arg4]);
        let result = fun.call1(py, args)?;

        Ok(result)

    });
    Ok(format!("{}", result.unwrap()))
}