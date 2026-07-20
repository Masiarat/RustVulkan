// Slang uses HLSL-style resource types.
// The [[vk::binding(binding, set)]] attribute maps directly to your Vulkan layout.
[[vk::binding(0, 0)]]
RWTexture2D<float4> image;

[shader("compute")]
[numthreads(16, 16, 1)]
void main(
    uint3 dispatchThreadID : SV_DispatchThreadID, // Replaces gl_GlobalInvocationID
    uint3 groupThreadID    : SV_GroupThreadID     // Replaces gl_LocalInvocationID
)
{
    int2 texelCoord = int2(dispatchThreadID.xy);

    // Query texture dimensions using GetDimensions
    uint width, height;
    image.GetDimensions(width, height);
    int2 size = int2(width, height);

    if (texelCoord.x < size.x && texelCoord.y < size.y)
    {
        float4 color = float4(0.0, 0.0, 0.0, 1.0);

        // Skips the top and left borders of each local workgroup
        if (groupThreadID.x != 0 && groupThreadID.y != 0)
        {
            color.x = float(texelCoord.x) / float(size.x);
            color.y = float(texelCoord.y) / float(size.y);
        }

        // Slang uses simple array subscript syntax instead of imageStore()
        image[texelCoord] = color;
    }
}