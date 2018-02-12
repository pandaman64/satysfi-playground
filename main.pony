use "net/http"
use "files"
use "crypto"
use "encode/base64"

actor Main
  new create(env: Env) =>
    let auth = try
      env.root as AmbientAuth
    else
      env.out.print("failed to get root capability")
      return
    end

    let base = try
      FilePath(auth, "/tmp")?
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
                if buf.size() == 0 then
                  break
                end
                response.add_chunk(consume buf)
              end
              payload.respond(consume response)
            else
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
      let file =
        match CreateFile(input)
        | let f: File => f
        else
          return
        end

      file.write(b)
    end

