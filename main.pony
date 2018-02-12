use "net/http"
use "files"
use "crypto"
use "process"
use "json"
use "collections"

actor Main
  new create(env: Env) =>
    let auth = try
      env.root as AmbientAuth
    else
      env.out.print("failed to get root capability")
      return
    end

    let base = try
      //FilePath(auth, "/tmp")?
      FilePath(auth, "")?
    else
      env.out.print("failed to get capability of /tmp")
      return
    end

    let server =
      HTTPServer(
        auth,
        object iso is ServerNotify end,
        {(session: HTTPSession tag) => Router(auth, env.out, session, base)}
        where host = "localhost", service = "8080")

class Router is HTTPHandler
  let _auth: AmbientAuth
  let _out: StdStream
  let _session: HTTPSession
  var _buffer: Array[U8] iso
  var _payload: (Payload val | None)
  var _compiling: Bool

  let _base: FilePath

  new create(auth: AmbientAuth, out: StdStream, session: HTTPSession, base: FilePath) =>
    _auth = auth
    _out = out
    _session = session
    _payload = None
    _buffer = recover iso Array[U8] end
    _compiling = false

    _base = base

  fun ref apply(payload: Payload val) =>
    if payload.method == "GET" then
      try
        var ok = true
        if payload.url.path.find("/files/")? == 0 then
          let filename = payload.url.path.cut(0, 7)
          _out.print(recover val filename.clone() end)
          try
            let path = _base.join(consume filename)?
            let response = payload.response()
            match OpenFile(path)
            | let file: File =>
              while true do
                let buf = file.read(2048)
                _out.print("bufread: " + buf.size().string())
                if buf.size() == 0 then
                  break
                end
                response.send_chunk(consume buf)
              end
              _out.print("responding")
              payload.respond(consume response)
            else
              _out.print("cannot open the given file")
              ok = false
            end
          else
            _out.print("cannot open the given file")
            ok = false
          end

          if ok then
            return
          end
        end
      end
    elseif (payload.method == "POST") and (payload.url.path == "/compile") then
      _out.print("posting")
      _payload = payload
      _compiling = true
      return
    end

    let response = payload.response(StatusNotFound)
    payload.respond(consume response)

  fun ref chunk(data: (String val | Array[U8] val)) =>
    if not _compiling then
      _out.print("not compiling")
      return
    end

    _out.print("chunk")
    _out.print(data)
    let d = match data
      | let s: String val => s.values()
      | let a: Array[U8] val => a.values()
    end

    for c in d do
        _buffer.push(c)
    end

  fun ref finished() =>
    if not _compiling then
      _out.print("not compiling")
    end

    _out.print("finish")
    let b' = (_buffer = recover iso Array[U8] end)
    let b = recover val consume b' end

    let len = _buffer.size()
    let s = recover iso String(len) end
    for c in b.values() do
      s.push(c)
    end
    _out.print(consume s)

    let sha256 = SHA256(b)
    let id = ToHexString(sha256)

    try
      let input = FilePath(_auth, id + ".saty")?
      let output = FilePath(_base, id + ".pdf")?
      let file =
        match CreateFile(input)
        | let f: File => f
        else
          return
        end

      file.write(b)

      let runner = FilePath(_auth, "run.sh")?
      let monitor = ProcessMonitor(
        _auth,
        _auth,
        object iso is ProcessNotify
          let command_output: String iso = recover String end

          fun ref created(process: ProcessMonitor ref) =>
            _out.print("created")
          fun ref failed(process: ProcessMonitor ref, err: ProcessError) =>
            _out.print("error")
          fun ref stdout(process: ProcessMonitor ref, data: Array[U8] iso) =>
            let d: Array[U8] val = consume data
            _out.print("STDOUT:")
            _out.print(d)
            command_output.append(d)
          fun ref stderr(process: ProcessMonitor ref, data: Array[U8] iso) =>
            let d: Array[U8] val = consume data
            _out.print("STDERR:")
            _out.print(d)
            command_output.append(d)
          fun ref dispose(process: ProcessMonitor ref, exit_code: I32) =>
            _out.print("dispose")
            let map = Map[String, JsonType]
            map("output") = command_output.clone()
            map("status") = exit_code.i64()
            map("name") = id + ".pdf"
            let json = JsonObject.from_map(consume map)
            
            match _payload
            | let p: Payload val =>
              let response = p.response()
              response.add_chunk(json.string())
              p.respond(consume response)
            else
              _out.print("payload not set")
              return
            end
        end,
        runner,
        ["run.sh"; input.path; output.path],
        [])
      monitor.done_writing()
    end

