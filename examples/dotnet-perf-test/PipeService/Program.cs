using System;
using System.IO;
using System.IO.Pipes;
using System.Net.Sockets;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

namespace PipeService;

class Program
{
    static async Task Main(string[] args)
    {
        // Get PIPE address from environment variable
        var pipeAddress = Environment.GetEnvironmentVariable("PIPE_ADDRESS");
        if (string.IsNullOrEmpty(pipeAddress))
        {
            Console.Error.WriteLine("ERROR: PIPE_ADDRESS environment variable not set");
            Environment.Exit(1);
            return;
        }

        Console.WriteLine($"[PipeService] Starting pipe service on {pipeAddress}...");

        // Check if running on Unix (Linux/macOS) or Windows
        bool isUnix = pipeAddress.StartsWith("/");
        
        if (isUnix)
        {
            await RunUnixPipeServerAsync(pipeAddress);
        }
        else
        {
            await RunWindowsPipeServerAsync(pipeAddress);
        }
    }

    static async Task RunUnixPipeServerAsync(string socketPath)
    {
        // Delete socket file if it already exists
        if (File.Exists(socketPath))
        {
            File.Delete(socketPath);
        }

        var socket = new Socket(AddressFamily.Unix, SocketType.Stream, ProtocolType.Unspecified);
        var endpoint = new UnixDomainSocketEndPoint(socketPath);
        
        socket.Bind(endpoint);
        socket.Listen(5);

        Console.WriteLine($"[PipeService] Unix socket server listening on {socketPath}");

        while (true)
        {
            try
            {
                var clientSocket = await socket.AcceptAsync();
                _ = Task.Run(async () => await HandleClientAsync(clientSocket));
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"Error accepting connection: {ex.Message}");
            }
        }
    }

    static async Task RunWindowsPipeServerAsync(string pipeName)
    {
        // Extract pipe name from full path (e.g., "\\.\pipe\my_pipe" -> "my_pipe")
        var actualPipeName = pipeName.Replace("\\\\.\\pipe\\", "");
        
        Console.WriteLine($"[PipeService] Windows named pipe server listening on {pipeName}");

        while (true)
        {
            try
            {
                using var pipeServer = new NamedPipeServerStream(
                    actualPipeName,
                    PipeDirection.InOut,
                    NamedPipeServerStream.MaxAllowedServerInstances,
                    PipeTransmissionMode.Byte,
                    PipeOptions.Asynchronous);

                await pipeServer.WaitForConnectionAsync();
                await HandlePipeClientAsync(pipeServer);
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"Error with pipe connection: {ex.Message}");
            }
        }
    }

    static async Task HandleClientAsync(Socket clientSocket)
    {
        try
        {
            using var stream = new NetworkStream(clientSocket, ownsSocket: true);
            using var reader = new StreamReader(stream, Encoding.UTF8);
            using var writer = new StreamWriter(stream, Encoding.UTF8) { AutoFlush = true };

            var requestJson = await reader.ReadToEndAsync();
            
            if (string.IsNullOrWhiteSpace(requestJson))
            {
                return;
            }

            var requestData = JsonSerializer.Deserialize<JsonElement>(requestJson);
            
            var method = requestData.GetProperty("method").GetString();
            var uri = requestData.GetProperty("uri").GetString();
            
            // Create response
            var responseBody = new
            {
                service = "dotnet-pipe-service",
                communication = "pipe",
                timestamp = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
                request = new { method, uri }
            };
            
            var responseBodyJson = JsonSerializer.Serialize(responseBody);
            var responseBodyBase64 = Convert.ToBase64String(Encoding.UTF8.GetBytes(responseBodyJson));
            
            var response = new
            {
                status = 200,
                headers = new { ContentType = "application/json" },
                body = responseBodyBase64
            };
            
            var responseJson = JsonSerializer.Serialize(response);
            await writer.WriteAsync(responseJson);
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error handling client: {ex.Message}");
        }
    }

    static async Task HandlePipeClientAsync(NamedPipeServerStream pipeStream)
    {
        try
        {
            using var reader = new StreamReader(pipeStream, Encoding.UTF8, leaveOpen: true);
            using var writer = new StreamWriter(pipeStream, Encoding.UTF8, leaveOpen: true) { AutoFlush = true };

            var requestJson = await reader.ReadToEndAsync();
            
            if (string.IsNullOrWhiteSpace(requestJson))
            {
                return;
            }

            var requestData = JsonSerializer.Deserialize<JsonElement>(requestJson);
            
            var method = requestData.GetProperty("method").GetString();
            var uri = requestData.GetProperty("uri").GetString();
            
            // Create response
            var responseBody = new
            {
                service = "dotnet-pipe-service",
                communication = "pipe",
                timestamp = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
                request = new { method, uri }
            };
            
            var responseBodyJson = JsonSerializer.Serialize(responseBody);
            var responseBodyBase64 = Convert.ToBase64String(Encoding.UTF8.GetBytes(responseBodyJson));
            
            var response = new
            {
                status = 200,
                headers = new { ContentType = "application/json" },
                body = responseBodyBase64
            };
            
            var responseJson = JsonSerializer.Serialize(response);
            await writer.WriteAsync(responseJson);
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Error handling pipe client: {ex.Message}");
        }
    }
}
