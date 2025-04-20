struct PS_INPUT {
    float4 pos: SV_POSITION;
    float4 col: COLOR0;
    float2 uv: TEXCOORD0;
};

sampler sampler0;
Texture2D texture0;

float3 sRGBToLinear( float3 color ) {
	return pow( color, 2.2 );
}

float4 sRGBToLinear( float4 color ) {
	return float4( sRGBToLinear( color.rgb ), color.a );
}

float4 main(PS_INPUT input): SV_Target {
    float4 out_col = sRGBToLinear(input.col * texture0.Sample(sampler0, input.uv));
    return out_col;
}