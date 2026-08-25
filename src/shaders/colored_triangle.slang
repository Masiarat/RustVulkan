struct Vertex{
    float3 position;
    float uv_x;
    float3 normal;
    float uv_y;
    float4 color;
}

struct VertexOutput {
    [[vk::location(0)]] float4 color : COLOR0;
    [[vk::location(1)]] float2 uv : TEXCOORD0;
    float4 pos : SV_Position;
};

struct PushConstants {
    float4x4 world;;
    Vertex* vertex_buffer; // Slang skompiluje to jako wskaźnik PhysicalStorageBuffer (64-bit ulong)
};

[[vk::push_constant]]
PushConstants pc;

[shader("vertex")]
VertexOutput vertex(uint vertexId : SV_VertexID) {
    VertexOutput output;

    Vertex v = pc.vertex_buffer[vertexId];

    output.pos = float4(v.position, 1.0);
    output.uv = float2(v.uv_x, v.uv_y);
    output.color = v.color;

    return output;
}

// ==========================================
// FRAGMENT SHADER
// ==========================================
[shader("fragment")]
float4 fragment(VertexOutput input) : SV_Target {

    return input.color;
}