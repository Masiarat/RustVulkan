// 1. Definicja wierzchołka - musi pasować do egui::epaint::Vertex!
// egui::epaint::Vertex: Pos2 (2×f32) + TexCoord (2×f32) + Color32 ([u8; 4])
struct Vertex {
    float2 pos;
    float2 uv;
    uint color_packed; // sRGB [u8; 4] color encoded as RGBA
};

// 2. Struktury wyjścia
struct VertexOutput {
    [[vk::location(0)]] float4 color : COLOR0;
    [[vk::location(1)]] float2 uv : TEXCOORD0;
    float4 pos : SV_Position;
};

// 3. Push Constants z wskaźnikiem na bufor Vulkanowy (Buffer Device Address)
struct PushConstants {
    float2 screen_size;
    Vertex* vertex_buffer; // Slang skompiluje to jako wskaźnik PhysicalStorageBuffer (64-bit ulong)
};

[[vk::push_constant]]
PushConstants pc;

// 4. Sampler
[[vk::binding(0, 0)]]
Sampler2D s_texture;

// ==========================================
// VERTEX SHADER
// ==========================================
[shader("vertex")]
VertexOutput vertex(uint vertexId : SV_VertexID) {
    VertexOutput output;

    // Pobranie danych wierzchołka bezpośrednio ze wskaźnika
    Vertex v = pc.vertex_buffer[vertexId];

    // Przeliczenie pikseli na przestrzeń znormalizowaną Vulkana (-1.0 do 1.0)
    output.pos = float4(
        2.0 * v.pos.x / pc.screen_size.x - 1.0,
        2.0 * v.pos.y / pc.screen_size.y - 1.0,
        0.0,
        1.0
    );

    // Dekodowanie sRGB koloru z 4 bajtów (RGBA)
    // egui Color32 to [u8; 4] w formacie RGBA
    float r = float((v.color_packed >> 0) & 0xFF) / 255.0;
    float g = float((v.color_packed >> 8) & 0xFF) / 255.0;
    float b = float((v.color_packed >> 16) & 0xFF) / 255.0;
    float a = float((v.color_packed >> 24) & 0xFF) / 255.0;

    output.color = float4(r, g, b, a);
    output.uv = v.uv;

    return output;
}

// ==========================================
// FRAGMENT SHADER
// ==========================================
[shader("fragment")]
float4 fragment(VertexOutput input) : SV_Target {
    // Sample the texture
    float4 tex_color = s_texture.Sample(input.uv);

    // For egui's premultiplied alpha:
    // - input.color is already in sRGB (from vertex)
    // - We multiply texture by vertex color
    // - The color must be premultiplied for correct blending
    float4 result = input.color * tex_color;

    // Ensure alpha is properly set
    result.a = input.color.a * tex_color.a;

    return result;
}