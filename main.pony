use "net/http"
use "files"

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
  let _buffer: Array[U8]
  var _payload: (Payload val | None)

  let _base: FilePath

  new create(auth: AmbientAuth, out: StdStream, session: HTTPSession, base: FilePath) =>
    _auth = auth
    _out = out
    _session = session
    _payload = None
    _buffer = Array[U8]

    _base = base

  fun ref apply(payload: Payload val) =>
    _payload = payload
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
              _payload = None
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
    end

    let response = payload.response(StatusNotFound)
    payload.respond(consume response)
    _payload = None

  fun ref chunk(data: (String val | Array[U8] val)) =>
    let d = match data
      | let s: String val => s.values()
      | let a: Array[U8] val => a.values()
      end
    _buffer.concat(consume d)

  fun ref finish() =>
    let len = _buffer.size()
    let s = recover iso String(len) end
    for c in _buffer.values() do
      s.push(c)
    end
    _out.print(consume s)
