// 1. Push Constants - wstrzykujemy rozmiar ekranu
struct PushConstants {
    float2 screen_size;
};

[[vk::push_constant]]
PushConstants pc;

// 2. Struktury wejścia/wyjścia
struct VertexInput {
    // Slang automatycznie przypisze location na podstawie kolejności
    // lub można wymusić używając [[vk::location(0)]]
    [[vk::location(0)]] float2 pos : POSITION;
    [[vk::location(1)]] float2 uv : TEXCOORD0;
    [[vk::location(2)]] float4 color : COLOR0;
};

struct VertexOutput {
    [[vk::location(0)]] float4 color : COLOR0;
    [[vk::location(1)]] float2 uv : TEXCOORD0;
    float4 pos : SV_Position;
};

// 3. Nasz koszyk z teksturą i samplerem (COMBINED_IMAGE_SAMPLER)
[[vk::binding(0, 0)]]
Sampler2D s_texture;


// ==========================================
// VERTEX SHADER
// ==========================================
[shader("vertex")]
VertexOutput vertexMain(VertexInput input) {
    VertexOutput output;

    // Przeliczenie pikseli na przestrzeń znormalizowaną Vulkana (-1.0 do 1.0)
    output.pos = float4(
        2.0 * input.pos.x / pc.screen_size.x - 1.0,
        2.0 * input.pos.y / pc.screen_size.y - 1.0,
        0.0,
        1.0
    );

    output.color = input.color;
    output.uv = input.uv;

    return output;
}

// ==========================================
// FRAGMENT SHADER
// ==========================================
[shader("fragment")]
float4 fragmentMain(VertexOutput input) : SV_Target {
    // Próbkowanie tekstury i przemnożenie przez kolor wierzchołka
    // (egui wymaga tzw. premultiplied alpha)
    float4 tex_color = s_texture.Sample(input.uv);
    return input.color * tex_color;
}