using System;
using System.IO;
using System.Net;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace HttpService;

class Program
{
    static async Task Main(string[] args)
    {
        // Get HTTP address from environment variable
        var httpAddress = Environment.GetEnvironmentVariable("HTTP_ADDRESS");
        if (string.IsNullOrEmpty(httpAddress))
        {
            Console.Error.WriteLine("ERROR: HTTP_ADDRESS environment variable not set");
            Environment.Exit(1);
            return;
        }

        Console.WriteLine($"[HttpService] Starting HTTP server on {httpAddress}...");

        var parts = httpAddress.Split(':');
        var host = parts[0];
        var port = int.Parse(parts[1]);

        var builder = WebApplication.CreateBuilder(args);
        
        // Configure Kestrel to listen on the specified address
        builder.WebHost.ConfigureKestrel(serverOptions =>
        {
            serverOptions.Listen(IPAddress.Parse(host), port);
        });

        // Disable logging for cleaner output
        builder.Logging.ClearProviders();

        var app = builder.Build();

        app.MapPost("/", async (HttpContext context) =>
        {
            try
            {
                using var reader = new StreamReader(context.Request.Body);
                var requestJson = await reader.ReadToEndAsync();
                
                var requestData = JsonSerializer.Deserialize<JsonElement>(requestJson);
                
                var method = requestData.GetProperty("method").GetString();
                var uri = requestData.GetProperty("uri").GetString();
                
                // Create response
                var responseBody = new
                {
                    service = "dotnet-http-service",
                    communication = "http",
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
                
                context.Response.ContentType = "application/json";
                await context.Response.WriteAsync(responseJson);
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"Error processing request: {ex.Message}");
                context.Response.StatusCode = 500;
            }
        });

        Console.WriteLine("[HttpService] HTTP server ready");
        
        await app.RunAsync();
    }
}
