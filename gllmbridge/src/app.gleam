import gleam/bit_array
import gleam/bytes_tree
import gleam/erlang/process
import gleam/http/request.{type Request}
import gleam/http/response.{type Response}
import gleam/int
import gleam/io
import gleam/option.{None, Some}
import gleam/otp/actor
import gleam/result
import gleam/string
import gleam/yielder
import mist.{type Connection, type ResponseData}

pub fn main() {
  let not_found =
    response.new(404)
    |> response.set_body(mist.Bytes(bytes_tree.new()))

  let assert Ok(_) =
    fn(req: Request(Connection)) -> Response(ResponseData) {
      case request.path_segments(req) {
        ["echo"] -> echo_body(req)
        _ -> not_found
      }
    }
    |> mist.new
    |> mist.port(8080)
    |> mist.start_http
  process.sleep_forever()
}

fn echo_body(request: Request(Connection)) -> Response(ResponseData) {
  let content_type =
    request
    |> request.get_header("content-type")
    |> result.unwrap("text/plain")
  // Convert body to string using erlang/otp

  // logging the request body
  mist.read_body(request, 1024 * 1024 * 10)
  |> result.map(fn(req) {
    let body_str = case bit_array.to_string(req.body) {
      Ok(str) -> str
      Error(_) ->
        "(non-UTF8 body, "
        <> int.to_string(bit_array.bit_size(req.body))
        <> " bytes)"
    }

    // Print the body
    io.println("Request Body: " <> body_str)

    response.new(200)
    |> response.set_body(mist.Bytes(bytes_tree.from_bit_array(req.body)))
    |> response.set_header("content-type", content_type)
  })
  |> result.lazy_unwrap(fn() {
    response.new(400)
    |> response.set_body(mist.Bytes(bytes_tree.new()))
  })
}
